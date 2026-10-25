use super::{client::request_with, parser::valid_hash};
use crate::platforms::{lyric::LyricData, CLIENT};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::json;

pub async fn get_lyric(hash: &str) -> Result<LyricData, String> {
    if !valid_hash(hash) {
        return Err("无效的酷狗歌曲 hash".into());
    }
    let v = request_with(
        &CLIENT,
        "https://lyrics.kugou.com",
        "/v1/search",
        "",
        json!({
            "hash": hash,
            "album_audio_id": 0,
            "duration": 0,
            "keyword": "",
            "lrctxt": 1,
            "man": "no"
        }),
        None,
    )
    .await?;
    let Some(candidate) = v["candidates"].as_array().and_then(|v| v.first()) else {
        return Ok(LyricData::default());
    };
    let v = request_with(
        &CLIENT,
        "https://lyrics.kugou.com",
        "/download",
        "",
        json!({
            "ver": 1,
            "client": "android",
            "id": candidate["id"],
            "accesskey": candidate["accesskey"],
            "fmt": "lrc",
            "charset": "utf8"
        }),
        None,
    )
    .await?;
    let content = v["content"].as_str().ok_or("酷狗歌词响应缺少正文")?;
    let bytes = STANDARD
        .decode(content)
        .map_err(|e| format!("解码酷狗歌词失败: {e}"))?;
    let lrc = String::from_utf8(bytes).map_err(|e| format!("解析酷狗歌词失败: {e}"))?;
    Ok(LyricData {
        lrc: Some(lrc.clone()),
        raw: Some(lrc),
        elrc: None,
        instrumental: false,
    })
}
