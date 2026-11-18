//! 单曲详情：按歌曲 hash 取 playInfo，复用统一歌曲解析器。

use serde_json::{json, Value};

use crate::platforms::CLIENT;

use super::parser::{parse_song, valid_hash};

/// 取单曲详情。
///
/// 酷狗分享链接里的标识是 32 位 hash，`getSongInfo.php?cmd=playInfo` 对匿名请求可用，
/// 返回结构与搜索接口字段名不完全一致，因此先归一化再交给 [`parse_song`]。
pub async fn fetch_song_detail(separator: &str, hash: &str) -> Result<String, String> {
    let hash = hash.trim().to_lowercase();
    if !valid_hash(&hash) {
        return Err("无效的酷狗歌曲 hash".into());
    }

    let value: Value = CLIENT
        .post("http://m.kugou.com/app/i/getSongInfo.php")
        .header("Host", "m.kugou.com")
        .json(&json!({ "cmd": "playInfo", "hash": hash, "from": "mkugou" }))
        .send()
        .await
        .map_err(|error| format!("酷狗详情网络错误: {}", error.without_url()))?
        .error_for_status()
        .map_err(|error| format!("酷狗详情 HTTP 错误: {}", error.without_url()))?
        .json()
        .await
        .map_err(|error| format!("解析酷狗详情响应失败: {error}"))?;

    // 会员歌曲会返回 `status=0` 与「需要付费」，但歌名、歌手、封面仍在响应里，
    // 此时保留元数据、音质列表为空，由界面提示「暂无可用音质」而不是整体失败。
    let payload = normalize(value, &hash);
    parse_song(&payload, separator)
        .filter(|song| !song["title"].as_str().unwrap_or_default().is_empty())
        .map(|song| song.to_string())
        .ok_or_else(|| "酷狗未返回该歌曲信息".to_string())
}

/// playInfo 的字段名与搜索接口不同（`songName` vs `SongName`、`albumid` vs `AlbumID`），
/// 这里补一份解析器认识的字段，避免标题、专辑与音质信息丢失。
fn normalize(value: Value, hash: &str) -> Value {
    let mut payload = match value.get("data") {
        Some(data) if data.is_object() => data.clone(),
        _ => value,
    };
    let Some(object) = payload.as_object_mut() else {
        return payload;
    };
    let aliases: [(&str, &[&str]); 4] = [
        ("SongName", &["songName", "audio_name"]),
        ("AlbumID", &["albumid", "req_albumid"]),
        ("Image", &["album_img", "imgUrl"]),
        ("FileName", &["fileName"]),
    ];
    for (target, sources) in aliases {
        if object.contains_key(target) {
            continue;
        }
        let found = sources
            .iter()
            .find_map(|key| object.get(*key).cloned())
            .filter(|value| !value.is_null());
        if let Some(value) = found {
            object.insert(target.to_string(), value);
        }
    }
    // VIP 歌曲在主字段里 `fileSize` 为 0，但 `extra` 仍列出 128/320/SQ/Hi-Res 四档的
    // hash 与体积。把这些补成解析器认识的键，登录用户才能看到完整音质列表。
    if let Some(extra) = object.get("extra").and_then(Value::as_object).cloned() {
        let tiers: [(&str, &str); 8] = [
            ("FileHash", "128hash"),
            ("FileSize", "128filesize"),
            ("HQFileHash", "320hash"),
            ("HQFileSize", "320filesize"),
            ("SQFileHash", "sqhash"),
            ("SQFileSize", "sqfilesize"),
            ("ResFileHash", "highhash"),
            ("ResFileSize", "highfilesize"),
        ];
        for (target, source) in tiers {
            if object.contains_key(target) {
                continue;
            }
            if let Some(value) = extra.get(source) {
                let usable = value
                    .as_str()
                    .map(|text| !text.is_empty())
                    .unwrap_or_else(|| value.as_u64().unwrap_or(0) > 0);
                if usable {
                    object.insert(target.to_string(), value.clone());
                }
            }
        }
    }
    // 主字段 `fileSize` 对 VIP 歌曲为 0，只有正值才值得补。
    if !object.contains_key("FileSize")
        && object.get("fileSize").and_then(Value::as_u64).unwrap_or(0) > 0
    {
        let size = object["fileSize"].clone();
        object.insert("FileSize".to_string(), size);
    }
    object
        .entry("hash".to_string())
        .or_insert_with(|| json!(hash));
    payload
}
