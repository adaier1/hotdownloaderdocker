use std::fs;
use std::path::Path;
#[cfg(not(target_os = "android"))]
use std::process::Command;
use tauri::command;
use tauri::AppHandle;

#[cfg(target_os = "linux")]
use open;
#[cfg(target_os = "android")]
use tauri_plugin_android_fs::{AndroidFsExt, FsUri};

/// 获取系统默认下载目录路径（内部实现）
pub(crate) fn get_default_download_dir_impl(_app: &AppHandle) -> String {
    #[cfg(target_os = "android")]
    {
        // 尝试获取外部存储根目录
        if let Ok(ext) = std::env::var("EXTERNAL_STORAGE") {
            let path = Path::new(&ext).join("Download");
            return path.to_string_lossy().to_string();
        }
        // 回退到常见路径（适用于大多数 Android 设备）
        "/storage/emulated/0/Download".to_string()
    }

    #[cfg(not(target_os = "android"))]
    {
        dirs::download_dir()
            .unwrap_or_else(|| Path::new(".").to_path_buf())
            .to_string_lossy()
            .to_string()
    }
}

/// 获取系统默认下载目录路径（命令）
#[command]
pub fn get_default_download_dir(app: AppHandle) -> String {
    get_default_download_dir_impl(&app)
}

/// 创建目录（若不存在）
#[command]
pub fn create_directory(path: String) -> Result<(), String> {
    fs::create_dir_all(&path).map_err(|e| e.to_string())
}

/// 打开文件所在目录并选中文件（Android 上打开文件本身）
#[command]
pub fn open_file_location(app: AppHandle, path: String) -> Result<(), String> {
    #[cfg(not(target_os = "android"))]
    let _ = &app; // 消除非 Android 平台未使用警告

    let file_path = Path::new(&path);
    if !file_path.exists() {
        return Err("文件不存在".to_string());
    }

    #[cfg(target_os = "android")]
    {
        use tauri_plugin_opener::OpenerExt;
        log::info!("Android 打开文件：{}", path);
        app.opener()
            .open_path(&path, None::<&str>)
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .args(["/select,", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        // 获取父目录
        let parent = file_path.parent().ok_or("无法获取父目录")?;

        // 尝试用 xdg-open 打开父目录（直接传 Path）
        if Command::new("xdg-open").arg(parent).spawn().is_ok() {
            return Ok(());
        }

        // 备选：使用 dbus-send 选中文件（部分文件管理器支持）
        if Command::new("dbus-send")
            .args([
                "--session",
                "--print-reply",
                "--dest=org.freedesktop.FileManager1",
                "/org/freedesktop/FileManager1",
                "org.freedesktop.FileManager1.ShowItems",
                &format!("array:string:{}", path), // path 是 String 类型
            ])
            .spawn()
            .is_ok()
        {
            return Ok(());
        }

        // 最后回退：用 open 打开父目录（不选中文件）
        open::that(parent).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// 选择 SAF 文件夹（仅 Android）
#[command]
pub fn pick_saf_folder(app: AppHandle) -> Result<String, String> {
    #[cfg(target_os = "android")]
    {
        let api = app.android_fs();
        let picker = api.picker();

        // 选择目录
        let uri_opt = picker
            .pick_dir(None, true) // 第二个参数 local_only = true
            .map_err(|e| e.to_string())?;

        if let Some(uri) = uri_opt {
            // 持久化权限，使应用重启后仍可访问该目录
            picker
                .persist_uri_permission(&uri)
                .map_err(|e| format!("持久化权限失败: {}", e))?;

            // 返回完整 FsUri JSON（包含 document_top_tree_uri）
            uri.to_json_string().map_err(|e| e.to_string())
        } else {
            // 用户取消选择
            Ok(String::new())
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = &app; // 消除非 Android 平台未使用警告
        Err("当前平台不支持 SAF".to_string())
    }
}

/// 删除 SAF 文件（仅 Android）
#[command]
pub fn delete_saf_file(app: AppHandle, uri: String) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        let api = app.android_fs();
        let fs_uri = FsUri::from_uri(uri);
        api.remove_file(&fs_uri).map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (&app, &uri); // 消除非 Android 平台未使用警告
        Err("当前平台不支持 SAF".to_string())
    }
}
