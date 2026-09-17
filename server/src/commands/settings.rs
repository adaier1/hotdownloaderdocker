use crate::ctx::AppCtx;
use crate::storage::store_wrapper;

pub async fn load_settings(ctx: &AppCtx) -> Result<String, String> {
    store_wrapper::load_string(ctx, "settings").map_err(|e| e.to_string())
}

pub async fn save_settings(ctx: &AppCtx, settings_json: String) -> Result<(), String> {
    store_wrapper::save_string(ctx, "settings", &settings_json).map_err(|e| e.to_string())
}
