//! MCP（Model Context Protocol）服务端。
//!
//! 以 Streamable HTTP（`POST /mcp`）把 HotDownloader 的核心能力暴露为 MCP 工具，
//! 供支持 MCP 的客户端调用。该模块**完全独立**于既有 Web/API：只在
//! `http::handle` 中新增一个 `/mcp` 路由，复用 `ServerRuntime` 与共享核心，
//! 不修改任何既有行为。
//!
//! 认证复用服务既有的 Basic / Bearer 规则（与 `/api` 相同）。
//! 未声明 `resources`/`prompts` 能力，但仍对探测请求返回空列表以兼容客户端。

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use serde_json::{json, Value};

use hotdownloader_core::platforms::qqmusic::login as qq_login;
use hotdownloader_core::settings::patch::{SettingsPatch, SettingsPatchError};
use hotdownloader_core::task::contract::CreateTaskRequest;
use hotdownloader_core::task::service::TaskService;

use crate::http::{error_response, json_response, read_json, HttpBody};
use crate::music;
use crate::runtime::ServerRuntime;

/// 处理 `POST /mcp`；其它方法返回 405。
pub async fn handle(request: Request<Incoming>, runtime: Arc<ServerRuntime>) -> Response<HttpBody> {
    if request.method() != Method::POST {
        return Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .header("allow", "POST")
            .body(Full::new(Bytes::new()).boxed_unsync())
            .expect("固定响应头应始终有效");
    }

    // 认证：支持既有 Basic / Bearer，也支持 URL 中的 `?key=`（设置页生成的 MCP 链接）。
    let query_key = request
        .uri()
        .query()
        .and_then(|query| {
            query.split('&').find_map(|pair| {
                let (name, value) = pair.split_once('=')?;
                (name == "key").then_some(value)
            })
        })
        .unwrap_or_default();
    let authorization = request
        .headers()
        .get("authorization")
        .and_then(|header| header.to_str().ok());
    let authorized = runtime.mcp_key.verify(query_key) || runtime.auth().accepts(authorization);
    if !authorized {
        return error_response(StatusCode::UNAUTHORIZED, "访问凭据无效");
    }

    let message = match read_json(request).await {
        Ok(value) => value,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, error),
    };

    match handle_message(runtime, message).await {
        Some(response) => json_response(StatusCode::OK, response),
        // 通知类消息无需响应，按 MCP 规范返回 202 且不带正文。
        None => Response::builder()
            .status(StatusCode::ACCEPTED)
            .body(Full::new(Bytes::new()).boxed_unsync())
            .expect("固定响应头应始终有效"),
    }
}

/// 处理单条 JSON-RPC 消息；无 `id` 的通知返回 `None`。
async fn handle_message(runtime: Arc<ServerRuntime>, message: Value) -> Option<Value> {
    let id = message.get("id").cloned()?;
    let method = message
        .get("method")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let params = message.get("params").cloned().unwrap_or(Value::Null);

    let result: Result<Value, (i64, String)> = match method {
        "initialize" => Ok(initialize_result(&params)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => call_tool(&runtime, &params).await,
        // 未声明对应能力，返回空列表以便客户端探测。
        "resources/list" => Ok(json!({ "resources": [] })),
        "prompts/list" => Ok(json!({ "prompts": [] })),
        _ => Err((-32601, format!("未知方法: {method}"))),
    };

    Some(match result {
        Ok(value) => json!({ "jsonrpc": "2.0", "id": id, "result": value }),
        Err((code, message)) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": code, "message": message },
        }),
    })
}

fn initialize_result(params: &Value) -> Value {
    // 回显客户端请求的协议版本，最大化与不同版本客户端的兼容性。
    let requested = params
        .get("protocolVersion")
        .and_then(|value| value.as_str())
        .unwrap_or("2024-11-05");
    json!({
        "protocolVersion": requested,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": "hotdownloader", "version": env!("CARGO_PKG_VERSION") },
        "instructions": "HotDownloader 音乐下载服务：可搜索歌曲/歌单/专辑/歌手、获取歌词、创建与管理下载任务、读写设置，以及管理曲库（列出/删除/重命名下载目录中的音频文件并写入标签）。"
    })
}

// ==================== 工具分发 ====================

async fn call_tool(runtime: &Arc<ServerRuntime>, params: &Value) -> Result<Value, (i64, String)> {
    let name = params
        .get("name")
        .and_then(|value| value.as_str())
        .ok_or((-32602, "缺少工具名称".to_string()))?;
    let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));

    let outcome: Result<Value, String> = match name {
        // 音乐查询：复用 HTTP 音乐入口的同一套核心分派。
        "search_songs" => run_music(runtime, "songs/search", args).await,
        "search_albums" => run_music(runtime, "albums/search", args).await,
        "search_artists" => run_music(runtime, "artists/search", args).await,
        "fetch_album_songs" => run_music(runtime, "albums/fetch", args).await,
        "fetch_artist_songs" => run_music(runtime, "artists/songs", args).await,
        "fetch_artist_albums" => run_music(runtime, "artists/albums", args).await,
        "fetch_playlist_songs" => run_music(runtime, "playlists/fetch", args).await,
        "search_playlists" => run_music(runtime, "playlists/search", args).await,
        "fetch_hot_keywords" => run_music(runtime, "hot-keywords", args).await,
        "fetch_suggestions" => run_music(runtime, "suggestions", args).await,
        "get_lyric_by_id" => run_music(runtime, "lyrics", args).await,
        // 任务
        "list_tasks" => Ok(json!(runtime.tasks.list())),
        "create_download_task" => create_download_task(runtime, args).await,
        "pause_task" => task_action(runtime, "pause", args).await,
        "resume_task" => task_action(runtime, "resume", args).await,
        "retry_task" => task_action(runtime, "retry", args).await,
        "remove_task" => task_action(runtime, "remove", args).await,
        "remove_tasks" => remove_tasks(runtime, args).await,
        // 设置 / 登录
        "get_settings" => Ok(json!(runtime.environment.settings_snapshot())),
        "patch_settings" => patch_settings(runtime, args),
        "get_default_download_dir" => Ok(json!(runtime.environment.default_download_dir())),
        "get_login_status" => get_login_status(runtime).await,
        // 曲库（下载目录）
        "list_library" => list_library(runtime, args),
        "delete_library_files" => delete_library_files(runtime, args),
        "rename_library_file" => rename_library_file(runtime, args),
        "write_library_metadata" => write_library_metadata(runtime, args),
        _ => return Err((-32602, format!("未知工具: {name}"))),
    };

    Ok(match outcome {
        Ok(value) => tool_result(&value.to_string(), false),
        Err(error) => tool_result(&error, true),
    })
}

fn tool_result(text: &str, is_error: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": is_error,
    })
}

async fn run_music(runtime: &ServerRuntime, action: &str, args: Value) -> Result<Value, String> {
    let request = serde_json::from_value::<music::MusicRequest>(args)
        .map_err(|error| format!("参数解析失败: {error}"))?;
    let separator = runtime.environment.artist_separator();
    music::execute(action, request, &separator).await
}

async fn create_download_task(runtime: &ServerRuntime, args: Value) -> Result<Value, String> {
    let request = serde_json::from_value::<CreateTaskRequest>(args)
        .map_err(|error| format!("参数解析失败: {error}"))?;
    let service = TaskService::new(&runtime.tasks, &runtime.engine, runtime.environment.as_ref());
    let result = service.create_download_task(request).await?;
    serde_json::to_value(result).map_err(|error| error.to_string())
}

async fn task_action(runtime: &ServerRuntime, action: &str, args: Value) -> Result<Value, String> {
    let task_id = args
        .get("taskId")
        .and_then(|value| value.as_str())
        .ok_or("缺少 taskId")?
        .to_string();
    let delete_file = args
        .get("deleteFile")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let service = TaskService::new(&runtime.tasks, &runtime.engine, runtime.environment.as_ref());
    match action {
        "pause" => {
            service.pause_task(task_id).await?;
            Ok(json!({ "ok": true }))
        }
        "resume" => {
            service.resume_task(task_id).await?;
            Ok(json!({ "ok": true }))
        }
        "retry" => {
            let retried = service.retry_task(task_id).await?;
            Ok(json!({ "retried": retried }))
        }
        "remove" => {
            service.remove_task(task_id, delete_file).await?;
            Ok(json!({ "ok": true }))
        }
        _ => Err(format!("未知任务操作: {action}")),
    }
}

async fn remove_tasks(runtime: &ServerRuntime, args: Value) -> Result<Value, String> {
    let task_ids: Vec<String> =
        serde_json::from_value(args.get("taskIds").cloned().unwrap_or_else(|| json!([])))
            .map_err(|error| format!("taskIds 解析失败: {error}"))?;
    let delete_file = args
        .get("deleteFile")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let service = TaskService::new(&runtime.tasks, &runtime.engine, runtime.environment.as_ref());
    let result = service.remove_tasks(task_ids, delete_file).await?;
    serde_json::to_value(result).map_err(|error| error.to_string())
}

fn patch_settings(runtime: &ServerRuntime, args: Value) -> Result<Value, String> {
    let patch: SettingsPatch =
        serde_json::from_value(args).map_err(|error| format!("参数解析失败: {error}"))?;
    match runtime.patch_settings(patch) {
        Ok(snapshot) => serde_json::to_value(snapshot).map_err(|error| error.to_string()),
        Err(SettingsPatchError::Invalid(message)) => Err(message),
        Err(SettingsPatchError::Conflict { fields, .. }) => {
            Err(format!("设置冲突: {}", fields.join(", ")))
        }
    }
}

async fn get_login_status(runtime: &ServerRuntime) -> Result<Value, String> {
    let result = qq_login::get_login_status(runtime.login_store.as_ref()).await?;
    serde_json::from_str(&result).map_err(|error| error.to_string())
}

// ==================== 曲库（下载目录） ====================

/// 曲库操作统一作用于服务端下载目录。
fn library_directory(runtime: &ServerRuntime) -> &str {
    runtime.environment.default_download_dir()
}

/// 列出下载目录中的音频文件，支持按文件名过滤与分页。
fn list_library(runtime: &ServerRuntime, args: Value) -> Result<Value, String> {
    let directory = library_directory(runtime);
    let mut files = hotdownloader_core::library::list_audio_files(directory)?;

    let keyword = args
        .get("keyword")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .trim()
        .to_lowercase();
    if !keyword.is_empty() {
        files.retain(|file| file.name.to_lowercase().contains(&keyword));
    }

    let total = files.len();
    let offset = args
        .get("offset")
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as usize;
    let limit = args
        .get("limit")
        .and_then(|value| value.as_u64())
        .map(|value| value as usize);
    let files: Vec<_> = match limit {
        Some(limit) => files.into_iter().skip(offset).take(limit).collect(),
        None => files.into_iter().skip(offset).collect(),
    };

    Ok(json!({ "directory": directory, "total": total, "files": files }))
}

/// 批量删除下载目录中的音频文件。
fn delete_library_files(runtime: &ServerRuntime, args: Value) -> Result<Value, String> {
    let paths: Vec<String> =
        serde_json::from_value(args.get("paths").cloned().unwrap_or_else(|| json!([])))
            .map_err(|error| format!("paths 解析失败: {error}"))?;
    let result =
        hotdownloader_core::library::delete_audio_files(library_directory(runtime), &paths)?;
    serde_json::to_value(result).map_err(|error| error.to_string())
}

/// 重命名下载目录中的音频文件。
fn rename_library_file(runtime: &ServerRuntime, args: Value) -> Result<Value, String> {
    let path = args
        .get("path")
        .and_then(|value| value.as_str())
        .ok_or("缺少 path")?;
    let new_name = args
        .get("newName")
        .and_then(|value| value.as_str())
        .ok_or("缺少 newName")?;
    let entry = hotdownloader_core::library::rename_audio_file(
        library_directory(runtime),
        path,
        new_name,
    )?;
    serde_json::to_value(entry).map_err(|error| error.to_string())
}

/// 写入/修改下载目录音频文件的标签。
fn write_library_metadata(runtime: &ServerRuntime, args: Value) -> Result<Value, String> {
    let path = args
        .get("path")
        .and_then(|value| value.as_str())
        .ok_or("缺少 path")?
        .to_string();
    // 未知字段（path 等）会被忽略，只解析标签字段。
    let update: hotdownloader_core::library::MetadataUpdate =
        serde_json::from_value(args).map_err(|error| format!("参数解析失败: {error}"))?;
    hotdownloader_core::library::write_audio_metadata(library_directory(runtime), &path, &update)?;
    Ok(json!({ "ok": true, "path": path }))
}

// ==================== 工具定义 ====================

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({ "name": name, "description": description, "inputSchema": input_schema })
}

fn platform_schema() -> Value {
    json!({ "type": "string", "enum": ["qqmusic", "kuwo"], "description": "音乐平台" })
}

fn paged_keyword_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "platform": platform_schema(),
            "keyword": { "type": "string", "description": "搜索关键词" },
            "page": { "type": "integer", "minimum": 1, "default": 1 },
            "limit": { "type": "integer", "minimum": 1, "maximum": 100, "default": 20 }
        },
        "required": ["platform", "keyword"]
    })
}

fn id_paged_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "platform": platform_schema(),
            "id": { "type": "string", "description": "专辑/歌手 ID" },
            "page": { "type": "integer", "minimum": 1, "default": 1 },
            "limit": { "type": "integer", "minimum": 1, "maximum": 100, "default": 20 }
        },
        "required": ["platform", "id"]
    })
}

fn task_id_schema() -> Value {
    json!({
        "type": "object",
        "properties": { "taskId": { "type": "string", "description": "任务 ID" } },
        "required": ["taskId"]
    })
}

fn tool_definitions() -> Vec<Value> {
    vec![
        tool("search_songs", "按关键词搜索歌曲（返回歌曲列表与是否有下一页）。", paged_keyword_schema()),
        tool("search_albums", "按关键词搜索专辑。", paged_keyword_schema()),
        tool("search_artists", "按关键词搜索歌手。", paged_keyword_schema()),
        tool("search_playlists", "按关键词搜索歌单。", paged_keyword_schema()),
        tool("fetch_album_songs", "获取指定专辑的歌曲列表。", id_paged_schema()),
        tool("fetch_artist_songs", "获取指定歌手的歌曲列表。", id_paged_schema()),
        tool("fetch_artist_albums", "获取指定歌手的专辑列表。", id_paged_schema()),
        tool(
            "fetch_playlist_songs",
            "通过歌单链接或 ID 获取歌单歌曲列表。",
            json!({
                "type": "object",
                "properties": {
                    "platform": platform_schema(),
                    "input": { "type": "string", "description": "歌单链接或 ID" }
                },
                "required": ["platform", "input"]
            }),
        ),
        tool(
            "fetch_hot_keywords",
            "获取指定平台的热搜关键词。",
            json!({
                "type": "object",
                "properties": { "platform": platform_schema() },
                "required": ["platform"]
            }),
        ),
        tool(
            "fetch_suggestions",
            "获取搜索建议。",
            json!({
                "type": "object",
                "properties": {
                    "platform": platform_schema(),
                    "keyword": { "type": "string", "description": "输入关键词" }
                },
                "required": ["platform", "keyword"]
            }),
        ),
        tool(
            "get_lyric_by_id",
            "根据歌曲数字 ID 获取歌词。",
            json!({
                "type": "object",
                "properties": {
                    "platform": platform_schema(),
                    "songId": { "type": "integer", "description": "歌曲数字 ID" }
                },
                "required": ["platform", "songId"]
            }),
        ),
        tool("list_tasks", "列出当前所有下载任务。", json!({ "type": "object", "properties": {} })),
        tool(
            "create_download_task",
            "创建下载任务。duplicateAction 为 ask 场景下可选择 overwrite/rename/cancel。",
            json!({
                "type": "object",
                "properties": {
                    "song": {
                        "type": "object",
                        "properties": {
                            "platform": platform_schema(),
                            "id": { "type": "integer" },
                            "mid": { "type": "string" },
                            "title": { "type": "string" },
                            "artist": { "type": "string" },
                            "album": { "type": "string" },
                            "coverUrl": { "type": "string" },
                            "mediaMid": { "type": "string" },
                            "qualities": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "quality": { "type": "string" },
                                        "filename": { "type": "string" },
                                        "size": { "type": "integer" }
                                    },
                                    "required": ["quality", "filename", "size"]
                                }
                            }
                        },
                        "required": ["platform", "id", "mid", "title", "artist", "album", "qualities"]
                    },
                    "desiredQuality": { "type": "string", "description": "期望音质" },
                    "duplicateAction": { "type": "string", "enum": ["overwrite", "rename", "cancel"] }
                },
                "required": ["song", "desiredQuality"]
            }),
        ),
        tool("pause_task", "暂停任务。", task_id_schema()),
        tool("resume_task", "恢复任务。", task_id_schema()),
        tool("retry_task", "重试任务（可能按设置自动降级音质）。", task_id_schema()),
        tool(
            "remove_task",
            "删除任务，可选同时删除已下载文件。",
            json!({
                "type": "object",
                "properties": {
                    "taskId": { "type": "string" },
                    "deleteFile": { "type": "boolean", "default": false }
                },
                "required": ["taskId"]
            }),
        ),
        tool(
            "remove_tasks",
            "批量删除任务，可选同时删除文件。",
            json!({
                "type": "object",
                "properties": {
                    "taskIds": { "type": "array", "items": { "type": "string" } },
                    "deleteFile": { "type": "boolean", "default": false }
                },
                "required": ["taskIds"]
            }),
        ),
        tool("get_settings", "获取当前设置快照。", json!({ "type": "object", "properties": {} })),
        tool(
            "patch_settings",
            "更新设置（字段级合并）。changes 为要修改的字段，expected 为这些字段的原值。",
            json!({
                "type": "object",
                "properties": {
                    "changes": { "type": "object" },
                    "expected": { "type": "object" }
                },
                "required": ["changes", "expected"]
            }),
        ),
        tool(
            "get_default_download_dir",
            "获取服务端默认下载目录。",
            json!({ "type": "object", "properties": {} }),
        ),
        tool("get_login_status", "获取 QQ 音乐登录状态。", json!({ "type": "object", "properties": {} })),
        tool(
            "list_library",
            "列出下载目录（曲库）中的音频文件，可按文件名关键词过滤并分页。",
            json!({
                "type": "object",
                "properties": {
                    "keyword": { "type": "string", "description": "按文件名过滤的关键词" },
                    "offset": { "type": "integer", "minimum": 0, "default": 0 },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 500 }
                }
            }),
        ),
        tool(
            "delete_library_files",
            "删除曲库（下载目录）中的音频文件，按 list_library 返回的路径批量删除。",
            json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "要删除的文件路径"
                    }
                },
                "required": ["paths"]
            }),
        ),
        tool(
            "rename_library_file",
            "重命名曲库中的音频文件（自动清洗非法字符，可省略扩展名以沿用原扩展名）。",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "原文件路径" },
                    "newName": { "type": "string", "description": "新的文件名（不含目录，可省略扩展名）" }
                },
                "required": ["path", "newName"]
            }),
        ),
        tool(
            "write_library_metadata",
            "写入或修改曲库音频文件的标签（标题/歌手/专辑/歌词）。仅传入的字段会被修改，空字符串表示清除该字段。",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "文件路径" },
                    "title": { "type": "string", "description": "标题" },
                    "artist": { "type": "string", "description": "歌手" },
                    "album": { "type": "string", "description": "专辑" },
                    "lyrics": { "type": "string", "description": "歌词文本" }
                },
                "required": ["path"]
            }),
        ),
    ]
}

// ==================== MCP 接入密钥 ====================

/// MCP 接入密钥（URL 中的 `?key=`）。持久化在数据目录，可在设置页查看与重置。
pub struct McpKey {
    path: PathBuf,
    key: RwLock<String>,
}

impl McpKey {
    pub fn load(data_dir: &Path) -> Result<Self, String> {
        let path = data_dir.join("mcp.json");
        let key = std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
            .and_then(|value| value.get("key").and_then(|key| key.as_str()).map(str::to_string))
            .filter(|key| !key.is_empty())
            .unwrap_or_else(generate_key);
        let instance = Self {
            path,
            key: RwLock::new(key),
        };
        instance.persist()?;
        Ok(instance)
    }

    pub fn value(&self) -> String {
        self.key.read().unwrap().clone()
    }

    /// 重新生成密钥并写回磁盘，返回新密钥。
    pub fn reset(&self) -> Result<String, String> {
        let key = generate_key();
        *self.key.write().unwrap() = key.clone();
        self.persist()?;
        Ok(key)
    }

    pub fn verify(&self, candidate: &str) -> bool {
        if candidate.is_empty() {
            return false;
        }
        let key = self.key.read().unwrap();
        constant_time_eq(&key, candidate)
    }

    fn persist(&self) -> Result<(), String> {
        let raw = serde_json::to_string_pretty(&json!({ "key": self.value() }))
            .map_err(|error| error.to_string())?;
        std::fs::write(&self.path, raw).map_err(|error| format!("写入 mcp.json 失败: {error}"))
    }
}

/// 生成 32 位十六进制密钥；优先使用系统随机源，非 Unix 环境回退到时间散列。
fn generate_key() -> String {
    use std::io::Read;

    let mut bytes = [0u8; 16];
    if std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .is_ok()
    {
        return bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    }

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let mut seed = nanos ^ (&bytes as *const _ as usize as u128);
    let mut key = String::with_capacity(32);
    for _ in 0..32 {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        key.push(std::char::from_digit(((seed >> 33) & 0xf) as u32, 16).unwrap());
    }
    key
}

fn constant_time_eq(expected: &str, supplied: &str) -> bool {
    let expected = expected.as_bytes();
    let supplied = supplied.as_bytes();
    let mut difference = expected.len() ^ supplied.len();
    for (a, b) in expected.iter().zip(supplied) {
        difference |= usize::from(a ^ b);
    }
    difference == 0
}
