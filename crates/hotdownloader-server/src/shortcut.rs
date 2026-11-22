//! 快捷指令入口（iOS 捷径、快捷脚本等）。
//!
//! `GET /api/shortcut/parse?key=<key>&url=<分享链接>`：用 URL 里的 key 认证，
//! 解析链接后**直接创建下载任务**，不需要人工确认，返回已创建任务的摘要。
//! 供手机捷径「分享 → 运行脚本」这类场景使用，不依赖 Basic 认证。

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use serde_json::{json, Value};

use hotdownloader_core::task::contract::{CreateTaskRequest, CreateTaskResult, DuplicateAction};
use hotdownloader_core::task::rules::TaskRules;
use hotdownloader_core::task::service::TaskService;

use crate::http::{error_response, json_response, HttpBody};
use crate::music;
use crate::runtime::ServerRuntime;

/// 单次最多处理的歌曲数，避免误分享歌单时一次性刷出上百个任务。
const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 100;

/// 处理 `GET /api/shortcut/parse?key=…&url=…[&limit=…]`。
pub async fn handle_parse(
    request: Request<Incoming>,
    runtime: Arc<ServerRuntime>,
) -> Response<HttpBody> {
    if request.method() != Method::GET {
        return error_response(
            StatusCode::METHOD_NOT_ALLOWED,
            "快捷指令入口只支持 GET",
        );
    }
    let query = request.uri().query().unwrap_or_default().to_string();
    let mut key = String::new();
    let mut url = String::new();
    let mut limit: Option<usize> = None;
    for pair in query.split('&') {
        let Some((name, value)) = pair.split_once('=') else {
            continue;
        };
        match name {
            "key" => key = percent_decode(value),
            "url" => url = percent_decode(value),
            "limit" => limit = percent_decode(value).parse().ok(),
            _ => {}
        }
    }
    if !runtime.shortcut_key.verify(&key) {
        return error_response(StatusCode::UNAUTHORIZED, "key 无效");
    }
    if url.trim().is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "缺少 url 参数");
    }

    match enqueue(&runtime, url.trim(), limit).await {
        Ok(payload) => json_response(StatusCode::OK, payload),
        Err(error) => error_response(StatusCode::BAD_REQUEST, error),
    }
}

/// 解析链接并按平台返回的歌曲逐条创建任务。
async fn enqueue(
    runtime: &ServerRuntime,
    input: &str,
    limit: Option<usize>,
) -> Result<Value, String> {
    let separator = runtime.environment.artist_separator();
    // 复用 HTTP 音乐入口的 links/parse，平台识别、登录态与错误文案保持同一套。
    let request: music::MusicRequest = serde_json::from_value(json!({ "input": input }))
        .map_err(|error| format!("参数构造失败: {error}"))?;
    let payload = music::execute(
        "links/parse",
        request,
        &separator,
        runtime.login_store.as_ref(),
        runtime.netease_store.as_ref(),
    )
    .await?;

    let platform = payload["platform"].as_str().unwrap_or_default().to_string();
    let kind = payload["kind"].as_str().unwrap_or_default().to_string();
    // 单曲返回 {song}，歌单/专辑返回 {songs}。
    let songs: Vec<Value> = match payload["data"]["songs"].as_array() {
        Some(list) => list.clone(),
        None => match payload["data"]["song"] {
            Value::Null => Vec::new(),
            ref song => vec![song.clone()],
        },
    };
    if songs.is_empty() {
        return Err("解析结果里没有可下载的歌曲".into());
    }

    let cap = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let settings = serde_json::to_value(runtime.environment.settings_snapshot())
        .map_err(|error| format!("读取设置失败: {error}"))?;
    let desired_quality = settings["defaultQuality"]
        .as_str()
        .unwrap_or("ask")
        .to_string();
    let rules = TaskRules::from_settings(&settings);
    let service = TaskService::new(&runtime.tasks, &runtime.engine, runtime.environment.as_ref());

    let mut created: Vec<Value> = Vec::new();
    let mut failed: Vec<Value> = Vec::new();
    for song in songs.iter().take(cap) {
        let title = song["title"].as_str().unwrap_or("").to_string();
        let quality = pick_quality(song, &desired_quality, &rules);
        let song_input = match serde_json::from_value(song.clone()) {
            Ok(input) => input,
            Err(_) => {
                failed.push(json!({ "title": title, "error": "歌曲数据不完整" }));
                continue;
            }
        };
        let request = CreateTaskRequest {
            stream: None,
            song: song_input,
            desired_quality: quality,
            // 重名自动改名：捷径场景没有交互，不能停在"是否覆盖"上。
            duplicate_action: Some(DuplicateAction::Rename),
        };
        match service.create_download_task(request).await {
            Ok(CreateTaskResult::Created { task }) => created.push(json!({
                "taskId": task.id,
                "title": task.song_title,
                "artist": task.artist,
                "quality": task.quality,
            })),
            Ok(CreateTaskResult::Existing { task }) => created.push(json!({
                "taskId": task.id,
                "title": task.song_title,
                "artist": task.artist,
                "quality": task.quality,
                "existing": true,
            })),
            Ok(CreateTaskResult::NeedsConfirmation { song_title }) => {
                failed.push(json!({ "title": song_title, "error": "存在同名文件" }))
            }
            Ok(CreateTaskResult::Cancelled) => {
                failed.push(json!({ "title": title, "error": "已取消" }))
            }
            Err(error) => failed.push(json!({ "title": title, "error": error })),
        }
    }

    let message = if created.is_empty() {
        format!("没有创建任务：{}", failed_summary(&failed))
    } else if failed.is_empty() {
        format!("已创建 {} 个下载任务", created.len())
    } else {
        format!(
            "已创建 {} 个任务，{} 个失败",
            created.len(),
            failed.len()
        )
    };

    Ok(json!({
        "ok": !created.is_empty(),
        "message": message,
        "platform": platform,
        "kind": kind,
        "total": songs.len(),
        "createdCount": created.len(),
        "failedCount": failed.len(),
        "songs": created,
        "failed": failed,
    }))
}

fn failed_summary(failed: &[Value]) -> String {
    failed
        .first()
        .and_then(|item| item["error"].as_str())
        .unwrap_or("未知原因")
        .to_string()
}

/// 捷径没有音质选择界面，这里按项目既有的品质优先级取最高可用档：
/// 拿不到时会由任务的自动降级继续往下走，比直接选最低档更符合预期。
/// 用户在设置里指定了默认音质时优先遵循。
fn pick_quality(song: &Value, desired: &str, rules: &TaskRules) -> String {
    let available: Vec<&str> = song["qualities"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item["quality"].as_str())
                .collect()
        })
        .unwrap_or_default();
    if available.is_empty() {
        return desired.to_string();
    }
    if desired != "ask" && available.iter().any(|quality| *quality == desired) {
        return desired.to_string();
    }
    for quality in &rules.quality_order {
        if available.iter().any(|item| item == quality) {
            return quality.clone();
        }
    }
    available[0].to_string()
}

/// 查询串解码：`url` 参数里的 `://`、`?`、`&` 会被编码成 `%3A` 等。
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).ok();
                match hex.and_then(|text| u8::from_str_radix(text, 16).ok()) {
                    Some(byte) => {
                        out.push(byte);
                        index += 3;
                    }
                    None => {
                        out.push(bytes[index]);
                        index += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

// ==================== 接入密钥 ====================

/// 快捷指令密钥，持久化在数据目录的 `shortcut.json`。
pub struct ShortcutKey {
    path: PathBuf,
    key: RwLock<String>,
}

impl ShortcutKey {
    pub fn load(data_dir: &Path) -> Result<Self, String> {
        let path = data_dir.join("shortcut.json");
        let key = std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
            .and_then(|value| {
                value
                    .get("key")
                    .and_then(|key| key.as_str())
                    .map(str::to_string)
            })
            .filter(|key| !key.is_empty())
            .unwrap_or_else(crate::mcp::generate_key);
        let instance = Self {
            path,
            key: RwLock::new(key),
        };
        instance.persist()?;
        Ok(instance)
    }

    pub fn value(&self) -> String {
        self.key.read().unwrap().clone()
    }

    pub fn reset(&self) -> Result<String, String> {
        let key = crate::mcp::generate_key();
        *self.key.write().unwrap() = key.clone();
        self.persist()?;
        Ok(key)
    }

    pub fn verify(&self, candidate: &str) -> bool {
        if candidate.is_empty() {
            return false;
        }
        let key = self.key.read().unwrap();
        crate::mcp::constant_time_eq(&key, candidate)
    }

    fn persist(&self) -> Result<(), String> {
        let raw = serde_json::to_string_pretty(&json!({ "key": self.value() }))
            .map_err(|error| error.to_string())?;
        std::fs::write(&self.path, raw).map_err(|error| format!("写入 shortcut.json 失败: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::{percent_decode, pick_quality};
    use serde_json::json;

    #[test]
    fn decodes_encoded_url() {
        assert_eq!(
            percent_decode("https%3A%2F%2Fy.qq.com%2Fn%2Fryqq%2FsongDetail%2F0039MnYb0qxYhV%3Fa%3D1%26b%3D2"),
            "https://y.qq.com/n/ryqq/songDetail/0039MnYb0qxYhV?a=1&b=2"
        );
        assert_eq!(percent_decode("plain"), "plain");
    }

    #[test]
    fn falls_back_to_first_quality() {
        use super::TaskRules;
        let rules = TaskRules::from_settings(&json!({}));
        let song = json!({ "qualities": [{ "quality": "flac" }, { "quality": "128kmp3" }] });
        // 指定音质不可用时，按品质优先级取最高可用档。
        assert_eq!(pick_quality(&song, "320kmp3", &rules), "flac");
        // 指定的音质可用时直接采用。
        assert_eq!(pick_quality(&song, "128kmp3", &rules), "128kmp3");
        assert_eq!(pick_quality(&json!({}), "flac", &rules), "flac");
    }
}
