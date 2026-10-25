//! 歌单导入命令路由层

use hotdownloader_core::platforms::{self, Platform};
use tauri::{command, AppHandle};

use crate::utils::settings::get_artist_separator;

#[command]
pub async fn fetch_playlist_songs(
    app: AppHandle,
    platform: String,
    input: String,
) -> Result<String, String> {
    let p = platform.parse::<Platform>()?;
    // 歌单解析需要用户设置的歌手分隔符，具体请求无需接触 AppHandle。
    let separator = get_artist_separator(&app);
    match p {
        Platform::Netease => {
            platforms::netease::playlist::fetch_playlist_songs(
                &separator,
                input,
                &super::login::netease_store(&app)?.cookie()?,
            )
            .await
        }
        Platform::QqMusic => {
            platforms::qqmusic::playlist::fetch_playlist_songs(&separator, input).await
        }
        Platform::Kuwo => platforms::kuwo::playlist::fetch_playlist_songs(&separator, input).await,
        Platform::Kugou => {
            platforms::kugou::playlist::fetch_playlist_songs(&separator, input).await
        }
    }
}

/// 搜索歌单命令路由层。
///
/// 接收平台标识、搜索关键词、页码（从 1 开始）和每页数量，调用对应平台的歌单搜索实现。
/// 返回 JSON 字符串，格式为 `{"playlists": [...], "has_more": true/false}`。
#[command]
pub async fn search_playlists(
    app: AppHandle,
    platform: String,
    keyword: String,
    page: u32,
    limit: u32,
) -> Result<String, String> {
    let p = platform.parse::<Platform>()?;
    match p {
        Platform::Netease => {
            platforms::netease::playlist::search_playlists(
                keyword,
                page,
                limit,
                &super::login::netease_store(&app)?.cookie()?,
            )
            .await
        }
        Platform::QqMusic => {
            platforms::qqmusic::playlist::search_playlists(keyword, page, limit).await
        }
        Platform::Kuwo => platforms::kuwo::playlist::search_playlists(keyword, page, limit).await,
        Platform::Kugou => platforms::kugou::playlist::search_playlists(keyword, page, limit).await,
    }
}

/// 读取当前 QQ 登录用户创建的歌单；UIN 从 Tauri 登录存储取得。
#[command]
pub async fn fetch_created_playlists(
    app: AppHandle,
    platform: Option<String>,
) -> Result<String, String> {
    match platform
        .as_deref()
        .unwrap_or("qqmusic")
        .parse::<Platform>()?
    {
        Platform::Netease => {
            return platforms::netease::playlist::fetch_created_playlists(
                &super::login::netease_store(&app)?.cookie()?,
            )
            .await
        }
        Platform::QqMusic => {}
        _ => return Err("该平台暂不支持个人歌单".into()),
    }
    crate::platforms::qqmusic::login::fetch_created_playlists(&app).await
}

/// 读取个人歌单详情，使用 ID 与目录 ID 定位歌单。
#[command]
pub async fn fetch_created_playlist_songs(
    app: AppHandle,
    platform: Option<String>,
    id: String,
    dirid: String,
) -> Result<String, String> {
    match platform
        .as_deref()
        .unwrap_or("qqmusic")
        .parse::<Platform>()?
    {
        Platform::Netease => {
            return platforms::netease::playlist::fetch_playlist_songs(
                &get_artist_separator(&app),
                id,
                &super::login::netease_store(&app)?.cookie()?,
            )
            .await
        }
        Platform::QqMusic => {}
        _ => return Err("该平台暂不支持个人歌单".into()),
    }
    let separator = get_artist_separator(&app);
    crate::platforms::qqmusic::login::fetch_created_playlist_songs(&app, &separator, id, dirid)
        .await
}
