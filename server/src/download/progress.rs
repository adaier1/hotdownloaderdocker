use crate::ctx::AppCtx;
use crate::events;

pub fn emit_progress(
    app_handle: &AppCtx,
    task_id: &str,
    downloaded: u64,
    total: u64,
    speed: u64,
) {
    let payload = events::DownloadProgressPayload {
        task_id: task_id.to_string(),
        downloaded,
        total,
        speed,
    };
    let _ = app_handle.emit(events::DOWNLOAD_PROGRESS, payload);
}

pub fn emit_completed(
    app_handle: &AppCtx,
    task_id: &str,
    final_path: &str,
    saf_folder_uri: Option<String>,
) {
    let payload = events::DownloadCompletedPayload {
        task_id: task_id.to_string(),
        final_path: final_path.to_string(),
        saf_folder_uri,
    };
    let _ = app_handle.emit(events::DOWNLOAD_COMPLETED, payload);
}

pub fn emit_error(app_handle: &AppCtx, task_id: &str, error_msg: &str) {
    let payload = events::DownloadErrorPayload {
        task_id: task_id.to_string(),
        error_msg: error_msg.to_string(),
    };
    let _ = app_handle.emit(events::DOWNLOAD_ERROR, payload);
}

pub fn emit_link_expired(app_handle: &AppCtx, task_id: &str, current_offset: u64) {
    let payload = events::DownloadLinkExpiredPayload {
        task_id: task_id.to_string(),
        current_offset,
    };
    let _ = app_handle.emit(events::DOWNLOAD_LINK_EXPIRED, payload);
}

/// 发送文件下载完成、进入处理中状态的事件
/// 前端收到后将任务状态置为 processing，进度自动显示 100%
pub fn emit_file_complete(app_handle: &AppCtx, task_id: &str) {
    let payload = events::DownloadFileCompletePayload {
        task_id: task_id.to_string(),
    };
    let _ = app_handle.emit(events::DOWNLOAD_FILE_COMPLETE, payload);
}

/// 发送元数据写入失败事件，前端弹出警告提示
pub fn emit_metadata_error(app_handle: &AppCtx, task_id: &str, error_msg: &str) {
    let payload = events::DownloadMetadataErrorPayload {
        task_id: task_id.to_string(),
        error_msg: error_msg.to_string(),
    };
    let _ = app_handle.emit(events::DOWNLOAD_METADATA_ERROR, payload);
}
