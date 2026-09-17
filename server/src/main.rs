//! HotDownloader Rust API 服务入口。
//!
//! 原项目为 Tauri 2 桌面应用，本服务把其 Rust 后端（搜索/歌单/登录/下载引擎）
//! 抽离为一个独立的 HTTP + SSE 服务，前端（Vue 3 SPA）通过 `/api` 访问：
//!
//! - `POST /api/invoke`  统一 RPC 分发（与 Tauri invoke 约定一致）
//! - `GET  /api/events`  SSE 实时事件流（下载进度/完成/错误…）
//! - `GET  /api/files`   下载服务端已下载的文件
//! - `GET  /api/info`    服务信息

mod api;
mod commands;
mod ctx;
mod download;
mod events;
mod platforms;
mod storage;
mod utils;

use std::path::PathBuf;
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tokio::net::TcpListener;

use ctx::AppCtx;
use download::engine::DownloadEngine;

#[tokio::main]
async fn main() {
    // 日志：默认 info，可通过 RUST_LOG 调整
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let data_dir = PathBuf::from(std::env::var("DATA_DIR").unwrap_or_else(|_| "/data".to_string()));
    let download_dir = PathBuf::from(
        std::env::var("DOWNLOAD_DIR").unwrap_or_else(|_| "/downloads".to_string()),
    );
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        log::error!("创建数据目录失败 {}: {}", data_dir.display(), e);
    }
    if let Err(e) = std::fs::create_dir_all(&download_dir) {
        log::error!("创建下载目录失败 {}: {}", download_dir.display(), e);
    }

    let ctx = AppCtx::new(data_dir, download_dir);

    // 初始化下载引擎，并恢复用户设置中的最大并发数
    let engine = Arc::new(DownloadEngine::new(ctx.clone()));
    let max_concurrent = ctx
        .load_string("settings")
        .ok()
        .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
        .and_then(|v| v.get("maxConcurrent")?.as_u64())
        .map(|n| n as u32)
        .unwrap_or(3);
    engine.set_concurrency(max_concurrent);

    // 启动下载调度器
    let scheduler_engine = engine.clone();
    tokio::spawn(async move {
        scheduler_engine.run_scheduler().await;
    });

    let state = api::AppState { ctx, engine };

    let app = Router::new()
        .route("/api/invoke", post(api::invoke_handler))
        .route("/api/events", get(api::events_handler))
        .route("/api/files", get(api::files_handler))
        .route("/api/info", get(api::info_handler))
        .with_state(state);

    let listener = match TcpListener::bind(&bind_addr).await {
        Ok(listener) => listener,
        Err(e) => {
            log::error!("监听 {} 失败: {}", bind_addr, e);
            std::process::exit(1);
        }
    };

    log::info!("HotDownloader API 服务已启动: http://{}", bind_addr);
    if let Err(e) = axum::serve(listener, app).await {
        log::error!("服务运行错误: {}", e);
    }
}
