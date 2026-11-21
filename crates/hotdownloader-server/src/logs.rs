//! 运行日志的进程级环形缓冲。
//!
//! 网页端的日志页只读取这里的快照，因此换设备、清理浏览器缓存或更换访问端口后记录仍在。
//! 日志属于辅助信息，写盘失败只记录一条警告，不影响任务与接口返回。

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::json;

/// 与服务端保留上限一致：超过后丢弃最旧的记录。
pub const MAX_LOG_ENTRIES: usize = 200;

const LEVELS: [&str; 4] = ["success", "info", "warn", "error"];
const SCOPES: [&str; 4] = ["parse", "download", "account", "system"];
const MAX_TITLE_LEN: usize = 300;
const MAX_DETAIL_LEN: usize = 2000;
const MAX_ID_LEN: usize = 64;
const MAX_PLATFORM_LEN: usize = 32;

/// 一条运行日志。字段与前端 `LogEntry` 一一对应。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: String,
    /// 事件发生时间（毫秒时间戳）。只由服务端生成，多设备的时间线保持一致。
    pub at: i64,
    pub level: String,
    pub scope: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
}

/// 客户端提交的日志字段。
///
/// `id` 允许由客户端提供，服务端按 id 幂等写入：页面先乐观插入再提交，
/// 随后收到的 SSE 回显不会产生重复条目。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogInput {
    #[serde(default)]
    pub id: String,
    pub level: String,
    pub scope: String,
    pub title: String,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
}

struct LogState {
    /// 新的在前，长度不超过 [`MAX_LOG_ENTRIES`]。
    entries: Vec<LogEntry>,
    sequence: u64,
}

pub struct LogBuffer {
    path: PathBuf,
    state: Mutex<LogState>,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

/// 先写临时文件再替换，避免进程中断留下半截 JSON。
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, bytes)?;
    #[cfg(windows)]
    {
        // Windows 上 rename 不能覆盖已存在文件，先复制再删除临时文件。
        if path.exists() {
            std::fs::copy(&temporary, path)?;
            std::fs::remove_file(&temporary)?;
        } else {
            std::fs::rename(&temporary, path)?;
        }
    }
    #[cfg(not(windows))]
    std::fs::rename(&temporary, path)?;
    Ok(())
}

fn trim_to(value: String, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value;
    }
    value.chars().take(limit).collect()
}

/// 归一化可选文本：全空白的细节视为未提供，避免日志行里出现空占位。
fn normalize_optional(value: Option<String>, limit: usize) -> Option<String> {
    value
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .map(|text| trim_to(text, limit))
}

impl LogBuffer {
    /// 读取历史日志。文件缺失或损坏时从空列表开始，日志问题不能阻止服务启动。
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("logs.json");
        let entries = match std::fs::read_to_string(&path) {
            Ok(raw) => match serde_json::from_str::<Vec<LogEntry>>(&raw) {
                Ok(mut entries) => {
                    entries.sort_by(|a, b| b.at.cmp(&a.at));
                    entries.truncate(MAX_LOG_ENTRIES);
                    entries
                }
                Err(error) => {
                    log::warn!("解析 logs.json 失败，本次从空日志开始: {error}");
                    Vec::new()
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => {
                log::warn!("读取 logs.json 失败，本次从空日志开始: {error}");
                Vec::new()
            }
        };
        Self {
            path,
            state: Mutex::new(LogState {
                entries,
                sequence: 0,
            }),
        }
    }

    /// 当前快照（新的在前）。
    pub fn list(&self) -> Vec<LogEntry> {
        self.state.lock().unwrap().entries.clone()
    }

    /// 追加一条客户端日志。id 已存在时直接返回原记录，重复提交不会产生多条。
    pub fn append(&self, input: LogInput) -> Result<LogEntry, String> {
        let level = input.level.trim().to_string();
        if !LEVELS.contains(&level.as_str()) {
            return Err(format!("未知日志级别: {level}"));
        }
        let scope = input.scope.trim().to_string();
        if !SCOPES.contains(&scope.as_str()) {
            return Err(format!("未知日志分类: {scope}"));
        }
        let title = trim_to(input.title.trim().to_string(), MAX_TITLE_LEN);
        if title.is_empty() {
            return Err("日志标题不能为空".into());
        }
        let id = trim_to(input.id.trim().to_string(), MAX_ID_LEN);
        let platform = normalize_optional(input.platform, MAX_PLATFORM_LEN);
        let detail = normalize_optional(input.detail, MAX_DETAIL_LEN);
        let mut state = self.state.lock().unwrap();
        if !id.is_empty() {
            if let Some(existing) = state.entries.iter().find(|entry| entry.id == id) {
                return Ok(existing.clone());
            }
        }
        let entry = LogEntry {
            id: if id.is_empty() { self.next_id(&mut state) } else { id },
            at: now_ms(),
            level,
            scope,
            title,
            detail,
            platform,
        };
        Self::insert(&mut state, entry.clone());
        self.persist(&state);
        Ok(entry)
    }

    /// 服务端自身产生的日志（下载完成、失败等），不需要经过 HTTP。
    pub fn record(
        &self,
        level: &str,
        scope: &str,
        title: String,
        detail: Option<String>,
        platform: Option<String>,
    ) -> LogEntry {
        let mut state = self.state.lock().unwrap();
        let entry = LogEntry {
            id: self.next_id(&mut state),
            at: now_ms(),
            level: level.to_string(),
            scope: scope.to_string(),
            title: trim_to(title, MAX_TITLE_LEN),
            detail: normalize_optional(detail, MAX_DETAIL_LEN),
            platform: normalize_optional(platform, MAX_PLATFORM_LEN),
        };
        Self::insert(&mut state, entry.clone());
        self.persist(&state);
        entry
    }

    /// 清空全部记录，同时删除落盘文件。
    pub fn clear(&self) {
        let mut state = self.state.lock().unwrap();
        state.entries.clear();
        if let Err(error) = std::fs::remove_file(&self.path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                log::warn!("删除 logs.json 失败: {error}");
            }
        }
    }

    fn next_id(&self, state: &mut LogState) -> String {
        state.sequence += 1;
        format!("{}-{:x}", now_ms(), state.sequence)
    }

    fn insert(state: &mut LogState, entry: LogEntry) {
        // 客户端时间可能与服务端有偏差，插入后统一按服务端时间倒序。
        state.entries.insert(0, entry);
        state.entries.sort_by(|a, b| b.at.cmp(&a.at));
        state.entries.truncate(MAX_LOG_ENTRIES);
    }

    fn persist(&self, state: &LogState) {
        let Ok(bytes) = serde_json::to_vec(&state.entries) else {
            log::warn!("序列化运行日志失败，本次不写盘");
            return;
        };
        if let Err(error) = write_atomic(&self.path, &bytes) {
            log::warn!("写入 logs.json 失败: {error}");
        }
    }
}

/// 歌手为空时用占位文案，日志行里不会出现空片段。
pub fn artist_or_unknown(artist: &str) -> &str {
    if artist.trim().is_empty() {
        "未知歌手"
    } else {
        artist
    }
}

/// 把若干片段拼成一行细节，空片段自动跳过，避免出现多余的分隔符。
pub fn detail_line(parts: &[&str]) -> String {
    let joined: Vec<&str> = parts
        .iter()
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect();
    joined.join(" · ")
}

/// 供 HTTP 层广播使用：把日志条目转成 SSE 数据帧内容。
pub fn to_event_data(entry: &LogEntry) -> String {
    json!(entry).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("hotdownloader-logs-{name}-{nonce}"));
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn input(id: &str, level: &str, title: &str) -> LogInput {
        LogInput {
            id: id.to_string(),
            level: level.to_string(),
            scope: "download".to_string(),
            title: title.to_string(),
            detail: None,
            platform: None,
        }
    }

    #[test]
    fn entries_survive_reload_and_keep_newest_first() {
        let directory = temporary_dir("reload");
        let buffer = LogBuffer::load(&directory);
        buffer.append(input("a", "info", "第一条")).unwrap();
        buffer.append(input("b", "error", "第二条")).unwrap();

        let reloaded = LogBuffer::load(&directory);
        let entries = reloaded.list();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].title, "第二条");
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn duplicate_id_is_idempotent() {
        let directory = temporary_dir("duplicate");
        let buffer = LogBuffer::load(&directory);
        let first = buffer.append(input("same", "info", "一次")).unwrap();
        let again = buffer.append(input("same", "info", "一次")).unwrap();
        assert_eq!(first.id, again.id);
        assert_eq!(buffer.list().len(), 1);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn ring_buffer_drops_oldest_entries() {
        let directory = temporary_dir("ring");
        let buffer = LogBuffer::load(&directory);
        for index in 0..MAX_LOG_ENTRIES + 10 {
            buffer
                .append(input(&format!("id-{index}"), "info", &format!("第 {index} 条")))
                .unwrap();
        }
        let entries = buffer.list();
        assert_eq!(entries.len(), MAX_LOG_ENTRIES);
        assert_eq!(entries[0].title, format!("第 {} 条", MAX_LOG_ENTRIES + 9));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rejects_unknown_level_and_empty_title() {
        let directory = temporary_dir("validate");
        let buffer = LogBuffer::load(&directory);
        assert!(buffer.append(input("x", "fatal", "标题")).is_err());
        assert!(buffer.append(input("y", "info", "   ")).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn clear_removes_file_and_memory() {
        let directory = temporary_dir("clear");
        let buffer = LogBuffer::load(&directory);
        buffer.append(input("a", "info", "一条")).unwrap();
        assert!(directory.join("logs.json").exists());
        buffer.clear();
        assert!(buffer.list().is_empty());
        assert!(!directory.join("logs.json").exists());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
