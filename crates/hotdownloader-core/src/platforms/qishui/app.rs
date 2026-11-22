//! 汽水 App 端（PC 客户端）接口：签名后的整曲取流与搜索。
//!
//! 这些接口要求「官方客户端」级别的请求：URL 带一整套 App 参数、`X-SS-STUB` 是请求体
//! 的 MD5、并且每次请求都由 [`super::signer`] 生成 `x-helios` / `x-medusa`。缺任何一项，
//! 服务端都回 `HTTP 200 + 空 body`——所以空响应要当成「签名或身份不对」而不是接口下线。
//!
//! 只有装了汽水客户端（或显式配置签名器）的机器才具备这条链路，调用前用
//! [`signed_ready`] 判断，未就绪时必须回退到免签 Web 端点。

use std::collections::HashMap;

use md5::{Digest, Md5};
use reqwest::Method;
use serde_json::{json, Value};

use super::signer::{self, Identity};
use super::{client, credentials, API_BASE};
use crate::platforms::CLIENT;

/// 整曲取流用的客户端 UA，与参数里的版本号对应。
pub(crate) const APP_UA: &str = "LunaPC/3.3.0(359450208)";
/// 搜索用的客户端 UA，与 `version_name=3.8.0` 对应。
const APP_SEARCH_UA: &str = "LunaPC/3.8.0";

/// Cookie 与签名器都就绪时才尝试 App 端接口。
pub fn signed_ready() -> bool {
    !credentials::cached_cookie().is_empty() && signer::available()
}

/// App 端公共查询参数；`device_id / iid / fp` 必须与签名器同一套身份。
fn app_params(version_name: &str, version_code: &str, identity: &Identity) -> Vec<(&'static str, String)> {
    vec![
        ("aid", super::AID.to_string()),
        ("app_name", "luna_pc".to_string()),
        ("region", "cn".to_string()),
        ("geo_region", "cn".to_string()),
        ("os_region", "cn".to_string()),
        ("sim_region", String::new()),
        ("device_id", identity.device_id.clone()),
        ("cdid", String::new()),
        ("iid", identity.iid.clone()),
        ("version_name", version_name.to_string()),
        ("version_code", version_code.to_string()),
        ("channel", "official".to_string()),
        ("build_mode", "master".to_string()),
        ("network_carrier", String::new()),
        ("ac", "wifi".to_string()),
        ("tz_name", "Asia/Shanghai".to_string()),
        ("resolution", String::new()),
        ("device_platform", "windows".to_string()),
        ("device_type", "Windows".to_string()),
        ("os_version", "Windows 11".to_string()),
        ("fp", identity.fp_or_device().to_string()),
    ]
}

pub(crate) fn track_v2_url(identity: &Identity) -> String {
    let params = app_params("3.3.0", "30030000", identity);
    query_url(&format!("{API_BASE}/pc/track_v2"), &params)
}

fn search_url(identity: &Identity, keyword: &str, cursor: u32, search_id: &str) -> String {
    let mut params = app_params("3.8.0", "38000000", identity);
    params.push(("q", keyword.to_string()));
    params.push(("cursor", cursor.to_string()));
    params.push(("search_id", search_id.to_string()));
    params.push(("search_method", "input".to_string()));
    params.push(("debug_params", String::new()));
    params.push(("from_search_id", String::new()));
    params.push(("search_scene", String::new()));
    query_url(&format!("{API_BASE}/pc/search/track"), &params)
}

fn query_url(base: &str, params: &[(&str, String)]) -> String {
    let pairs: Vec<(&str, &str)> = params
        .iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect();
    client::url_with(base, &pairs)
}

/// 请求体的 `X-SS-STUB`：大写十六进制 MD5。
fn stub_of(body: &str) -> String {
    format!("{:X}", Md5::digest(body.as_bytes()))
}

fn luna_headers(user_agent: &str, local_user: bool) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    headers.insert("User-Agent".to_string(), user_agent.to_string());
    headers.insert("Accept".to_string(), "application/json".to_string());
    headers.insert("x-luna-background-type".to_string(), "foreground".to_string());
    headers.insert("x-luna-is-background-req".to_string(), "0".to_string());
    headers.insert(
        "x-luna-is-local-user".to_string(),
        if local_user { "1" } else { "0" }.to_string(),
    );
    headers
}

/// 取 PC 端整曲详情；返回的 `track_player` 里是完整时长变体（含 `hi_res`）。
pub async fn fetch_pc_track(track_id: &str) -> Result<Value, String> {
    if credentials::cached_cookie().is_empty() {
        return Err("汽水 PC 取流需要登录 Cookie".into());
    }
    let identity = signer::identity().await?;
    let url = track_v2_url(&identity);
    let body = json!({
        "track_id": track_id.trim(),
        "media_type": "track",
        "queue_type": "favorite_track_playlist",
        "scene_name": "library",
    })
    .to_string();

    let mut headers = luna_headers(APP_UA, true);
    headers.insert("Content-Type".to_string(), "application/json; charset=utf-8".to_string());
    headers.insert("X-SS-STUB".to_string(), stub_of(&body));
    headers.insert("Cookie".to_string(), credentials::cached_cookie());

    let signature = signer::sign("POST", &url, &body, &headers).await?;
    send(
        Method::POST,
        &signature.url,
        Some(body),
        &headers,
        signature.headers,
    )
    .await
}

/// 带签名的 PC 搜索；返回结构与免签 Web 搜索一致（`result_groups`）。
pub async fn fetch_pc_search(keyword: &str) -> Result<Value, String> {
    if credentials::cached_cookie().is_empty() {
        return Err("汽水 PC 搜索需要登录 Cookie".into());
    }
    let identity = signer::identity().await?;
    let url = search_url(&identity, keyword, 0, &super::search::search_id());
    let mut headers = luna_headers(APP_SEARCH_UA, true);
    headers.insert("Cookie".to_string(), credentials::cached_cookie());

    let signature = signer::sign("GET", &url, "", &headers).await?;
    send(
        Method::GET,
        &signature.url,
        None,
        &headers,
        signature.headers,
    )
    .await
}

async fn send(
    method: Method,
    url: &str,
    body: Option<String>,
    headers: &HashMap<String, String>,
    signed: HashMap<String, String>,
) -> Result<Value, String> {
    let mut request = CLIENT.request(method, url);
    for (key, value) in headers.iter().chain(signed.iter()) {
        request = request.header(key, value);
    }
    if let Some(body) = body {
        request = request.body(body);
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("网络错误: {}", error.without_url()))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| format!("读取响应失败: {}", error.without_url()))?;
    if !status.is_success() {
        return Err(format!("汽水 PC 接口 HTTP 错误: {status}"));
    }
    if text.trim().is_empty() {
        return Err(
            "汽水 PC 接口返回空响应（签名头或设备身份未被接受，检查签名器与客户端登录态）".into(),
        );
    }
    serde_json::from_str(&text).map_err(|error| format!("解析响应失败: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{app_params, search_url, stub_of, track_v2_url};
    use crate::platforms::qishui::signer::Identity;

    fn identity() -> Identity {
        Identity {
            device_id: "1795350000000".into(),
            iid: "1795350000001".into(),
            fp: "1795350000000".into(),
        }
    }

    #[test]
    fn stub_is_uppercase_md5_of_body() {
        // MD5("{}") = 99914b932bd37a50b983c5e7c90ae93b
        assert_eq!(stub_of("{}"), "99914B932BD37A50B983C5E7C90AE93B");
    }

    #[test]
    fn app_params_carry_device_identity_and_version() {
        let params = app_params("3.3.0", "30030000", &identity());
        let lookup = |key: &str| {
            params
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| value.clone())
                .unwrap_or_default()
        };
        assert_eq!(lookup("device_id"), "1795350000000");
        assert_eq!(lookup("fp"), "1795350000000");
        assert_eq!(lookup("device_platform"), "windows");
        assert_eq!(lookup("version_code"), "30030000");
    }

    #[test]
    fn track_and_search_urls_are_signed_ready() {
        let track = track_v2_url(&identity());
        assert!(track.starts_with("https://api.qishui.com/luna/pc/track_v2?"));
        assert!(track.contains("aid=386088"));

        let search = search_url(&identity(), "周杰伦", 0, "sid");
        assert!(search.contains("q=%E5%91%A8%E6%9D%B0%E4%BC%A6"));
        assert!(search.contains("search_method=input"));
        assert!(search.contains("version_name=3.8.0"));
    }
}
