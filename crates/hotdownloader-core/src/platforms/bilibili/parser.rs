//! 从视频详情构造歌曲对象：清洗标题，并在高置信度时拆出歌手。

use serde_json::{json, Value};

/// B 站 dash 音频流 id → 品质标签。
pub(super) fn quality_label(id: u64) -> Option<&'static str> {
    match id {
        30216 => Some("64kaac"),
        30232 => Some("132kaac"),
        30280 => Some("192kaac"),
        30250 => Some("杜比全景声"),
        30251 => Some("Hi-Res 无损"),
        _ => None,
    }
}

/// 去掉「【...】」「[...]」前缀（如「【官方 MV】」），保留正文。
fn clean_title(raw: &str) -> String {
    let mut text = raw.trim();
    for _ in 0..3 {
        let trimmed = text.trim_start();
        let stripped = if let Some(rest) = trimmed.strip_prefix('【') {
            rest.find('】').map(|end| &rest[end + '】'.len_utf8()..])
        } else if let Some(rest) = trimmed.strip_prefix('[') {
            rest.find(']').map(|end| &rest[end + 1..])
        } else {
            None
        };
        match stripped {
            Some(rest) => text = rest.trim_start(),
            None => break,
        }
    }
    text.trim().to_string()
}

/// 只在能明确区分时拆歌手：
///
/// - `歌名 - Rick Astley`（右侧是纯拉丁文名字）
/// - `《歌名》 - 歌手`（书名号里一定是歌名）
///
/// 中文的「周杰伦 - 晴天」无法可靠区分方向，保持整条标题不动，
/// 后续搜索与换源仍能用包含关系匹配到。
fn split_artist(title: &str) -> (String, String) {
    let Some((left, right)) = split_dash(title) else {
        return (title.to_string(), String::new());
    };
    let (left, right) = (left.trim(), right.trim());
    if left.is_empty() || right.is_empty() {
        return (title.to_string(), String::new());
    }
    let left_song = left.contains('《');
    let right_song = right.contains('《');
    if left_song != right_song {
        return if left_song {
            (strip_brackets(left), right.to_string())
        } else {
            (strip_brackets(right), left.to_string())
        };
    }
    if is_latin_name(right) {
        return (left.to_string(), right.to_string());
    }
    (title.to_string(), String::new())
}

fn split_dash(text: &str) -> Option<(&str, &str)> {
    for separator in [" - ", " – ", " — "] {
        if let Some(parts) = text.split_once(separator) {
            return Some(parts);
        }
    }
    None
}

fn strip_brackets(text: &str) -> String {
    text.replace('《', "").replace('》', "").trim().to_string()
}

fn is_latin_name(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 40
        && text
            .chars()
            .all(|c| c.is_ascii_alphabetic() || matches!(c, ' ' | '.' | '&' | '\'' | '-'))
        && text.chars().any(|c| c.is_ascii_alphabetic())
}

/// 从 dash 音频流构造音质列表。
///
/// `filename` 编码 `{bvid}_{cid}_{音频流 id}.m4a`：下载任务只持久化这个文件名，
/// 取流链接会在真正下载时重新请求，避免 B 站返回的限时地址提前过期。
pub(super) fn qualities_from_dash(
    data: &Value,
    bvid: &str,
    cid: u64,
    duration: u64,
) -> Vec<Value> {
    let Some(audio) = data["dash"]["audio"].as_array() else {
        return Vec::new();
    };
    let mut list: Vec<Value> = Vec::new();
    for item in audio {
        let Some(id) = item["id"].as_u64() else {
            continue;
        };
        let Some(label) = quality_label(id) else {
            continue;
        };
        if list.iter().any(|entry| entry["quality"] == label) {
            continue;
        }
        // dash 不返回体积，按码率 × 时长估算，用于界面展示。
        let bandwidth = item["bandwidth"].as_u64().unwrap_or(0);
        list.push(json!({
            "quality": label,
            "format": "m4a",
            "size": bandwidth / 8 * duration,
            "filename": format!("{bvid}_{cid}_{id}.m4a"),
        }));
    }
    list
}

/// 构造前端使用的歌曲对象。
pub(super) fn song(data: &Value, bvid: &str, qualities: Vec<Value>) -> Value {
    let cleaned = clean_title(data["title"].as_str().unwrap_or_default());
    let (title, artist) = split_artist(&cleaned);
    let artists = if artist.is_empty() {
        json!([])
    } else {
        json!([{ "id": "", "mid": "", "name": artist, "coverUrl": "" }])
    };
    json!({
        "id": data["aid"].as_u64().unwrap_or(0),
        "mid": bvid,
        "title": if title.is_empty() { "B 站视频".to_string() } else { title },
        "artist": artist,
        "artists": artists,
        "album": "",
        "albumId": "",
        "albumMid": "",
        "coverUrl": data["pic"].as_str().unwrap_or_default(),
        "mediaMid": bvid,
        "qualities": qualities,
    })
}

#[cfg(test)]
mod tests {
    use super::{clean_title, split_artist};

    #[test]
    fn strips_bracket_prefixes() {
        assert_eq!(
            clean_title("【官方 MV】Never Gonna Give You Up - Rick Astley"),
            "Never Gonna Give You Up - Rick Astley"
        );
        assert_eq!(clean_title("【4K修复】晴天"), "晴天");
        assert_eq!(clean_title("晴天"), "晴天");
    }

    #[test]
    fn splits_latin_artist_only() {
        assert_eq!(
            split_artist("Never Gonna Give You Up - Rick Astley"),
            ("Never Gonna Give You Up".into(), "Rick Astley".into())
        );
        // 中文标题方向无法确定，保持原样
        assert_eq!(
            split_artist("周杰伦 - 晴天"),
            ("周杰伦 - 晴天".into(), String::new())
        );
        // 书名号一侧一定是歌名
        assert_eq!(
            split_artist("《晴天》 - 周杰伦"),
            ("晴天".into(), "周杰伦".into())
        );
    }
}
