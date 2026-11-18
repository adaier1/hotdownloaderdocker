//! 单曲详情：按 rid 取歌曲元信息。
//!
//! 酷我的 www 接口需要 csrf 令牌，单曲详情改用移动端只读接口，
//! 匿名可用；该接口不返回 `N_MINFO`，因此音质列表按 128k MP3 兜底，
//! 这也是酷我无需登录即可下载的档位。

use serde_json::{json, Value};

use crate::platforms::CLIENT;

/// 取单曲详情。
///
/// 酷我该接口会偶发限流（返回非 200 或空 `data`），解析是即时操作，
/// 重试一次足以覆盖大部分抖动，两次都失败才向调用方报错。
pub async fn fetch_song_detail(separator: &str, rid: &str) -> Result<String, String> {
    let rid: u64 = rid
        .trim()
        .parse()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| "无效的酷我歌曲 ID".to_string())?;

    let mut data = None;
    let mut last_error = String::new();
    for attempt in 0..2 {
        // 失败后稍等再试：酷我对连续请求敏感，立即重试只会更快触限流。
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(600)).await;
        }
        match request_detail(rid).await {
            Ok(value) => {
                data = Some(value);
                break;
            }
            Err(error) => last_error = error,
        }
    }
    let data = data.ok_or(last_error)?;
    let info = &data["data"]["songinfo"];
    let title = text(info, &["name", "songName"]);
    if title.is_empty() {
        return Err("酷我未返回歌曲名称".into());
    }
    let raw_artist = text(info, &["artist", "artistName"]);
    let artists: Vec<Value> = raw_artist
        .split('&')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| {
            json!({
                "id": "",
                "mid": "",
                "name": name,
                "coverUrl": ""
            })
        })
        .collect();
    let artist = artists
        .iter()
        .filter_map(|item| item["name"].as_str())
        .collect::<Vec<_>>()
        .join(separator);
    let duration = parse_duration(&text(info, &["songTimeMinutes", "duration"]));
    // 128k MP3 是酷我匿名可用的档位，filename 约定与歌单接口保持一致（`{bitrate}.{format}`）。
    let qualities = vec![json!({
        "quality": "128kmp3",
        "format": "mp3",
        "size": duration * 16_000,
        "filename": "128.mp3"
    })];

    Ok(json!({
        "id": rid,
        "mid": rid.to_string(),
        "title": title,
        "artist": artist,
        "artists": artists,
        "album": text(info, &["album", "albumName"]),
        "albumId": text(info, &["albumId"]),
        "albumMid": "",
        "duration": duration,
        "coverUrl": text(info, &["albumpic", "pic"]),
        "mediaMid": rid.to_string(),
        "qualities": qualities
    })
    .to_string())
}

async fn request_detail(rid: u64) -> Result<Value, String> {
    let data: Value = CLIENT
        .get("http://m.kuwo.cn/newh5/singles/songinfoandlrc")
        .query(&[("musicId", rid.to_string()), ("httpsStatus", "1".into())])
        .header("Referer", "http://m.kuwo.cn/")
        .send()
        .await
        .map_err(|error| format!("酷我详情网络错误: {}", error.without_url()))?
        .error_for_status()
        .map_err(|error| format!("酷我详情 HTTP 错误: {}", error.without_url()))?
        .json()
        .await
        .map_err(|error| format!("解析酷我详情响应失败: {error}"))?;
    let status = data["status"].as_u64().unwrap_or(0);
    if status != 200 {
        return Err(match status {
            // 301 表示酷我要求先做一次验证/跳转，通常是请求过密触发的限流。
            301 => "酷我接口要求验证（status=301），请稍后重试".to_string(),
            _ => format!("酷我详情接口返回错误（status={status}）"),
        });
    }
    if !data["data"]["songinfo"].is_object() {
        return Err("酷我未返回该歌曲信息".into());
    }
    // 限流时接口会返回 200 但字段为空，这种情况同样值得重试。
    let named = ["name", "songName"].iter().any(|key| {
        data["data"]["songinfo"][*key]
            .as_str()
            .is_some_and(|text| !text.trim().is_empty())
    });
    if !named {
        return Err("酷我接口繁忙（返回数据不完整），请稍后重试".into());
    }
    Ok(data)
}

fn text(value: &Value, keys: &[&str]) -> String {
    keys.iter()
        .filter_map(|key| value[*key].as_str())
        .find(|text| !text.trim().is_empty())
        .unwrap_or_default()
        .to_string()
}

/// 接口给的是 `03:45` 或纯秒数，统一转成秒。
fn parse_duration(raw: &str) -> u64 {
    let raw = raw.trim();
    if let Some((minutes, seconds)) = raw.split_once(':') {
        let minutes: u64 = minutes.parse().unwrap_or(0);
        let seconds: u64 = seconds.parse().unwrap_or(0);
        return minutes * 60 + seconds;
    }
    raw.parse().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::parse_duration;

    #[test]
    fn durations_are_normalized_to_seconds() {
        assert_eq!(parse_duration("03:45"), 225);
        assert_eq!(parse_duration("225"), 225);
        assert_eq!(parse_duration(""), 0);
    }
}
