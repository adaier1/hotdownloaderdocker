//! B 站请求统一入口：固定 UA 与 Referer，并把 `code != 0` 转成可读错误。

use serde_json::Value;

use crate::platforms::CLIENT;

/// B 站接口对 UA 与 Referer 敏感，缺失时可能返回 -352 风控错误。
pub(crate) const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
pub(crate) const REFERER: &str = "https://www.bilibili.com";

/// 发起 GET 请求并返回 `data` 字段；接口错误按平台文案抛出。
pub(super) async fn get_json(url: &str) -> Result<Value, String> {
    let response = CLIENT
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header("Referer", REFERER)
        .send()
        .await
        .map_err(|error| format!("B 站请求失败: {}", error.without_url()))?
        .error_for_status()
        .map_err(|error| format!("B 站请求失败: {}", error.without_url()))?;
    let value: Value = response
        .json()
        .await
        .map_err(|error| format!("B 站响应解析失败: {error}"))?;
    let code = value["code"].as_i64().unwrap_or(-1);
    if code != 0 {
        let message = value["message"].as_str().unwrap_or("未知错误");
        return Err(format!("B 站接口返回错误（{code}）：{message}"));
    }
    Ok(value["data"].clone())
}
