//! 任务相关命令：任务持久化、下载任务注册/调度、并发数、路径冲突检查。

use std::path::Path;
use std::sync::Arc;

use serde_json::json;

use crate::ctx::AppCtx;
use crate::download::engine::DownloadEngine;
use crate::platforms::Platform;
use crate::storage::store_wrapper;

pub async fn load_tasks(ctx: &AppCtx) -> Result<String, String> {
    store_wrapper::load_string(ctx, "tasks").map_err(|e| e.to_string())
}

pub async fn save_tasks(ctx: &AppCtx, tasks_json: String) -> Result<(), String> {
    store_wrapper::save_string(ctx, "tasks", &tasks_json).map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
pub async fn add_download_task(
    _ctx: &AppCtx,
    engine: &Arc<DownloadEngine>,
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
) -> Result<(), String> {
    let p = Platform::from_str(&platform)?;
    engine
        .add_task(
            task_id, p, song_id, song_mid, url, save_path, quality, filename, key, file_size,
            song_title, artist, album, cover_url,
        )
        .await;
    Ok(())
}

pub async fn enqueue_task(
    _ctx: &AppCtx,
    engine: &Arc<DownloadEngine>,
    task_id: String,
    offset: u64,
) -> Result<(), String> {
    // 上下文缺失时返回 ERR_TASK_CONTEXT_MISSING，前端会据此重新注册任务
    engine.enqueue_task(&task_id, offset).await
}

pub async fn pause_task(
    _ctx: &AppCtx,
    engine: &Arc<DownloadEngine>,
    task_id: String,
) -> Result<(), String> {
    engine.pause(&task_id).await;
    Ok(())
}

pub async fn resume_task(
    _ctx: &AppCtx,
    engine: &Arc<DownloadEngine>,
    task_id: String,
) -> Result<(), String> {
    engine.resume(&task_id).await;
    Ok(())
}

pub async fn cancel_task(
    _ctx: &AppCtx,
    engine: &Arc<DownloadEngine>,
    task_id: String,
    delete_file: bool,
) -> Result<(), String> {
    engine.cancel(&task_id, delete_file).await;
    Ok(())
}

pub async fn remove_task(
    _ctx: &AppCtx,
    engine: &Arc<DownloadEngine>,
    task_id: String,
    delete_file: bool,
) -> Result<(), String> {
    engine.remove(&task_id, delete_file).await
}

/// 批量移除任务。返回后端真实的成功/失败数量，供前端展示准确通知。
pub async fn remove_tasks(
    _ctx: &AppCtx,
    engine: &Arc<DownloadEngine>,
    task_ids: Vec<String>,
    delete_file: bool,
) -> Result<String, String> {
    let mut errors: Vec<String> = Vec::new();
    let mut succeeded: usize = 0;
    let mut failed: usize = 0;
    for task_id in task_ids {
        if let Err(e) = engine.remove(&task_id, delete_file).await {
            log::error!("批量移除任务失败 {}: {}", task_id, e);
            errors.push(format!("{}: {}", task_id, e));
            failed += 1;
        } else {
            succeeded += 1;
        }
    }
    let result = json!({
        "succeeded": succeeded,
        "failed": failed,
        "errors": errors,
    });
    Ok(result.to_string())
}

pub fn set_max_concurrent(
    _ctx: &AppCtx,
    engine: &Arc<DownloadEngine>,
    max: u32,
) -> Result<(), String> {
    engine.set_concurrency(max);
    Ok(())
}

/// 检查目标下载路径是否已存在，并给出建议的重命名路径。
#[allow(clippy::too_many_arguments)]
pub async fn check_download_path(
    ctx: &AppCtx,
    _song_id: u64,
    _song_mid: String,
    song_title: String,
    artist: String,
    album: String,
    cover_url: String,
    quality_filename: String,
    quality: String,
) -> Result<String, String> {
    // 构建 SongInfo 对象，quality 必须传入，否则命名模板中 {quality} 会出错
    let song_info = crate::download::task::SongInfo {
        title: song_title,
        artist,
        album,
        quality,
        cover_url,
    };

    let (dir_setting, template_setting, saf_uri_setting, _, _) =
        crate::download::task_path::get_download_settings(ctx).await;
    let (is_saf, download_dir, _saf_folder_uri) =
        crate::download::task_path::resolve_download_path(
            &dir_setting,
            &template_setting,
            saf_uri_setting.as_deref(),
            &song_info,
            &quality_filename,
        );

    // 服务端不使用 SAF，这里保留分支以防未来扩展
    let exists = if is_saf {
        false
    } else {
        Path::new(&download_dir).exists()
    };

    if exists {
        let path = Path::new(&download_dir);
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed");
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let mut counter = 1;
        loop {
            let new_name = if ext.is_empty() {
                format!("{} ({})", stem, counter)
            } else {
                format!("{} ({}).{}", stem, counter, ext)
            };
            let new_path = path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(&new_name)
                .to_string_lossy()
                .to_string();
            if !Path::new(&new_path).exists() {
                return Ok(json!({
                    "original_path": download_dir,
                    "exists": exists,
                    "suggested_path": new_path,
                    "is_saf": is_saf,
                })
                .to_string());
            }
            counter += 1;
        }
    }

    Ok(json!({
        "original_path": download_dir,
        "exists": exists,
        "suggested_path": download_dir,
        "is_saf": is_saf,
    })
    .to_string())
}
