//! 登录相关命令路由层

use crate::platforms::Platform;
use crate::ctx::AppCtx;

/// 平台暂未实现登录功能时返回的统一错误消息
const UNSUPPORTED_LOGIN: &str = "该平台暂不支持登录";

pub async fn create_qr_login(app: AppCtx, platform: String) -> Result<String, String> {
    let p = Platform::from_str(&platform)?;
    match p {
        Platform::QqMusic => crate::platforms::qqmusic::login::create_qr_login(app).await,
        Platform::Kuwo => Err(UNSUPPORTED_LOGIN.into()),
    }
}

pub async fn check_qr_login(platform: String, qrcode_id: String) -> Result<String, String> {
    let p = Platform::from_str(&platform)?;
    match p {
        Platform::QqMusic => crate::platforms::qqmusic::login::check_qr_login(qrcode_id).await,
        Platform::Kuwo => Err(UNSUPPORTED_LOGIN.into()),
    }
}

pub async fn login_with_uin_authst(
    app: AppCtx,
    platform: String,
    uin: String,
    authst: String,
    refresh_token: Option<String>,
    refresh_key: Option<String>,
    access_token: Option<String>,
    openid: Option<String>,
) -> Result<String, String> {
    let p = Platform::from_str(&platform)?;
    match p {
        Platform::QqMusic => {
            crate::platforms::qqmusic::login::login_with_uin_authst(
                app,
                uin,
                authst,
                refresh_token,
                refresh_key,
                access_token,
                openid,
            )
            .await
        }
        Platform::Kuwo => Err(UNSUPPORTED_LOGIN.into()),
    }
}

pub async fn logout(app: AppCtx, platform: String) -> Result<(), String> {
    let p = Platform::from_str(&platform)?;
    match p {
        Platform::QqMusic => crate::platforms::qqmusic::login::logout(app).await,
        Platform::Kuwo => Err(UNSUPPORTED_LOGIN.into()),
    }
}

pub async fn get_login_status(app: AppCtx, platform: String) -> Result<String, String> {
    let p = Platform::from_str(&platform)?;
    match p {
        Platform::QqMusic => crate::platforms::qqmusic::login::get_login_status(app).await,
        Platform::Kuwo => Err(UNSUPPORTED_LOGIN.into()),
    }
}
