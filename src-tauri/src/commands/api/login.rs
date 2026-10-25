//! 登录相关命令路由层

use hotdownloader_core::platforms::{netease, Platform};
use serde::Deserialize;
use tauri::Manager;
use tauri::{command, AppHandle};

pub(crate) fn netease_store(app: &AppHandle) -> Result<netease::login::Store, String> {
    Ok(netease::login::Store::new(
        app.path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
            .join("netease-credentials.json"),
    ))
}

#[command]
pub async fn netease_login(
    app: AppHandle,
    action: String,
    id: Option<String>,
    credentials: Option<netease::login::ManualCredentials>,
) -> Result<serde_json::Value, String> {
    netease::login::execute(
        &netease_store(&app)?,
        &action,
        id.as_deref().unwrap_or(""),
        credentials,
    )
    .await
}

pub(crate) fn kugou_store(
    app: &AppHandle,
) -> Result<hotdownloader_core::platforms::kugou::login::Store, String> {
    let directory = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(hotdownloader_core::platforms::kugou::login::Store::new(
        directory.join("kugou-credentials.json"),
    ))
}

#[command]
pub async fn kugou_login(
    app: AppHandle,
    action: String,
    id: Option<String>,
    credentials: Option<hotdownloader_core::platforms::kugou::login::ManualCredentials>,
) -> Result<serde_json::Value, String> {
    hotdownloader_core::platforms::kugou::login::execute(
        &kugou_store(&app)?,
        &action,
        id.as_deref().unwrap_or(""),
        credentials,
    )
    .await
}

/// 平台暂未实现登录功能时返回的统一错误消息
const UNSUPPORTED_LOGIN: &str = "该平台暂不支持登录";

/// 手动登录的 IPC 载荷；可选令牌保留现有前端字段名。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualLoginInput {
    uin: String,
    authst: String,
    refresh_token: Option<String>,
    refresh_key: Option<String>,
    access_token: Option<String>,
    openid: Option<String>,
}

#[command]
pub async fn create_qr_login(app: AppHandle, platform: String) -> Result<String, String> {
    let p = platform.parse::<Platform>()?;
    match p {
        Platform::QqMusic => crate::platforms::qqmusic::login::create_qr_login(app).await,
        Platform::Kuwo | Platform::Kugou | Platform::Netease => Err(UNSUPPORTED_LOGIN.into()),
    }
}

#[command]
pub async fn check_qr_login(platform: String, qrcode_id: String) -> Result<String, String> {
    let p = platform.parse::<Platform>()?;
    match p {
        Platform::QqMusic => crate::platforms::qqmusic::login::check_qr_login(qrcode_id).await,
        Platform::Kuwo | Platform::Kugou | Platform::Netease => Err(UNSUPPORTED_LOGIN.into()),
    }
}

#[command]
pub async fn login_with_uin_authst(
    app: AppHandle,
    platform: String,
    credentials: ManualLoginInput,
) -> Result<String, String> {
    let p = platform.parse::<Platform>()?;
    match p {
        Platform::QqMusic => {
            crate::platforms::qqmusic::login::login_with_uin_authst(
                app,
                credentials.uin,
                credentials.authst,
                credentials.refresh_token,
                credentials.refresh_key,
                credentials.access_token,
                credentials.openid,
            )
            .await
        }
        Platform::Kuwo | Platform::Kugou | Platform::Netease => Err(UNSUPPORTED_LOGIN.into()),
    }
}

#[command]
pub async fn logout(app: AppHandle, platform: String) -> Result<(), String> {
    let p = platform.parse::<Platform>()?;
    match p {
        Platform::QqMusic => crate::platforms::qqmusic::login::logout(app).await,
        Platform::Kuwo | Platform::Kugou | Platform::Netease => Err(UNSUPPORTED_LOGIN.into()),
    }
}

#[command]
pub async fn get_login_status(app: AppHandle, platform: String) -> Result<String, String> {
    let p = platform.parse::<Platform>()?;
    match p {
        Platform::QqMusic => crate::platforms::qqmusic::login::get_login_status(app).await,
        Platform::Kuwo | Platform::Kugou | Platform::Netease => Err(UNSUPPORTED_LOGIN.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::ManualLoginInput;

    #[test]
    fn manual_login_ipc_payload_uses_existing_field_names() {
        let input: ManualLoginInput = serde_json::from_value(serde_json::json!({
            "uin": "12345",
            "authst": "token",
            "refreshToken": "refresh",
            "refreshKey": "key",
            "accessToken": "access",
            "openid": "open-id",
        }))
        .unwrap();
        assert_eq!(input.uin, "12345");
        assert_eq!(input.refresh_token.as_deref(), Some("refresh"));
        assert_eq!(input.refresh_key.as_deref(), Some("key"));
        assert_eq!(input.access_token.as_deref(), Some("access"));
        assert_eq!(input.openid.as_deref(), Some("open-id"));
    }
}
