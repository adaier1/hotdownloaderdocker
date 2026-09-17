use crate::ctx::AppCtx;
use crate::storage::store_wrapper;

pub async fn load_history(ctx: &AppCtx) -> Result<String, String> {
    store_wrapper::load_string(ctx, "history").map_err(|e| e.to_string())
}

pub async fn save_history(ctx: &AppCtx, history_json: String) -> Result<(), String> {
    store_wrapper::save_string(ctx, "history", &history_json).map_err(|e| e.to_string())
}
