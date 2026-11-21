//! 汽水 Cookie 凭据：独立文件保存，进程内缓存供所有接口请求复用。
//!
//! 匿名接口对未登录请求只返回试听甚至空响应，带上登录 Cookie 才能解锁搜索。
//! Cookie 只写入运行时数据目录，不进入设置快照、任务记录或任何 API 响应。

use std::path::PathBuf;
use std::sync::RwLock;

use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::json;

use crate::platforms::qqmusic::login::{FileLoginStore, LoginCredentialStore};

static CACHED: Lazy<RwLock<String>> = Lazy::new(|| RwLock::new(String::new()));

/// 运行时启动与保存后刷新进程内缓存，请求侧不必每次读文件。
pub fn set_cached_cookie(value: &str) {
    if let Ok(mut guard) = CACHED.write() {
        *guard = value.trim().to_string();
    }
}

/// 所有汽水请求共用的当前 Cookie；未配置时为空串。
pub fn cached_cookie() -> String {
    CACHED.read().map(|guard| guard.clone()).unwrap_or_default()
}

/// 只暴露是否配置、长度与键名，绝不回显值。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub configured: bool,
    pub length: usize,
    /// Cookie 的键名摘要，例如 `sessionid, sid_guard, uid_tt`。
    pub keys: String,
}

pub struct Store {
    file: FileLoginStore,
}

impl Store {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            file: FileLoginStore::new(path),
        }
    }

    fn load(&self) -> Result<serde_json::Value, String> {
        self.file.load().map_err(|error| error.replace("QQ", "汽水"))
    }

    fn save(&self, value: &serde_json::Value) -> Result<(), String> {
        self.file.save(value).map_err(|error| error.replace("QQ", "汽水"))
    }

    /// 读取已保存的 Cookie；文件被手工改坏时返回可读错误。
    pub fn cookie(&self) -> Result<String, String> {
        let value = self.load()?;
        let cookie = value["cookie"].as_str().unwrap_or("").trim();
        if cookie.is_empty() {
            return Ok(String::new());
        }
        normalize(cookie)
    }

    pub fn status(&self) -> Result<Status, String> {
        let cookie = self.cookie()?;
        Ok(Status {
            configured: !cookie.is_empty(),
            length: cookie.len(),
            keys: cookie_keys(&cookie),
        })
    }

    /// 保存并立即刷新进程内缓存，保存后无需重启即可生效。
    pub fn save_cookie(&self, cookie: &str) -> Result<Status, String> {
        let cookie = normalize(cookie)?;
        self.save(&json!({
            "cookie": cookie,
            "updatedAt": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_millis() as u64)
                .unwrap_or(0),
        }))?;
        set_cached_cookie(&cookie);
        self.status()
    }

    /// 清除后回到匿名请求，缓存同步置空。
    pub fn clear(&self) -> Result<(), String> {
        self.save(&json!({}))?;
        set_cached_cookie("");
        Ok(())
    }

    /// 启动时把磁盘上的 Cookie 载入缓存；文件异常不影响应用启动。
    pub fn preload(&self) {
        match self.cookie() {
            Ok(cookie) => set_cached_cookie(&cookie),
            Err(error) => {
                log::warn!("汽水 Cookie 读取失败，按未配置处理: {error}");
                set_cached_cookie("");
            }
        }
    }
}

/// 允许整行粘贴：剥掉 `Cookie:` 前缀，校验字符集与基本格式。
fn normalize(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    let cookie = trimmed
        .strip_prefix("Cookie:")
        .or_else(|| trimmed.strip_prefix("cookie:"))
        .unwrap_or(trimmed)
        .trim();
    if cookie.is_empty() {
        return Err("请输入汽水 Cookie".into());
    }
    if cookie.len() > 16_384 {
        return Err("Cookie 过长，请确认粘贴内容是否完整".into());
    }
    if cookie.bytes().any(|byte| !(32..127).contains(&byte)) {
        return Err("Cookie 含非法字符（换行或中文），请只粘贴单行 Cookie".into());
    }
    if !cookie.contains('=') {
        return Err("Cookie 格式不正确，应形如 sessionid=xxx; uid_tt=yyy".into());
    }
    Ok(cookie.to_string())
}

fn cookie_keys(cookie: &str) -> String {
    let keys: Vec<&str> = cookie
        .split(';')
        .filter_map(|pair| pair.split_once('=').map(|(key, _)| key.trim()))
        .filter(|key| !key.is_empty())
        .take(8)
        .collect();
    keys.join(", ")
}

#[cfg(test)]
mod tests {
    use super::{cookie_keys, normalize, set_cached_cookie, Store};

    #[test]
    fn cookie_input_accepts_full_header_and_rejects_noise() {
        assert_eq!(
            normalize("Cookie: sessionid=abc; uid_tt=def").unwrap(),
            "sessionid=abc; uid_tt=def"
        );
        assert!(normalize("sessionid=abc").is_ok());
        assert!(normalize("").is_err());
        assert!(normalize("多行\nCookie=1").is_err());
        assert!(normalize("no-equals-sign").is_err());
    }

    #[test]
    fn status_only_exposes_key_names() {
        let keys = cookie_keys("sessionid=secret; uid_tt=secret2; sid_guard=secret3");
        assert_eq!(keys, "sessionid, uid_tt, sid_guard");
        assert!(!keys.contains("secret"));
    }

    #[test]
    fn saving_and_clearing_updates_the_process_cache() {
        let path = std::env::temp_dir().join(format!(
            "hotdownloader-qishui-cookie-{:x}.json",
            rand::random::<u64>()
        ));
        let store = Store::new(&path);
        let status = store.save_cookie("sessionid=abc; uid_tt=def").unwrap();
        assert!(status.configured);
        assert_eq!(super::cached_cookie(), "sessionid=abc; uid_tt=def");

        store.clear().unwrap();
        assert_eq!(super::cached_cookie(), "");
        set_cached_cookie("");
        let _ = std::fs::remove_file(path);
    }
}
