//! 可由 Tauri 客户端及独立 Rust 服务复用的任务业务核心。
//! 原生窗口、Android SAF、系统通知和 IPC 适配保留在各自的运行时项目中。

pub mod adapters;
pub mod download;
/// 本地新增：曲库文件管理（MCP 的 list/delete/rename/write_metadata 依赖此模块）。
pub mod library;
pub mod platforms;
pub mod playback;
pub mod settings;
pub mod task;
