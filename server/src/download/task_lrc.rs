use std::fs;
use std::path::Path;

use crate::ctx::AppCtx;

/// 将普通 LRC 歌词写入与歌曲同名的 `.lrc` 文件。
/// 所有错误仅记录日志，不阻塞主下载流程。
pub(crate) async fn write_lrc_file(
    _ctx: &AppCtx,
    lrc_content: &str,
    song_file_path: &str,
) -> Option<String> {
    // 提取歌曲文件名的 stem（不含扩展名）
    let song_name = Path::new(song_file_path);
    let stem = song_name
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    // 与歌曲文件同目录，生成 .lrc 文件
    let parent = song_name.parent().unwrap_or_else(|| Path::new("."));
    let lrc_path = parent.join(format!("{}.lrc", stem));
    if let Err(e) = fs::write(&lrc_path, lrc_content) {
        log::warn!("写入 LRC 歌词文件失败 {}: {}", lrc_path.display(), e);
        None
    } else {
        log::info!("LRC 歌词文件已保存: {}", lrc_path.display());
        Some(lrc_path.to_string_lossy().to_string())
    }
}
