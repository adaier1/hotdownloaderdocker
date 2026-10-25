//! 歌词获取命令路由层

use hotdownloader_core::platforms::lyric::LyricData;
use hotdownloader_core::platforms::{self, Platform};
use tauri::command;

/// Tauri 命令：获取歌词。
#[command]
pub async fn get_lyric_by_id(
    platform: String,
    song_id: u64,
    song_mid: Option<String>,
) -> Result<LyricData, String> {
    match platform.parse::<Platform>()? {
        Platform::Netease => platforms::netease::lyrics::get_lyric_by_id(song_id).await,
        Platform::Kugou => {
            platforms::kugou::lyrics::get_lyric(song_mid.as_deref().unwrap_or("")).await
        }
        Platform::QqMusic => platforms::qqmusic::lyrics::get_lyric_by_id(song_id).await,
        Platform::Kuwo => platforms::kuwo::lyrics::get_lyric_by_id(song_id).await,
    }
}
