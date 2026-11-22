//! 汽水搜索：签名 PC 接口 → 免签 Web 接口 → 移动端匿名接口，逐级兜底。
//!
//! 2026-11 实测：免签搜索一律返回 `200 + 空 body`，只有带 `x-helios` / `x-medusa` 的
//! App 端请求才能拿到结果。所以「能不能搜」取决于本机有没有汽水客户端（[`super::signer`]），
//! 没有时行为与改造前完全一致，只是提示语更准确。
//!
//! 两个接口都按 `cursor` 翻页且不做页码承诺，这里只提供首屏：
//! 项目其余平台用 `page/limit`，与其猜游标不如让前端拿到真实的一页。

use serde_json::{json, Value};

use super::client;
use super::parser;
use super::{ANDROID_UA, API_BASE};

/// 搜索结果没有音源信息，先给一个默认档位让任务可以创建；
/// 真正取流时 `link` 会重新请求详情，按码率挑最接近的变体。
const DEFAULT_QUALITY: &str = "128kaac";
const DEFAULT_BITRATE: u64 = 128;

pub async fn search_songs(
    separator: &str,
    keyword: String,
    page: u32,
    limit: u32,
) -> Result<String, String> {
    let keyword = keyword.trim().to_string();
    if keyword.is_empty() {
        return Err("请输入搜索关键词".into());
    }
    // 游标无法由页码推导，第二页起返回空列表，避免同一批结果反复出现。
    if page > 1 {
        return Ok(json!({ "songs": [], "has_more": false }).to_string());
    }
    let count = limit.clamp(1, 20);
    let body = fetch(&keyword, count).await?;
    let mut songs = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for track in tracks(&body) {
        let Some(track_id) = track["id"].as_str().map(str::to_string) else {
            continue;
        };
        if seen.contains(&track_id) {
            continue;
        }
        seen.push(track_id.clone());
        songs.push(with_default_quality(
            parser::song(track, separator, &[], &track_id),
        ));
    }
    Ok(json!({ "songs": songs, "has_more": false }).to_string())
}

/// 搜索结果只有元数据，补一个默认可下载档位；详情接口会给出真实音质列表。
fn with_default_quality(mut song: Value) -> Value {
    let duration = song["duration"].as_u64().unwrap_or(0);
    let Some(object) = song.as_object_mut() else {
        return song;
    };
    object.insert(
        "qualities".into(),
        json!([{
            "quality": DEFAULT_QUALITY,
            "format": "m4a",
            "size": duration * DEFAULT_BITRATE * 1000 / 8,
            "filename": "128.m4a",
        }]),
    );
    song
}

async fn fetch(keyword: &str, count: u32) -> Result<Value, String> {
    if super::app::signed_ready() {
        match super::app::fetch_pc_search(keyword).await {
            Ok(body) if body["result_groups"].is_array() => return Ok(body),
            Ok(_) => log::warn!("汽水签名搜索响应缺少结果分组，回退免签搜索"),
            Err(error) => log::warn!("汽水签名搜索失败，回退免签搜索：{error}"),
        }
    }
    let web = client::get_json(&web_search_url(keyword, count)).await;
    match web {
        Ok(body) if body["result_groups"].is_array() => Ok(body),
        Ok(_) => fallback(keyword, count, "Web 搜索响应缺少结果分组").await,
        Err(error) => fallback(keyword, count, &error).await,
    }
}

async fn fallback(keyword: &str, count: u32, reason: &str) -> Result<Value, String> {
    // 两个匿名入口都会被风控拦成 200 + 空 body，先把技术原因记进日志，
    // 对用户给可执行的建议，避免把"空响应"这类底层细节暴露到界面。
    let mobile = client::get_json_with_ua(&android_search_url(keyword, count), ANDROID_UA).await;
    if let Ok(body) = &mobile {
        if body["result_groups"].is_array() {
            return Ok(body.clone());
        }
    }
    log::warn!(
        "汽水搜索失败：web={reason}；mobile={:?}",
        mobile.as_ref().err()
    );
    Err(if super::signer::available() {
        "汽水搜索暂不可用：接口返回空响应，请改用分享链接解析".into()
    } else {
        "汽水搜索需要本机汽水客户端提供签名；未安装客户端时请改用分享链接解析".into()
    })
}

fn web_search_url(keyword: &str, count: u32) -> String {
    let count = count.to_string();
    let search_id = search_id();
    client::url_with(
        &format!("{API_BASE}/pc/search/track"),
        &[
            ("q", keyword),
            ("cursor", "0"),
            ("count", &count),
            ("search_id", &search_id),
            ("search_method", "input"),
            ("debug_params", ""),
            ("from_search_id", ""),
            ("search_scene", ""),
            ("aid", super::AID),
            ("device_platform", "web"),
            ("channel", "pc_web"),
        ],
    )
}

/// 移动端搜索的固定设备指纹与 UA 版本对应；只作为 Web 搜索失败后的兜底。
fn android_search_url(keyword: &str, count: u32) -> String {
    let count = count.to_string();
    let ticket = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "0".into());
    client::url_with(
        &format!("{API_BASE}/search/track"),
        &[
            ("q", keyword),
            ("cursor", "0"),
            ("count", &count),
            ("aid", super::AID),
            ("device_platform", "android"),
            ("app_name", "luna"),
            ("version_code", "100198030"),
            ("version_name", "19.8.0"),
            ("manifest_version_code", "100198030"),
            ("device_id", "2204957404565290"),
            ("iid", "2204957404569386"),
            ("cdid", "46556f98-1720-4248-83da-62b74b60b46a"),
            ("channel", "xiaomi_8478_64"),
            ("os", "android"),
            ("os_version", "15"),
            ("device_type", "ABR-AL80"),
            ("device_brand", "HUAWEI"),
            ("device_model", "ABR-AL80"),
            ("sim_region", "cn"),
            ("tz_name", "Asia/Shanghai"),
            ("_rticket", &ticket),
        ],
    )
}

/// 从 `result_groups[].data[].entity.track` 取出曲目，忽略歌单与视频。
fn tracks(body: &Value) -> Vec<&Value> {
    body["result_groups"]
        .as_array()
        .map(|groups| {
            groups
                .iter()
                .filter_map(|group| group["data"].as_array())
                .flatten()
                .filter_map(|item| {
                    let track = &item["entity"]["track"];
                    track["id"].is_string().then_some(track)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 与客户端一致：每次搜索生成一个随机 UUID，服务端用它串联同一次搜索。
pub(crate) fn search_id() -> String {
    let mut bytes: [u8; 16] = rand::random();
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

#[cfg(test)]
mod tests {
    use super::{search_id, tracks, with_default_quality};
    use serde_json::json;

    #[test]
    fn search_ids_are_uuid_v4_shaped() {
        let id = search_id();
        assert_eq!(id.len(), 36);
        assert_eq!(id.as_bytes()[14], b'4');
        assert_ne!(id, search_id());
    }

    #[test]
    fn only_tracks_are_collected_from_result_groups() {
        let body = json!({
            "result_groups": [
                {"data": [
                    {"entity": {"track": {"id": "7512772704484722704"}}},
                    {"entity": {"playlist": {"id": "1"}}}
                ]}
            ]
        });
        assert_eq!(tracks(&body).len(), 1);
    }

    #[test]
    fn default_quality_is_attached_for_search_results() {
        let song = with_default_quality(json!({"duration": 200}));
        assert_eq!(song["qualities"][0]["filename"], "128.m4a");
        assert_eq!(song["qualities"][0]["quality"], "128kaac");
        assert!(song["qualities"][0]["size"].as_u64().unwrap() > 0);
    }
}
