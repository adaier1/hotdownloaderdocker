//! 汽水接口的 HTTP 封装：统一浏览器 UA，并把空响应识别成签名缺失。
//!
//! 错误前缀沿用 `download::link::is_retryable_link_error` 认可的文案，
//! 临时网络问题才能触发现有重试逻辑。

use serde_json::Value;

use super::WEB_UA;
use crate::platforms::CLIENT;

/// 带浏览器 UA 的 GET 请求，汽水所有匿名端点都走这里。
///
/// 配置了登录 Cookie 时一并携带：未登录请求的搜索会被服务端拦成空响应。
pub(crate) fn get(url: &str) -> reqwest::RequestBuilder {
    with_identity(
        CLIENT
            .get(url)
            .header("User-Agent", WEB_UA)
            .header("Accept", "application/json, text/plain, */*"),
    )
}

/// 统一附加登录 Cookie，避免每个调用点各写一遍。
fn with_identity(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    let cookie = super::credentials::cached_cookie();
    if cookie.is_empty() {
        request
    } else {
        request.header("Cookie", cookie)
    }
}

/// 取 JSON 响应。空 body 是服务端拒绝签名的固定表现，单独给出可读原因。
pub async fn get_json(url: &str) -> Result<Value, String> {
    get_json_with_ua(url, WEB_UA).await
}

/// 移动端兜底接口需要与参数版本一致的 UA。
pub async fn get_json_with_ua(url: &str, user_agent: &str) -> Result<Value, String> {
    let response = with_identity(
        CLIENT
            .get(url)
            .header("User-Agent", user_agent)
            .header("Accept", "application/json"),
    )
    .send()
    .await
    .map_err(|error| format!("网络错误: {}", error.without_url()))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| format!("读取响应失败: {}", error.without_url()))?;
    if !status.is_success() {
        return Err(format!("汽水接口 HTTP 错误: {status}"));
    }
    if text.trim().is_empty() {
        return Err("汽水接口返回空响应".into());
    }
    serde_json::from_str(&text).map_err(|error| format!("解析响应失败: {error}"))
}

/// 取整页 HTML，用于分享短链落地页里的内嵌数据。
pub async fn get_text(url: &str) -> Result<(String, String), String> {
    let response = get(url)
        .header("Accept", "text/html,application/xhtml+xml")
        .send()
        .await
        .map_err(|error| format!("网络错误: {}", error.without_url()))?;
    let final_url = response.url().to_string();
    if !response.status().is_success() {
        return Err(format!("汽水分享页 HTTP 错误: {}", response.status()));
    }
    let text = response
        .text()
        .await
        .map_err(|error| format!("读取响应失败: {}", error.without_url()))?;
    Ok((final_url, text))
}

/// 拼接查询串；布尔与数字都由调用方转成字符串，避免引入额外依赖。
pub(crate) fn url_with(base: &str, params: &[(&str, &str)]) -> String {
    let mut url = reqwest::Url::parse(base).expect("汽水接口地址固定");
    {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in params {
            pairs.append_pair(key, value);
        }
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::{get, url_with};
    use crate::platforms::qishui::credentials::set_cached_cookie;

    #[test]
    fn query_parameters_are_appended_in_order() {
        let url = url_with(
            "https://api.qishui.com/luna/pc/track_v2",
            &[("track_id", "7512772704484722704"), ("media_type", "track")],
        );
        assert!(url.contains("track_id=7512772704484722704"));
        assert!(url.contains("media_type=track"));
    }

    #[test]
    fn login_cookie_is_attached_only_when_configured() {
        set_cached_cookie("sessionid=abc; uid_tt=def");
        let request = get("https://api.qishui.com/luna/pc/track_v2")
            .build()
            .unwrap();
        assert_eq!(
            request.headers().get("Cookie").unwrap(),
            "sessionid=abc; uid_tt=def"
        );

        set_cached_cookie("");
        let request = get("https://api.qishui.com/luna/pc/track_v2")
            .build()
            .unwrap();
        assert!(request.headers().get("Cookie").is_none());
    }
}
