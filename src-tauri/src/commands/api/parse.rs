//! 分享链接解析命令。
//!
//! 平台与内容类型由链接自身推断，`platform` 只用于用户直接输入歌单 ID 的场景。

use hotdownloader_core::platforms::{self, Platform};
use tauri::{command, AppHandle};

use crate::utils::settings::get_artist_separator;

#[command]
pub async fn parse_share_link(
    app: AppHandle,
    input: String,
    platform: Option<String>,
) -> Result<String, String> {
    let fallback = platform
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::parse::<Platform>)
        .transpose()?;
    let link = platforms::link_parse::resolve(&input, fallback).await?;
    // 只有网易云的查询需要登录 Cookie，其余平台查询是匿名接口。
    let cookie = match link.platform {
        Platform::Netease => super::login::netease_store(&app)?.cookie()?,
        _ => String::new(),
    };
    let separator = get_artist_separator(&app);
    // QQ 详情接口对会员/版权歌曲要求登录态，带上当前凭据（未登录时为 None）。
    let qq_auth = crate::platforms::qqmusic::login::auth_from_store(&app);
    let payload = platforms::link_parse::fetch(&link, &separator, &cookie, qq_auth.as_ref()).await?;
    let data: serde_json::Value = serde_json::from_str(&payload)
        .map_err(|error| format!("解析平台响应失败: {error}"))?;
    Ok(serde_json::json!({
        "platform": link.platform.as_str(),
        "kind": link.kind.as_str(),
        "data": data,
    })
    .to_string())
}
