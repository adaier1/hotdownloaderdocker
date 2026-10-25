use super::{
    client::{check_response, number, request_with},
    parser::valid_hash,
};
use serde_json::{json, Value};

fn source(filename: &str) -> Result<(&str, &str, &str, &str), String> {
    let (stem, extension) = filename.rsplit_once('.').ok_or("无效的酷狗音质文件名")?;
    let parts: Vec<_> = stem.split('_').collect();
    if parts.len() != 4
        || !valid_hash(parts[0])
        || parts[1].parse::<u64>().is_err()
        || parts[2].parse::<u64>().is_err()
    {
        return Err("无效的酷狗音源标识".into());
    }
    let quality = match (parts[3], extension) {
        ("128kmp3", "mp3") => "128",
        ("320kmp3", "mp3") => "320",
        ("flac", "flac") => "flac",
        ("hires", "flac") => "high",
        _ => return Err("不支持的酷狗音质".into()),
    };
    Ok((parts[0], parts[1], parts[2], quality))
}

fn parse_link(v: &Value) -> Result<(String, String), String> {
    // 不能把试听片段当作完整文件下载。
    if number(&v["is_free_part"]) != 0 || number(&v["free_part"]) != 0 {
        return Err("酷狗仅返回试听片段，无法获取完整音源".into());
    }
    let url = v["url"]
        .as_array()
        .and_then(|a| a.iter().filter_map(Value::as_str).find(|s| !s.is_empty()))
        .or_else(|| v["url"].as_str())
        .ok_or("酷狗未返回可用音源，请检查歌曲权限或登录状态")?;
    let parsed = reqwest::Url::parse(url).map_err(|_| "酷狗返回了无效音源地址")?;
    if !["http", "https"].contains(&parsed.scheme()) {
        return Err("酷狗返回了无效音源协议".into());
    }
    Ok((url.to_owned(), String::new()))
}

pub async fn get_download_link(
    client: &reqwest::Client,
    song_mid: &str,
    filename: &str,
) -> Result<(String, String), String> {
    get_download_link_with_credentials(client, song_mid, filename, json!({})).await
}

pub async fn get_download_link_with_credentials(
    client: &reqwest::Client,
    song_mid: &str,
    filename: &str,
    mut credentials: Value,
) -> Result<(String, String), String> {
    let (hash, album, id, quality) = source(filename)?;
    credentials
        .as_object_mut()
        .ok_or("酷狗凭据格式错误")?
        .extend(
            json!({
                "hash": hash.to_lowercase(),
                "album_id": album,
                "album_audio_id": id,
                "quality": quality,
                "area_code": 1,
                "ssa_flag": "is_fromtrack",
                "version": 11430,
                "page_id": 151369488,
                "behavior": "play",
                "pid": 2,
                "cmd": 26,
                "pidversion": 3001,
                "IsFreePart": 0,
                "ppage_id": "463467626,350369493,788954147",
                "cdnBackup": 1,
                "module": "",
                "clientver": 11430
            })
            .as_object()
            .unwrap()
            .clone(),
        );
    let primary = request_with(
        client,
        "https://gateway.kugou.com",
        "/v5/url",
        "trackercdn.kugou.com",
        credentials,
        None,
    )
    .await
    .and_then(|value| parse_link(&value));
    match primary {
        Ok(link) => Ok(link),
        // 移动端接口固定返回 128k MP3，只能用于对应候选。
        // 高音质失败由共用降级流程切换品质、文件后缀和大小后再尝试。
        Err(primary_error) if quality == "128" => fetch_mobile_link(client, song_mid)
            .await
            .map_err(|fallback_error| {
                format!("{primary_error}；酷狗 128k MP3 兜底失败：{fallback_error}")
            }),
        Err(error) => Err(error),
    }
}

fn parse_mobile_link(value: Value, hash: &str) -> Result<(String, String), String> {
    let mut value = check_response(value)?;
    if !value["hash"]
        .as_str()
        .is_some_and(|returned| returned.eq_ignore_ascii_case(hash))
    {
        return Err("酷狗兜底返回了不匹配的歌曲 hash".into());
    }
    if number(&value["bitRate"]) != 128 || value["extName"].as_str() != Some("mp3") {
        return Err("酷狗兜底未返回 128k MP3 音源".into());
    }
    if number(&value["fileHead"]) != 0 {
        return Err("酷狗兜底返回了不完整音源".into());
    }
    if value["url"]
        .as_str()
        .map_or(true, |url| url.trim().is_empty())
    {
        value["url"] = value["backup_url"].clone();
    }
    parse_link(&value)
}

/// m.kugou 的 playInfo 请求不携带品质参数和登录凭据，固定作为 128k MP3 兜底。
async fn fetch_mobile_link(
    client: &reqwest::Client,
    hash: &str,
) -> Result<(String, String), String> {
    if !valid_hash(hash) {
        return Err("无效的酷狗歌曲 hash".into());
    }
    let response = client
        .post("http://m.kugou.com/app/i/getSongInfo.php")
        .header("Host", "m.kugou.com")
        .json(&json!({ "cmd": "playInfo", "hash": hash, "from": "mkugou" }))
        .send()
        .await
        .map_err(|error| format!("酷狗兜底网络错误: {}", error.without_url()))?
        .error_for_status()
        .map_err(|error| format!("酷狗兜底 HTTP 错误: {}", error.without_url()))?;
    let value = response
        .json::<Value>()
        .await
        .map_err(|error| format!("解析酷狗兜底响应失败: {error}"))?;
    parse_mobile_link(value, hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quality_source_is_preserved() {
        assert_eq!(
            source("0123456789abcdef0123456789abcdef_123_456_flac.flac").unwrap(),
            ("0123456789abcdef0123456789abcdef", "123", "456", "flac")
        );
        assert!(source("0123456789abcdef0123456789abcdef_123_456_flac.mp3").is_err());
        assert!(parse_link(&json!({
            "url": ["https://example.org/audio"],
            "is_free_part": 1
        }))
        .is_err());
        assert!(parse_link(&json!({
            "url": []
        }))
        .is_err());
    }

    #[test]
    fn mobile_fallback_checks_actual_song_and_quality() {
        let hash = "7EC5DC174A67E301C51900C8B684A76E";
        let response = json!({
            "status": 1, "errcode": 0, "hash": hash.to_lowercase(),
            "bitRate": 128, "extName": "mp3", "fileHead": 0,
            "url": "https://example.org/audio.mp3",
            "backup_url": ["https://example.org/backup.mp3"]
        });
        assert_eq!(
            parse_mobile_link(response.clone(), hash).unwrap().0,
            "https://example.org/audio.mp3"
        );
        let mut backup = response.clone();
        backup["url"] = json!("");
        assert_eq!(
            parse_mobile_link(backup, hash).unwrap().0,
            "https://example.org/backup.mp3"
        );
        for (field, value) in [
            ("status", json!(0)),
            ("errcode", json!(20028)),
            ("hash", json!("0123456789abcdef0123456789abcdef")),
            ("bitRate", json!(320)),
            ("extName", json!("flac")),
            ("fileHead", json!(1)),
            ("is_free_part", json!(1)),
        ] {
            let mut invalid = response.clone();
            invalid[field] = value;
            assert!(parse_mobile_link(invalid, hash).is_err(), "{field}");
        }
    }

    #[tokio::test]
    #[ignore = "手动验证用户提供的酷狗移动端兜底接口"]
    async fn live_mobile_fallback() {
        let (url, key) = fetch_mobile_link(
            &crate::platforms::CLIENT,
            "7EC5DC174A67E301C51900C8B684A76E",
        )
        .await
        .unwrap();
        assert!(url.starts_with("https://"));
        assert!(key.is_empty());
    }
}
