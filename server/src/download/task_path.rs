use std::path::Path;

use crate::ctx::AppCtx;

use super::task::SongInfo;
use crate::utils::filename;

/// 获取下载目录（绝对路径）及文件命名模板
pub(crate) async fn get_download_settings(
    app_handle: &AppCtx,
) -> (String, String, Option<String>, bool, bool) {
    use crate::storage::store_wrapper;

    let default_dir = crate::commands::file_ops::get_default_download_dir_impl(app_handle);
    let default_template = "{song} - {artist}".to_string();

    let settings_json = store_wrapper::load_string(app_handle, "settings").unwrap_or_default();
    let settings: serde_json::Value =
        serde_json::from_str(&settings_json).unwrap_or(serde_json::json!({}));

    let dir = settings
        .get("downloadDir")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| default_dir.clone());

    // 过滤无效路径（Android 应用私有目录）
    let dir = if dir.contains("/data/user/0/") || dir.contains("/data/data/") {
        log::warn!("检测到应用私有目录路径，已回退为默认下载目录: {}", dir);
        default_dir
    } else if Path::new(&dir).is_absolute() || dir == "saf://" {
        dir
    } else {
        log::warn!(
            "下载目录不是绝对路径，已回退为默认下载目录: {}",
            default_dir
        );
        default_dir
    };

    let saf_folder_uri = settings
        .get("safFolderUri")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let template = settings
        .get("namingTemplate")
        .and_then(|v| v.as_str())
        .unwrap_or(&default_template)
        .to_string();

    // 是否写入歌曲标签（歌词/封面）
    let write_metadata_enabled = settings
        .get("writeMetadata")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // 是否单独下载 LRC 歌词文件
    let download_lrc_enabled = settings
        .get("downloadLrc")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    (
        dir,
        template,
        saf_folder_uri,
        write_metadata_enabled,
        download_lrc_enabled,
    )
}

/// 解析最终下载路径。
/// 根据传入的设置、模板和歌曲信息，返回是否为 SAF 模式、最终路径或文件名、SAF 文件夹 URI。
/// 该函数不负责读取设置，由调用方提供，避免重复调用 `get_download_settings`。
pub(crate) fn resolve_download_path(
    dir_setting: &str,
    template_setting: &str,
    saf_uri_setting: Option<&str>,
    song_info: &SongInfo,
    quality_filename: &str,
) -> (bool, String, Option<String>) {
    if dir_setting == "saf://" && cfg!(target_os = "android") && saf_uri_setting.is_some() {
        let fname = filename::build_filename(template_setting, song_info);
        let raw_ext = Path::new(quality_filename)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("flac");
        let ext = map_decrypted_extension(raw_ext);
        let file_name = format!("{}.{}", fname, ext);
        (true, file_name, saf_uri_setting.map(|s| s.to_string()))
    } else {
        let fname = filename::build_filename(template_setting, song_info);
        let raw_ext = Path::new(quality_filename)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("flac");
        let ext = map_decrypted_extension(raw_ext);
        let full_path = Path::new(dir_setting).join(format!("{}.{}", fname, ext));
        (false, full_path.to_string_lossy().to_string(), None)
    }
}

/// 将加密文件扩展名映射为解密后的真实扩展名
pub(crate) fn map_decrypted_extension(ext: &str) -> &str {
    match ext {
        "mgg" => "ogg",
        "mflac" => "flac",
        // 未知则保持原样
        _ => ext,
    }
}
