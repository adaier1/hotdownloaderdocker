//! 汽水签名器：为 `x-helios` / `x-medusa` 提供两种后端。
//!
//! 整曲取流（`POST /luna/pc/track_v2`）与搜索都要求逐请求的应用级签名，缺口令时
//! 服务端返回 `HTTP 200 + 空 body`。签名只能由官方原生组件 `bdms.node`
//! （mssdk / metasecml.dll）生成，没有可复刻的纯软件实现，所以支持两条路：
//!
//! - **本机（默认）**：进程内用 JS 运行时调 `bdms.node`，适合桌面端；
//! - **远程**：设置 `QISHUI_SIGNER_URL` 指向一台能跑 `bdms.node` 的签名服务，
//!   适合 Docker / 服务器——容器里没有 Windows 原生组件，只能借宿主机的。
//!
//! 远程模式下签名服务会看到完整请求头（含 Cookie），只应指向自己信任的实例。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::RwLock;
use std::time::Duration;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// 桥接脚本：stdin 收一次请求、stdout 回签名头。
const BRIDGE_JS: &str = include_str!("assets/qishui-bridge.cjs");
const SIGN_TIMEOUT: Duration = Duration::from_secs(20);
const NODE_ENV: &str = "QISHUI_NODE";
const CLIENT_DIR_ENV: &str = "QISHUI_CLIENT_DIR";
const BDMS_ENV: &str = "QISHUI_BDMS_PATH";
const DEVICE_ENV: &str = "QISHUI_DEVICE_ID";
const IDENTITY_ENV: &str = "QISHUI_IDENTITY_FILE";
/// 远程签名服务地址，例如 `http://host.docker.internal:8899/sign`。
const SIGNER_URL_ENV: &str = "QISHUI_SIGNER_URL";
/// 远程签名服务的 Bearer token（可选）。
const SIGNER_TOKEN_ENV: &str = "QISHUI_SIGNER_TOKEN";
/// 远程签名超时（毫秒），公网部署时按链路延迟调大；默认 8 秒。
const SIGNER_TIMEOUT_ENV: &str = "QISHUI_SIGNER_TIMEOUT_MS";

/// 与签名器同源的设备身份；`fp` 缺省时用 `device_id`。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub device_id: String,
    pub iid: String,
    pub fp: String,
}

impl Identity {
    pub(crate) fn fp_or_device(&self) -> &str {
        if self.fp.trim().is_empty() {
            &self.device_id
        } else {
            &self.fp
        }
    }
}

/// 一次签名的结果：远程服务可能要求把 `msToken` / `a_bogus` 拼进 URL。
#[derive(Clone, Debug)]
pub struct Signature {
    pub url: String,
    pub headers: HashMap<String, String>,
}

/// 对外只暴露「能不能用」与路径摘要，不返回任何凭据。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub available: bool,
    /// `local` / `remote` / 空字符串。
    pub mode: String,
    pub bdms: String,
    pub runtime: String,
    pub signer_url: String,
    pub electron_mode: bool,
    pub device_id: String,
}

struct LocalRuntime {
    program: PathBuf,
    electron_mode: bool,
    bdms: PathBuf,
}

struct RemoteSigner {
    url: String,
    token: String,
}

enum Backend {
    Local(LocalRuntime),
    Remote(RemoteSigner),
}

static BACKEND: Lazy<Option<Backend>> = Lazy::new(probe_backend);
static IDENTITY: Lazy<RwLock<Option<Identity>>> = Lazy::new(|| RwLock::new(None));

/// 是否具备签名能力（本机 bdms.node 或远程签名服务）。
pub fn available() -> bool {
    BACKEND.is_some()
}

/// 当前签名器状态，供设置页/诊断使用；设备身份只回缓存值，不会触发远程请求。
pub fn status() -> Status {
    let mut status = Status {
        available: false,
        mode: String::new(),
        bdms: String::new(),
        runtime: String::new(),
        signer_url: String::new(),
        electron_mode: false,
        device_id: String::new(),
    };
    match BACKEND.as_ref() {
        Some(Backend::Local(runtime)) => {
            status.available = true;
            status.mode = "local".into();
            status.bdms = runtime.bdms.display().to_string();
            status.runtime = runtime.program.display().to_string();
            status.electron_mode = runtime.electron_mode;
        }
        Some(Backend::Remote(remote)) => {
            status.available = true;
            status.mode = "remote".into();
            status.signer_url = remote.url.clone();
        }
        None => return status,
    }
    status.device_id = cached_identity()
        .map(|identity| mask(&identity.device_id))
        .unwrap_or_default();
    status
}

/// 已缓存的设备身份；没有就返回 `None`（不发起网络请求）。
pub fn cached_identity() -> Option<Identity> {
    IDENTITY.read().ok().and_then(|guard| guard.clone())
}

/// 读缓存或首次取设备身份；远程模式向签名服务要，本机模式本地生成后落盘。
pub async fn identity() -> Result<Identity, String> {
    if let Some(cached) = cached_identity() {
        return Ok(cached);
    }
    let identity = match BACKEND.as_ref() {
        Some(Backend::Remote(remote)) => remote_identity(remote).await?,
        _ => load_or_create_identity()?,
    };
    if let Ok(mut guard) = IDENTITY.write() {
        *guard = Some(identity.clone());
    }
    Ok(identity)
}

/// 给一条请求签名。`headers` 会一并参与签名（含 Cookie），必须与实际请求一致。
pub async fn sign(
    method: &str,
    url: &str,
    body: &str,
    headers: &HashMap<String, String>,
) -> Result<Signature, String> {
    match BACKEND.as_ref() {
        Some(Backend::Local(runtime)) => local_sign(runtime, method, url, body, headers).await,
        Some(Backend::Remote(remote)) => remote_sign(remote, method, url, body, headers).await,
        None => Err("未找到汽水签名器（bdms.node、JS 运行时或 QISHUI_SIGNER_URL）".into()),
    }
}

fn probe_backend() -> Option<Backend> {
    if let Some(remote) = remote_from_env() {
        return Some(Backend::Remote(remote));
    }
    probe_local().map(Backend::Local)
}

fn remote_from_env() -> Option<RemoteSigner> {
    let url = std::env::var(SIGNER_URL_ENV).ok()?.trim().to_string();
    if url.is_empty() {
        return None;
    }
    Some(RemoteSigner {
        url,
        token: std::env::var(SIGNER_TOKEN_ENV)
            .unwrap_or_default()
            .trim()
            .to_string(),
    })
}

fn signer_timeout() -> Duration {
    let millis = std::env::var(SIGNER_TIMEOUT_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(8_000)
        .clamp(1_000, 60_000);
    Duration::from_millis(millis)
}

fn signer_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(signer_timeout())
        .build()
        .map_err(|error| format!("创建签名客户端失败: {error}"))
}

fn config_url(signer_url: &str) -> String {
    let trimmed = signer_url.trim().trim_end_matches('/');
    let base = trimmed.strip_suffix("/sign").unwrap_or(trimmed);
    format!("{}/config", base.trim_end_matches('/'))
}

/// 向签名服务要设备身份；这是后续请求参数与签名必须共用的那套。
async fn remote_identity(remote: &RemoteSigner) -> Result<Identity, String> {
    if let Ok(device) = std::env::var(DEVICE_ENV) {
        let device = device.trim().to_string();
        if !device.is_empty() {
            return Ok(Identity {
                iid: format!("{device}1"),
                fp: device.clone(),
                device_id: device,
            });
        }
    }
    let url = config_url(&remote.url);
    let mut request = signer_client()?.get(&url).header("Accept", "application/json");
    if !remote.token.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", remote.token));
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("请求签名服务 {url} 失败: {}", error.without_url()))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| format!("读取签名服务响应失败: {}", error.without_url()))?;
    if !status.is_success() {
        return Err(format!("签名服务 /config 返回 {status}"));
    }
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("签名服务 /config 无法解析: {error}"))?;
    let device_id = value["identity"]["device_id"]
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .or_else(|| value["device_id"].as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if device_id.is_empty() {
        return Err("签名服务 /config 未返回 device_id".into());
    }
    Ok(Identity {
        iid: value["identity"]["iid"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string(),
        fp: value["identity"]["fp"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string(),
        device_id,
    })
}

async fn local_sign(
    runtime: &LocalRuntime,
    method: &str,
    url: &str,
    body: &str,
    headers: &HashMap<String, String>,
) -> Result<Signature, String> {
    let identity = identity().await?;
    let payload = json!({
        "bdms": runtime.bdms.display().to_string(),
        "device_id": identity.device_id,
        "method": method,
        "url": url,
        "body": body,
        "headers": headers,
    });

    let mut command = Command::new(&runtime.program);
    command
        .arg("-e")
        .arg(BRIDGE_JS)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if runtime.electron_mode {
        command.env("ELECTRON_RUN_AS_NODE", "1");
    }

    let mut child = command
        .spawn()
        .map_err(|error| format!("启动签名进程失败: {error}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(payload.to_string().as_bytes())
            .await
            .map_err(|error| format!("写入签名请求失败: {error}"))?;
        drop(stdin);
    }

    let output = tokio::time::timeout(SIGN_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| "签名超时（20 秒）".to_string())?
        .map_err(|error| format!("签名进程异常: {error}"))?;
    if !output.status.success() {
        let reason = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if reason.is_empty() {
            "签名进程退出但未给出原因".into()
        } else {
            format!("签名失败: {reason}")
        });
    }
    let headers = parse_signature(&String::from_utf8_lossy(&output.stdout))?;
    Ok(Signature {
        url: url.to_string(),
        headers,
    })
}

async fn remote_sign(
    remote: &RemoteSigner,
    method: &str,
    url: &str,
    body: &str,
    headers: &HashMap<String, String>,
) -> Result<Signature, String> {
    let payload = json!({
        "url": url,
        "method": method.to_uppercase(),
        "body": body,
        "ts_ms": now_millis(),
        "headers": headers,
    });
    let mut request = signer_client()?
        .post(&remote.url)
        .header("Content-Type", "application/json; charset=utf-8")
        .header("Accept", "application/json")
        .body(payload.to_string());
    if !remote.token.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", remote.token));
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("请求签名服务失败: {}", error.without_url()))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| format!("读取签名响应失败: {}", error.without_url()))?;
    if !status.is_success() {
        return Err(format!("签名服务返回 {status}: {}", text.trim()));
    }
    parse_remote_signature(url, &text)
}

/// 解析远程签名回包：`{"ok":true,"headers":{...},"ms_token":"…","a_bogus":"…"}`，
/// 同时兼容 Meting 风格的扁平 `{"ok":true,"X-Helios":"…","X-Medusa":"…"}`。
fn parse_remote_signature(url: &str, raw: &str) -> Result<Signature, String> {
    let value: serde_json::Value =
        serde_json::from_str(raw.trim()).map_err(|error| format!("签名响应无法解析: {error}"))?;
    if value["ok"].as_bool() == Some(false) {
        let reason = value["error"].as_str().unwrap_or("未知错误");
        return Err(format!("签名服务拒绝: {reason}"));
    }

    let mut headers = HashMap::new();
    if let Some(map) = value["headers"].as_object() {
        for (key, item) in map {
            if let Some(text) = item.as_str().filter(|text| !text.trim().is_empty()) {
                headers.insert(key.clone(), text.to_string());
            }
        }
    }
    for (alias, canonical) in [
        ("X-Helios", "x-helios"),
        ("x-helios", "x-helios"),
        ("X-Medusa", "x-medusa"),
        ("x-medusa", "x-medusa"),
    ] {
        if let Some(text) = value[alias].as_str().filter(|text| !text.trim().is_empty()) {
            headers.insert(canonical.to_string(), text.to_string());
        }
    }
    let has_helios = headers
        .keys()
        .any(|key| key.eq_ignore_ascii_case("x-helios"));
    let has_medusa = headers
        .keys()
        .any(|key| key.eq_ignore_ascii_case("x-medusa"));
    if !has_helios || !has_medusa {
        return Err("签名结果缺少 x-helios / x-medusa".into());
    }

    let mut signed_url = url.to_string();
    if let Ok(mut parsed) = reqwest::Url::parse(url) {
        let mut changed = false;
        for (key, field) in [("msToken", "ms_token"), ("a_bogus", "a_bogus")] {
            if let Some(value) = value[field].as_str().filter(|text| !text.trim().is_empty()) {
                parsed.query_pairs_mut().append_pair(key, value);
                changed = true;
            }
        }
        if changed {
            signed_url = parsed.to_string();
        }
    }
    Ok(Signature {
        url: signed_url,
        headers,
    })
}

/// 解析本机桥接进程的输出：`{"headers":{"X-Helios":"…","X-Medusa":"…"}}`。
fn parse_signature(raw: &str) -> Result<HashMap<String, String>, String> {
    let value: serde_json::Value =
        serde_json::from_str(raw.trim()).map_err(|error| format!("签名结果无法解析: {error}"))?;
    let mut headers = HashMap::new();
    if let Some(map) = value["headers"].as_object() {
        for (key, item) in map {
            if let Some(text) = item.as_str().filter(|text| !text.trim().is_empty()) {
                headers.insert(key.clone(), text.to_string());
            }
        }
    }
    let has_helios = headers
        .keys()
        .any(|key| key.eq_ignore_ascii_case("x-helios"));
    let has_medusa = headers
        .keys()
        .any(|key| key.eq_ignore_ascii_case("x-medusa"));
    if !has_helios || !has_medusa {
        return Err("签名结果缺少 x-helios / x-medusa".into());
    }
    Ok(headers)
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn discover_bdms() -> Option<PathBuf> {
    if let Some(value) = std::env::var_os(BDMS_ENV) {
        let path = PathBuf::from(value);
        if path.exists() {
            return Some(path);
        }
    }
    if let Some(value) = std::env::var_os(CLIENT_DIR_ENV) {
        let root = PathBuf::from(value);
        for candidate in [
            root.join("resources")
                .join("app.asar.unpacked")
                .join("bdms.node"),
            root.join("app.asar.unpacked").join("bdms.node"),
        ] {
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    default_client_roots()
        .into_iter()
        .find_map(|root| latest_bdms_under(&root))
}

/// 客户端安装位置：Windows 有 `版本号/` 与 `Packages/版本号/` 两种布局。
fn default_client_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        roots.push(PathBuf::from(local).join("Programs").join("Soda Music"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        roots.push(
            home.join("Applications")
                .join("SodaMusic.app")
                .join("Contents"),
        );
        roots.push(PathBuf::from("/Applications/SodaMusic.app/Contents"));
        roots.push(PathBuf::from("/opt/SodaMusic"));
    }
    roots
}

/// 一个安装根目录下有多个版本时，取字典序最大的版本（3.8.0 > 3.7.0）。
fn latest_bdms_under(root: &Path) -> Option<PathBuf> {
    let mut found: Vec<(String, PathBuf)> = Vec::new();
    let mut levels = vec![root.to_path_buf()];
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                levels.push(path.join("Packages"));
                levels.push(path);
            }
        }
    }
    for level in levels {
        let Ok(entries) = std::fs::read_dir(&level) else {
            continue;
        };
        for entry in entries.flatten() {
            let version_dir = entry.path();
            if !version_dir.is_dir() {
                continue;
            }
            for candidate in [
                version_dir
                    .join("resources")
                    .join("app.asar.unpacked")
                    .join("bdms.node"),
                version_dir.join("app.asar.unpacked").join("bdms.node"),
            ] {
                if candidate.exists() {
                    let name = version_dir
                        .file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or("")
                        .to_string();
                    found.push((name, candidate));
                }
            }
        }
    }
    found.sort_by(|left, right| left.0.cmp(&right.0));
    found.pop().map(|(_, path)| path)
}

fn probe_local() -> Option<LocalRuntime> {
    let bdms = discover_bdms()?;
    if let Some(value) = std::env::var_os(NODE_ENV) {
        let program = PathBuf::from(value);
        if program.exists() {
            return Some(LocalRuntime {
                program,
                electron_mode: false,
                bdms,
            });
        }
    }
    if let Some(program) = which_node() {
        return Some(LocalRuntime {
            program,
            electron_mode: false,
            bdms,
        });
    }
    // 没有系统 Node 时借用客户端自带的 Electron，只要装了客户端就能签名。
    let electron = find_client_exe(&bdms)?;
    Some(LocalRuntime {
        program: electron,
        electron_mode: true,
        bdms,
    })
}

fn which_node() -> Option<PathBuf> {
    let name = if cfg!(windows) { "node.exe" } else { "node" };
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

fn find_client_exe(bdms: &Path) -> Option<PathBuf> {
    let version_dir = bdms.parent()?.parent()?.parent()?;
    let name = if cfg!(windows) {
        "SodaMusic.exe"
    } else if cfg!(target_os = "macos") {
        "SodaMusic"
    } else {
        "soda-music"
    };
    let candidate = version_dir.join(name);
    candidate.is_file().then_some(candidate)
}

fn identity_file() -> PathBuf {
    if let Some(value) = std::env::var_os(IDENTITY_ENV) {
        return PathBuf::from(value);
    }
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("HotDownloader").join("qishui-identity.json")
}

fn load_or_create_identity() -> Result<Identity, String> {
    if let Some(value) = std::env::var_os(DEVICE_ENV) {
        let device_id = value.to_string_lossy().trim().to_string();
        if !device_id.is_empty() {
            return Ok(Identity {
                iid: format!("{device_id}1"),
                fp: device_id.clone(),
                device_id,
            });
        }
    }
    let path = identity_file();
    if let Ok(raw) = std::fs::read_to_string(&path) {
        if let Ok(identity) = serde_json::from_str::<Identity>(&raw) {
            if !identity.device_id.trim().is_empty() {
                return Ok(identity);
            }
        }
    }
    let millis = now_millis();
    let device_id = millis.to_string();
    let identity = Identity {
        iid: (millis + 1).to_string(),
        fp: device_id.clone(),
        device_id,
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string(&identity) {
        if std::fs::write(&path, text).is_err() {
            log::warn!("汽水设备身份写入失败：{}", path.display());
        }
    }
    Ok(identity)
}

fn mask(value: &str) -> String {
    let value = value.trim();
    if value.len() <= 6 {
        return "***".into();
    }
    format!("{}***{}", &value[..4], &value[value.len() - 2..])
}

#[cfg(test)]
mod tests {
    use super::{config_url, mask, parse_remote_signature, parse_signature, Identity};

    #[test]
    fn signature_output_is_parsed_with_case_insensitive_checks() {
        let raw = r#"{"headers":{"X-Helios":"h1","X-Medusa":"m1"}}"#;
        let headers = parse_signature(raw).unwrap();
        assert_eq!(headers.len(), 2);
        assert_eq!(headers.get("X-Helios").unwrap(), "h1");
    }

    #[test]
    fn incomplete_signature_is_rejected() {
        assert!(parse_signature(r#"{"headers":{"X-Helios":"h1"}}"#).is_err());
        assert!(parse_signature("not json").is_err());
    }

    #[test]
    fn remote_signature_accepts_flat_and_nested_shapes() {
        let nested = parse_remote_signature(
            "https://api.example/track",
            r#"{"ok":true,"headers":{"x-helios":"h","x-medusa":"m"}}"#,
        )
        .unwrap();
        assert_eq!(nested.headers.len(), 2);
        assert_eq!(nested.url, "https://api.example/track");

        let flat = parse_remote_signature(
            "https://api.example/track",
            r#"{"ok":true,"X-Helios":"h","X-Medusa":"m","ms_token":"tok"}"#,
        )
        .unwrap();
        assert!(flat.headers.keys().any(|key| key == "x-helios"));
        assert!(flat.url.contains("msToken=tok"));
    }

    #[test]
    fn remote_failure_surfaces_reason() {
        let error =
            parse_remote_signature("https://api.example", r#"{"ok":false,"error":"unauthorized"}"#)
                .unwrap_err();
        assert!(error.contains("unauthorized"));
    }

    #[test]
    fn config_url_strips_sign_suffix() {
        assert_eq!(
            config_url("http://127.0.0.1:8899/sign"),
            "http://127.0.0.1:8899/config"
        );
        assert_eq!(
            config_url("http://127.0.0.1:8899/sign/"),
            "http://127.0.0.1:8899/config"
        );
        assert_eq!(
            config_url("http://127.0.0.1:8899"),
            "http://127.0.0.1:8899/config"
        );
    }

    #[test]
    fn identity_falls_back_to_device_id_for_fp() {
        let identity = Identity {
            device_id: "1795350000000".into(),
            iid: "1795350000001".into(),
            fp: String::new(),
        };
        assert_eq!(identity.fp_or_device(), "1795350000000");
    }

    #[test]
    fn device_ids_are_masked() {
        assert_eq!(mask("1795350000000"), "1795***00");
        assert_eq!(mask("123"), "***");
    }
}
