use std::fs::{self, OpenOptions};
use std::io::BufWriter;

use super::progress;
use crate::ctx::AppCtx;

/// 文件写入缓冲区容量（64 KB）。
///
/// 用于 `BufWriter`，在写入磁盘前暂存数据，减少频繁的 I/O 操作。
const FILE_BUFFER_CAPACITY: usize = 64 * 1024;

/// 打开或创建下载目标文件，并返回带缓冲的写入器。
///
/// 根据 `downloaded` 的值决定是全新下载（截断模式）还是续传（追加模式）。
/// 在续传时会检查文件大小是否匹配，若不匹配则重置文件并从头下载。
///
/// # 参数
/// - `ctx`: 应用上下文，用于发送错误事件。
/// - `task_id`: 下载任务唯一标识，用于错误事件。
/// - `download_dir`: 完整的本地文件路径。
/// - `downloaded`: 可变引用，已下载字节数。文件异常时可能被重置为 0。
///
/// # 返回
/// - `Some(BufWriter<fs::File>)`：成功打开文件。
/// - `None`：文件创建/打开失败，或续传时文件异常且重置失败（已发送错误事件）。
pub(crate) async fn open_download_file(
    ctx: &AppCtx,
    task_id: &str,
    download_dir: &str,
    downloaded: &mut u64,
) -> Option<BufWriter<fs::File>> {
    if *downloaded == 0 {
        // 全新下载：以写入、创建、截断模式打开
        match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(download_dir)
        {
            Ok(f) => Some(BufWriter::with_capacity(FILE_BUFFER_CAPACITY, f)),
            Err(e) => {
                log::error!("文件创建失败: {}", e);
                progress::emit_error(ctx, task_id, "文件创建失败，请检查磁盘空间");
                None
            }
        }
    } else {
        // 续传任务：先以追加模式打开
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(download_dir)
        {
            Ok(f) => {
                // 校验文件大小：如果文件长度小于期望的偏移，说明文件异常，重置下载
                if let Ok(meta) = f.metadata() {
                    if meta.len() < *downloaded {
                        // 文件被截断或损坏，清空文件并从头下载
                        drop(f); // 先关闭文件，避免占用
                        match OpenOptions::new()
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .open(download_dir)
                        {
                            Ok(new_f) => {
                                *downloaded = 0;
                                Some(BufWriter::with_capacity(FILE_BUFFER_CAPACITY, new_f))
                            }
                            Err(e) => {
                                log::error!("文件重置失败: {}", e);
                                progress::emit_error(ctx, task_id, "文件异常，请重试");
                                None
                            }
                        }
                    } else {
                        // 文件大小正常，直接使用追加模式打开的文件
                        Some(BufWriter::with_capacity(FILE_BUFFER_CAPACITY, f))
                    }
                } else {
                    // 无法获取元数据，保守起见改为从头下载
                    drop(f);
                    match OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(download_dir)
                    {
                        Ok(new_f) => {
                            *downloaded = 0;
                            Some(BufWriter::with_capacity(FILE_BUFFER_CAPACITY, new_f))
                        }
                        Err(e) => {
                            log::error!("文件重置失败: {}", e);
                            progress::emit_error(ctx, task_id, "文件异常，请重试");
                            None
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("文件打开失败: {}", e);
                progress::emit_error(ctx, task_id, "文件访问失败");
                None
            }
        }
    }
}
