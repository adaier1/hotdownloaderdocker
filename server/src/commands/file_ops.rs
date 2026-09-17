//! 文件相关命令。
//!
//! Web 版运行在服务端容器中，下载目录由 `DOWNLOAD_DIR` 决定，
//! 因此不再提供“打开文件位置 / 选择 SAF 目录”等本地能力。

use crate::ctx::AppCtx;

/// 获取默认下载目录（服务端挂载的下载卷）。
pub(crate) fn get_default_download_dir_impl(ctx: &AppCtx) -> String {
    ctx.download_dir().to_string_lossy().to_string()
}

/// 命令：获取默认下载目录。
pub fn get_default_download_dir(ctx: &AppCtx) -> String {
    get_default_download_dir_impl(ctx)
}
