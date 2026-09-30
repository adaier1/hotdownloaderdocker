//! 下载目录音频文件管理。
//!
//! 曲库展示的是磁盘上真实存在的歌曲文件，而不是任务记录，因此这里
//! 直接读写下载目录一级目录中的音频文件：扫描、删除、重命名、写入标签。
//! 供 Tauri 命令、独立服务 HTTP 入口与 MCP 工具共用。

use std::path::{Path, PathBuf};

use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::{ItemKey, Tag, TagType};
use serde::{Deserialize, Serialize};

/// 下载落盘后常见的可播放音频扩展名（小写、不含点）。
const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "m4a", "aac", "ogg", "opus", "wav", "wma", "ape",
];

/// 重命名时会被替换为下划线的非法字符。
const ILLEGAL_NAME_CHARS: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

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

/// 校验并定位下载目录内的音频文件；只取文件名，防止目录穿越。
fn resolve_audio_file(directory: &str, path: &str) -> Result<PathBuf, String> {
    let dir = Path::new(directory);
    if !dir.is_dir() {
        return Err(format!("下载目录不存在: {directory}"));
    }
    let name = Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("非法文件路径")?;
    let target = dir.join(name);
    let extension = target
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !AUDIO_EXTENSIONS.contains(&extension.as_str()) {
        return Err("不是可操作的音频文件".to_string());
    }
    if !target.is_file() {
        return Err(format!("文件不存在: {}", target.display()));
    }
    Ok(target)
}

/// 根据磁盘文件构造 `AudioFileEntry`。
fn entry_for(path: &Path) -> Result<AudioFileEntry, String> {
    let metadata =
        std::fs::metadata(path).map_err(|error| format!("读取文件信息失败: {error}"))?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("非法文件名")?
        .to_string();
    Ok(AudioFileEntry {
        name,
        path: path.to_string_lossy().into_owned(),
        size: metadata.len(),
        extension,
    })
}

/// 规范化用户提供的文件名：去掉目录部分、替换非法字符、必要时补上音频扩展名。
fn normalize_file_name(raw: &str, fallback_extension: &str) -> Result<String, String> {
    let name = Path::new(raw)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("非法文件名")?
        .trim();
    if name.is_empty() {
        return Err("文件名不能为空".to_string());
    }
    let mut cleaned: String = name
        .chars()
        .map(|ch| if ILLEGAL_NAME_CHARS.contains(&ch) { '_' } else { ch })
        .collect();
    let extension = Path::new(&cleaned)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !AUDIO_EXTENSIONS.contains(&extension.as_str()) {
        let base = cleaned.trim_end_matches('.').trim();
        if base.is_empty() {
            return Err("文件名不能为空".to_string());
        }
        cleaned = format!("{base}.{fallback_extension}");
    }
    Ok(cleaned)
}

/// 重命名下载目录内的音频文件，返回新的文件条目。
///
/// `new_name` 只取文件名部分；不含受支持扩展名时会沿用原扩展名。
pub fn rename_audio_file(
    directory: &str,
    path: &str,
    new_name: &str,
) -> Result<AudioFileEntry, String> {
    let old_path = resolve_audio_file(directory, path)?;
    let fallback_extension = old_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("mp3")
        .to_ascii_lowercase();
    let file_name = normalize_file_name(new_name, &fallback_extension)?;
    let new_path = Path::new(directory).join(&file_name);

    if new_path == old_path {
        return entry_for(&old_path);
    }
    if new_path.exists() {
        return Err(format!("目标文件已存在: {file_name}"));
    }
    std::fs::rename(&old_path, &new_path).map_err(|error| format!("重命名失败: {error}"))?;
    entry_for(&new_path)
}

/// 音频标签修改请求：仅提供的字段会被写入，空字符串表示清除该字段，
/// 未提供的字段保持不变。
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataUpdate {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub lyrics: Option<String>,
}

/// 写入/修改下载目录内音频文件的标签（标题、歌手、专辑、歌词）。
pub fn write_audio_metadata(
    directory: &str,
    path: &str,
    update: &MetadataUpdate,
) -> Result<(), String> {
    if update.title.is_none()
        && update.artist.is_none()
        && update.album.is_none()
        && update.lyrics.is_none()
    {
        return Err("没有需要修改的标签字段".to_string());
    }
    let target = resolve_audio_file(directory, path)?;

    let mut tagged_file =
        lofty::read_from_path(&target).map_err(|error| format!("读取音频文件失败: {error}"))?;
    // 与下载收尾保持一致：移除 128 字节的旧 ID3v1，避免多字节字符截断。
    if tagged_file.remove(TagType::Id3v1).is_some() {
        log::info!("已移除 ID3v1 标签，避免多字节字符截断引发崩溃");
    }
    let tag_type = tagged_file.primary_tag_type();
    if tagged_file.primary_tag().is_none() {
        tagged_file.insert_tag(Tag::new(tag_type));
    }
    let tag = tagged_file
        .primary_tag_mut()
        .ok_or_else(|| "无法获取音频标签".to_string())?;

    for (key, value) in [
        (ItemKey::TrackTitle, update.title.as_ref()),
        (ItemKey::TrackArtist, update.artist.as_ref()),
        (ItemKey::AlbumTitle, update.album.as_ref()),
        (ItemKey::Lyrics, update.lyrics.as_ref()),
    ] {
        let Some(value) = value else { continue };
        tag.remove_key(&key);
        if !value.is_empty() {
            tag.insert_text(key, value.clone());
        }
    }

    tagged_file
        .save_to_path(&target, WriteOptions::default())
        .map_err(|error| format!("保存标签失败: {error}"))?;
    log::info!("音频标签已写入: {}", target.display());
    Ok(())
}
