//! 汽水音乐设置：登录 Cookie 的保存、查询与清除。
//!
//! Cookie 存在应用数据目录的独立文件里，不进入设置快照；
//! 保存后立即刷新进程内缓存，搜索与详情无需重启即可带上登录态。

use hotdownloader_core::platforms::qishui::credentials::{self, Store};
use hotdownloader_core::platforms::qishui::signer;
use tauri::{command, AppHandle, Manager};

fn store(app: &AppHandle) -> Result<Store, String> {
    Ok(Store::new(
        app.path()
            .app_data_dir()
            .map_err(|error| error.to_string())?
            .join("qishui-credentials.json"),
    ))
}

/// 只返回是否配置与键名摘要，不回显 Cookie 值。
#[command]
pub async fn get_qishui_cookie_status(app: AppHandle) -> Result<credentials::Status, String> {
    store(&app)?.status()
}

#[command]
pub async fn save_qishui_cookie(
    app: AppHandle,
    cookie: String,
) -> Result<credentials::Status, String> {
    store(&app)?.save_cookie(&cookie)
}

#[command]
pub async fn clear_qishui_cookie(app: AppHandle) -> Result<(), String> {
    store(&app)?.clear()
}

/// 签名器状态：本机 bdms.node 或远程服务是否可用，只回路径摘要与掩码后的设备号。
#[command]
pub async fn get_qishui_signer_status() -> Result<signer::Status, String> {
    Ok(signer::status())
}

/// 汽水单曲详情：搜索结果只给占位音质，下载前用它补齐真实档位（含 spatial/hi_res）。
#[command]
pub async fn qishui_song_detail(app: AppHandle, song_mid: String) -> Result<String, String> {
    let separator = crate::utils::settings::get_artist_separator(&app);
    hotdownloader_core::platforms::qishui::song::fetch_song_detail(&separator, &song_mid).await
}
