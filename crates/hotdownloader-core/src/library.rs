//! 下载目录音频文件扫描。
//!
//! 曲库页面展示的是磁盘上真实存在的歌曲文件，而不是任务记录，
//! 因此这里只读地枚举下载目录一级目录中的音频文件，供 Tauri 命令与
//! 独立服务 HTTP 入口共用。扫描过程不修改任何状态。

use std::path::Path;

use serde::Serialize;

/// 下载落盘后常见的可播放音频扩展名（小写、不含点）。
const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "m4a", "aac", "ogg", "opus", "wav", "wma", "ape",
];

/// 单个音频文件的元信息，字段与前端 `AudioFileEntry` 契约保持一致。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFileEntry {
    /// 文件名（含扩展名）。
    pub name: String,
    /// 文件绝对路径。
    pub path: String,
    /// 文件字节大小。
    pub size: u64,
    /// 小写扩展名，不含点。
    pub extension: String,
}

/// 扫描 `directory` 一级目录，返回按文件名排序的音频文件。
///
/// 目录不存在或不可读时返回错误；单个损坏条目会被跳过，不中断整个列表。
pub fn list_audio_files(directory: &str) -> Result<Vec<AudioFileEntry>, String> {
    let entries = std::fs::read_dir(Path::new(directory))
        .map_err(|error| format!("读取下载目录失败: {error}"))?;

    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(metadata) = entry.metadata() else { continue };
        if !metadata.is_file() {
            continue;
        }
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !AUDIO_EXTENSIONS.contains(&extension.as_str()) {
            continue;
        }
        files.push(AudioFileEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: path.to_string_lossy().into_owned(),
            size: metadata.len(),
            extension,
        });
    }

    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

/// 单个文件删除失败的信息。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFailure {
    /// 请求删除的原始路径。
    pub path: String,
    /// 失败原因。
    pub error: String,
}

/// 批量删除结果。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResult {
    /// 成功删除的文件数量。
    pub deleted: usize,
    /// 删除失败的条目。
    pub failed: Vec<DeleteFailure>,
}

/// 删除 `directory` 下由 `paths` 指定的音频文件。
///
/// 为防止目录穿越，只取每个路径的文件名并拼接回受信任的下载目录，
/// 因此调用方即使传入任意路径也无法删除目录外的文件。非音频扩展名会被拒绝。
pub fn delete_audio_files(directory: &str, paths: &[String]) -> Result<DeleteResult, String> {
    let dir = Path::new(directory);
    if !dir.is_dir() {
        return Err(format!("下载目录不存在: {directory}"));
    }

    let mut deleted = 0usize;
    let mut failed = Vec::new();
    for raw in paths {
        let Some(name) = Path::new(raw).file_name().and_then(|name| name.to_str()) else {
            failed.push(DeleteFailure {
                path: raw.clone(),
                error: "非法文件名".to_string(),
            });
            continue;
        };
        let target = dir.join(name);
        let extension = target
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !AUDIO_EXTENSIONS.contains(&extension.as_str()) {
            failed.push(DeleteFailure {
                path: raw.clone(),
                error: "不是可删除的音频文件".to_string(),
            });
            continue;
        }
        match std::fs::remove_file(&target) {
            Ok(()) => deleted += 1,
            Err(error) => failed.push(DeleteFailure {
                path: raw.clone(),
                error: error.to_string(),
            }),
        }
    }

    Ok(DeleteResult { deleted, failed })
}
