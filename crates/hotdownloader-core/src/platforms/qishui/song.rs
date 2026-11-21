//! 单曲详情：按曲目 ID 取元数据、歌词与可用音质。

use super::{client, is_track_id, parser, track_id_from_text};

/// 把分享短链解析成曲目 ID：先离线正则，必要时跟随跳转并读落地页。
///
/// 汽水分享页的 `track_id` 可能只出现在内嵌 JSON 里，最终 URL 上看不到，
/// 所以带 ID 的 URL 和整页 HTML 都要再匹配一次。
pub async fn resolve_track_id(text: &str) -> Result<String, String> {
    if let Some(track_id) = track_id_from_text(text) {
        return Ok(track_id);
    }
    if !text.starts_with("http") {
        return Err("未能从汽水分享内容中提取曲目 ID".into());
    }
    let (final_url, body) = client::get_text(text).await?;
    track_id_from_text(&final_url)
        .or_else(|| track_id_from_text(&body))
        .ok_or_else(|| "未能从汽水分享页提取曲目 ID，请确认是单曲分享链接".to_string())
}

/// 取单曲详情并折算成统一歌曲对象。
///
/// 音源拿不到时仍然返回元数据：分享链接解析需要歌名和封面，
/// 只是 `qualities` 为空，前端会提示暂无可用音质。
pub async fn fetch_song_detail(separator: &str, track_id: &str) -> Result<String, String> {
    let track_id = track_id.trim();
    if !is_track_id(track_id) {
        return Err("无效的汽水曲目 ID".into());
    }
    let payload = parser::fetch_track_payload(track_id).await?;
    let track = parser::primary_track(&payload)
        .cloned()
        .ok_or_else(|| "汽水未返回曲目信息".to_string())?;
    let full_duration = parser::track_duration(&track);
    let players = parser::players(&payload, full_duration).await;
    Ok(parser::song(&track, separator, &players, track_id).to_string())
}
