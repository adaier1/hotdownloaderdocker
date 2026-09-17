//! 通知模块（Web 版）。
//!
//! 桌面端使用 `tauri-plugin-notification` 弹系统通知；Web 版改为通过 SSE
//! 推送 `download-notification` 事件，由浏览器端的 Notification API 展示。

use crate::ctx::AppCtx;

/// 发送“下载完成”通知事件。
pub fn send_download_complete_notification(ctx: &AppCtx, song_title: &str, artist: &str) {
    ctx.emit(
        "download-notification",
        serde_json::json!({
            "title": format!("下载完成：{}", song_title),
            "body": format!("{} - {}", song_title, artist),
        }),
    );
}

/// 通知权限由浏览器自行管理，服务端直接返回已授权。
pub async fn request_notification_permission(_ctx: &AppCtx) -> Result<bool, String> {
    Ok(true)
}

/// 通知权限由浏览器自行管理，服务端直接返回已授权。
pub async fn check_notification_permission(_ctx: &AppCtx) -> Result<bool, String> {
    Ok(true)
}
