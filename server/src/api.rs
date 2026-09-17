//! HTTP API 层：RPC 分发（/api/invoke）、SSE 事件推送（/api/events）、
//! 文件下载（/api/files）与服务信息（/api/info）。
//!
//! `/api/invoke` 复刻了 Tauri `invoke` 的调用约定：请求体为
//! `{"cmd": "命令名", "args": {camelCase 参数}}`，响应体为
//! `{"code": 0, "data": <命令返回值>, "error": null}`（失败时 code=1）。

use std::collections::HashMap;
use std::convert::Infallible;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures_util::stream::{Stream, StreamExt};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tokio_util::io::ReaderStream;

use crate::commands;
use crate::ctx::AppCtx;
use crate::download::engine::DownloadEngine;

/// axum 全局状态
#[derive(Clone)]
pub struct AppState {
    pub ctx: AppCtx,
    pub engine: Arc<DownloadEngine>,
}

/// RPC 请求体
#[derive(Deserialize)]
pub struct InvokeRequest {
    pub cmd: String,
    #[serde(default)]
    pub args: Value,
}

// ==================== RPC 分发 ====================

/// 统一命令结果 → JSON Value
fn to_value<T: Serialize>(r: Result<T, String>) -> Result<Value, String> {
    r.map(|v| serde_json::to_value(v).unwrap_or(Value::Null))
}

/// 解析参数（camelCase → 结构体）
fn parse<T: DeserializeOwned>(args: &Value) -> Result<T, String> {
    serde_json::from_value(args.clone()).map_err(|e| format!("参数解析失败: {}", e))
}

/// 命令参数结构体（字段名与前端 invoke 的 camelCase 参数一致）
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlatformArgs {
    platform: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchSongsArgs {
    platform: String,
    keyword: String,
    page: u32,
    limit: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FetchDownloadLinkArgs {
    platform: String,
    song_mid: String,
    filename: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeywordArgs {
    platform: String,
    keyword: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaylistSongsArgs {
    platform: String,
    input: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SongIdArgs {
    platform: String,
    song_id: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LyricArgs {
    platform: String,
    song_id: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheckDownloadPathArgs {
    song_id: u64,
    song_mid: String,
    song_title: String,
    artist: String,
    album: String,
    cover_url: String,
    quality_filename: String,
    quality: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct QrLoginArgs {
    platform: String,
    qrcode_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginWithUinArgs {
    platform: String,
    uin: String,
    authst: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    refresh_key: Option<String>,
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    openid: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveJsonArgs {
    settings_json: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveTasksArgs {
    tasks_json: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveHistoryArgs {
    history_json: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddDownloadTaskArgs {
    task_id: String,
    platform: String,
    song_id: u64,
    song_mid: String,
    url: String,
    save_path: String,
    quality: String,
    filename: String,
    key: String,
    file_size: u64,
    song_title: String,
    artist: String,
    album: String,
    cover_url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskIdArgs {
    task_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EnqueueArgs {
    task_id: String,
    offset: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskDeleteArgs {
    task_id: String,
    delete_file: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveTasksArgs {
    task_ids: Vec<String>,
    delete_file: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MaxConcurrentArgs {
    max: u32,
}

/// 分发命令调用
async fn dispatch(state: &AppState, cmd: &str, args: &Value) -> Result<Value, String> {
    match cmd {
        // ===== 搜索 / 建议 =====
        "search_songs" => {
            let a: SearchSongsArgs = parse(args)?;
            to_value(
                commands::api::search::search_songs(
                    state.ctx.clone(),
                    a.platform,
                    a.keyword,
                    a.page,
                    a.limit,
                )
                .await,
            )
        }
        "fetch_hot_keywords" => {
            let a: PlatformArgs = parse(args)?;
            to_value(commands::api::suggest::fetch_hot_keywords(a.platform).await)
        }
        "fetch_suggestions" => {
            let a: KeywordArgs = parse(args)?;
            to_value(commands::api::suggest::fetch_suggestions(a.platform, a.keyword).await)
        }
        "fetch_cover" => {
            let a: SongIdArgs = parse(args)?;
            to_value(commands::api::search::fetch_cover(a.platform, a.song_id).await)
        }

        // ===== 歌单 =====
        "fetch_playlist_songs" => {
            let a: PlaylistSongsArgs = parse(args)?;
            to_value(
                commands::api::playlist::fetch_playlist_songs(state.ctx.clone(), a.platform, a.input)
                    .await,
            )
        }
        "search_playlists" => {
            let a: SearchSongsArgs = parse(args)?;
            to_value(
                commands::api::playlist::search_playlists(
                    a.platform,
                    a.keyword,
                    a.page,
                    a.limit,
                )
                .await,
            )
        }

        // ===== 下载链接 / 歌词 =====
        "fetch_download_link" => {
            let a: FetchDownloadLinkArgs = parse(args)?;
            to_value(
                commands::api::download::fetch_download_link(
                    state.ctx.clone(),
                    a.platform,
                    a.song_mid,
                    a.filename,
                )
                .await,
            )
        }
        "get_lyric_by_id" => {
            let a: LyricArgs = parse(args)?;
            to_value(commands::api::lyrics::get_lyric_by_id(a.platform, a.song_id).await)
        }
        "check_download_path" => {
            let a: CheckDownloadPathArgs = parse(args)?;
            to_value(
                commands::tasks::check_download_path(
                    &state.ctx,
                    a.song_id,
                    a.song_mid,
                    a.song_title,
                    a.artist,
                    a.album,
                    a.cover_url,
                    a.quality_filename,
                    a.quality,
                )
                .await,
            )
        }

        // ===== 登录 =====
        "create_qr_login" => {
            let a: PlatformArgs = parse(args)?;
            to_value(commands::api::login::create_qr_login(state.ctx.clone(), a.platform).await)
        }
        "check_qr_login" => {
            let a: QrLoginArgs = parse(args)?;
            to_value(commands::api::login::check_qr_login(a.platform, a.qrcode_id).await)
        }
        "login_with_uin_authst" => {
            let a: LoginWithUinArgs = parse(args)?;
            to_value(
                commands::api::login::login_with_uin_authst(
                    state.ctx.clone(),
                    a.platform,
                    a.uin,
                    a.authst,
                    a.refresh_token,
                    a.refresh_key,
                    a.access_token,
                    a.openid,
                )
                .await,
            )
        }
        "logout" => {
            let a: PlatformArgs = parse(args)?;
            to_value(commands::api::login::logout(state.ctx.clone(), a.platform).await)
        }
        "get_login_status" => {
            let a: PlatformArgs = parse(args)?;
            to_value(
                commands::api::login::get_login_status(state.ctx.clone(), a.platform).await,
            )
        }

        // ===== 设置 / 历史 =====
        "load_settings" => to_value(commands::settings::load_settings(&state.ctx).await),
        "save_settings" => {
            let a: SaveJsonArgs = parse(args)?;
            to_value(commands::settings::save_settings(&state.ctx, a.settings_json).await)
        }
        "load_history" => to_value(commands::history::load_history(&state.ctx).await),
        "save_history" => {
            let a: SaveHistoryArgs = parse(args)?;
            to_value(commands::history::save_history(&state.ctx, a.history_json).await)
        }
        "get_default_download_dir" => Ok(json!(commands::file_ops::get_default_download_dir(
            &state.ctx
        ))),

        // ===== 任务 =====
        "load_tasks" => to_value(commands::tasks::load_tasks(&state.ctx).await),
        "save_tasks" => {
            let a: SaveTasksArgs = parse(args)?;
            to_value(commands::tasks::save_tasks(&state.ctx, a.tasks_json).await)
        }
        "add_download_task" => {
            let a: AddDownloadTaskArgs = parse(args)?;
            to_value(
                commands::tasks::add_download_task(
                    &state.ctx,
                    &state.engine,
                    a.task_id,
                    a.platform,
                    a.song_id,
                    a.song_mid,
                    a.url,
                    a.save_path,
                    a.quality,
                    a.filename,
                    a.key,
                    a.file_size,
                    a.song_title,
                    a.artist,
                    a.album,
                    a.cover_url,
                )
                .await,
            )
        }
        "enqueue_task" => {
            let a: EnqueueArgs = parse(args)?;
            to_value(
                commands::tasks::enqueue_task(&state.ctx, &state.engine, a.task_id, a.offset).await,
            )
        }
        "pause_task" => {
            let a: TaskIdArgs = parse(args)?;
            to_value(commands::tasks::pause_task(&state.ctx, &state.engine, a.task_id).await)
        }
        "resume_task" => {
            let a: TaskIdArgs = parse(args)?;
            to_value(commands::tasks::resume_task(&state.ctx, &state.engine, a.task_id).await)
        }
        "cancel_task" => {
            let a: TaskDeleteArgs = parse(args)?;
            to_value(
                commands::tasks::cancel_task(&state.ctx, &state.engine, a.task_id, a.delete_file)
                    .await,
            )
        }
        "remove_task" => {
            let a: TaskDeleteArgs = parse(args)?;
            to_value(
                commands::tasks::remove_task(&state.ctx, &state.engine, a.task_id, a.delete_file)
                    .await,
            )
        }
        "remove_tasks" => {
            let a: RemoveTasksArgs = parse(args)?;
            to_value(
                commands::tasks::remove_tasks(
                    &state.ctx,
                    &state.engine,
                    a.task_ids,
                    a.delete_file,
                )
                .await,
            )
        }
        "set_max_concurrent" => {
            let a: MaxConcurrentArgs = parse(args)?;
            to_value(commands::tasks::set_max_concurrent(&state.ctx, &state.engine, a.max))
        }

        // ===== 通知 =====
        "request_notification_permission" => {
            to_value(commands::notify::request_notification_permission(&state.ctx).await)
        }
        "check_notification_permission" => {
            to_value(commands::notify::check_notification_permission(&state.ctx).await)
        }

        // ===== 更新检查 =====
        "check_update" => to_value(commands::api::update::check_update().await),

        _ => Err(format!("未知命令: {}", cmd)),
    }
}

/// POST /api/invoke
pub async fn invoke_handler(
    State(state): State<AppState>,
    Json(req): Json<InvokeRequest>,
) -> Json<Value> {
    match dispatch(&state, &req.cmd, &req.args).await {
        Ok(value) => Json(json!({ "code": 0, "data": value, "error": null })),
        Err(e) => Json(json!({ "code": 1, "data": null, "error": e })),
    }
}

// ==================== SSE 事件推送 ====================

/// GET /api/events —— Server-Sent Events 实时推送下载进度/完成/错误等事件。
/// 新连接先回放最近的事件历史（防止断线重连后错过关键事件），再接收实时事件。
pub async fn events_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (rx, history) = state.ctx.subscribe();

    // 历史回放
    let replay = futures_util::stream::iter(history.into_iter().map(|(event, payload)| {
        Ok::<_, Infallible>(Event::default().event(event).data(payload))
    }));

    // 实时事件
    let live = futures_util::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok((event, payload)) => {
                    return Some((
                        Ok::<_, Infallible>(Event::default().event(event).data(payload)),
                        rx,
                    ))
                }
                // 消费者积压导致跳过旧事件：进度类事件可丢弃，继续接收
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                // 发送端关闭（不会发生），结束流
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });

    Sse::new(replay.chain(live)).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

// ==================== 文件下载 ====================

/// GET /api/files?name=<文件名> —— 下载服务端下载目录中的文件（仅文件名，防目录穿越）。
pub async fn files_handler(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let name = params.get("name").cloned().unwrap_or_default();
    // 仅允许纯文件名（不含任何路径分隔符 / 父目录引用）
    let file_name = Path::new(&name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    if file_name.is_empty() || file_name != name {
        return (
            StatusCode::BAD_REQUEST,
            "无效的文件名：只允许下载服务端下载目录中的文件",
        )
            .into_response();
    }

    let path = state.ctx.download_dir().join(&file_name);
    if !path.is_file() {
        return (StatusCode::NOT_FOUND, "文件不存在或已被删除").into_response();
    }

    match tokio::fs::File::open(&path).await {
        Ok(file) => {
            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);
            let mut resp = Response::new(body);
            resp.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/octet-stream"),
            );
            // RFC 5987 UTF-8 文件名编码，支持中文文件名
            let encoded: String = url::form_urlencoded::byte_serialize(file_name.as_bytes()).collect();
            resp.headers_mut().insert(
                header::CONTENT_DISPOSITION,
                HeaderValue::from_str(&format!("attachment; filename*=UTF-8''{}", encoded))
                    .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
            );
            resp
        }
        Err(e) => {
            log::error!("读取文件失败 {}: {}", path.display(), e);
            (StatusCode::INTERNAL_SERVER_ERROR, "文件读取失败").into_response()
        }
    }
}

// ==================== 服务信息 ====================

/// GET /api/info —— 服务端信息（版本、下载目录等）
pub async fn info_handler(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "mode": "web",
        "version": env!("CARGO_PKG_VERSION"),
        "download_dir": state.ctx.download_dir().to_string_lossy(),
        "data_dir": state.ctx.data_dir().to_string_lossy(),
    }))
}
//（注：内容由AI生成）
