//! 歌词：详情接口本身带 `lyric.content`，不需要额外请求。
//!
//! 汽水对免费曲目返回带时间轴的 LRC，会员试听曲目同样有正文；
//! 没有正文时按「无歌词」处理，而不是把任务判成失败。

use super::{is_track_id, parser};
use crate::platforms::lyric::LyricData;

pub async fn get_lyric(track_id: &str) -> Result<LyricData, String> {
    let track_id = track_id.trim();
    if !is_track_id(track_id) {
        return Err("无效的汽水曲目 ID".into());
    }
    let payload = parser::fetch_track_payload(track_id).await?;
    let Some(raw) = parser::lyric_text(&payload) else {
        return Ok(LyricData::default());
    };
    // 汽水给的是逐字歌词，转成标准 LRC 再交给播放器；原文留在 raw 字段备查。
    let Some(lrc) = parser::lyric_to_lrc(&raw) else {
        return Ok(LyricData::default());
    };
    Ok(LyricData {
        lrc: Some(lrc),
        raw: Some(raw),
        elrc: None,
        instrumental: false,
    })
}
