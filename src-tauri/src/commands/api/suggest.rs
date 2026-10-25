//! 热搜与搜索建议命令路由层

use hotdownloader_core::platforms::{self, Platform};
use tauri::{command, AppHandle};

#[command]
pub async fn fetch_hot_keywords(app: AppHandle, platform: String) -> Result<String, String> {
    let p = platform.parse::<Platform>()?;
    match p {
        Platform::Netease => {
            platforms::netease::suggest::fetch_hot_keywords(
                &super::login::netease_store(&app)?.cookie()?,
            )
            .await
        }
        Platform::QqMusic => platforms::qqmusic::suggest::fetch_hot_keywords().await,
        Platform::Kuwo => platforms::kuwo::suggest::fetch_hot_keywords().await,
        Platform::Kugou => platforms::kugou::suggest::fetch_hot_keywords().await,
    }
}

#[command]
pub async fn fetch_suggestions(
    app: AppHandle,
    platform: String,
    keyword: String,
) -> Result<String, String> {
    let p = platform.parse::<Platform>()?;
    match p {
        Platform::Netease => {
            platforms::netease::suggest::fetch_suggestions(
                &crate::utils::settings::get_artist_separator(&app),
                keyword,
                &super::login::netease_store(&app)?.cookie()?,
            )
            .await
        }
        Platform::QqMusic => platforms::qqmusic::suggest::fetch_suggestions(keyword).await,
        Platform::Kuwo => platforms::kuwo::suggest::fetch_suggestions(keyword).await,
        Platform::Kugou => platforms::kugou::suggest::fetch_suggestions(keyword).await,
    }
}
