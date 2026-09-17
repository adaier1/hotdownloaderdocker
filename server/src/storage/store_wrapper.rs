//! 存储包装层：保持与原 Tauri 版本一致的调用形态（字符串键值），
//! 底层由 [`AppCtx`] 写入服务端 `data.json`。

use crate::ctx::AppCtx;

/// 从 `data.json` 存储加载字符串。
pub fn load_string(ctx: &AppCtx, key: &str) -> Result<String, String> {
    ctx.load_string(key)
}

/// 保存字符串到 `data.json` 存储。
pub fn save_string(ctx: &AppCtx, key: &str, value: &str) -> Result<(), String> {
    ctx.save_string(key, value)
}
