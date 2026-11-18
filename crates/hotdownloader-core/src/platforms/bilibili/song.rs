//! 单曲详情：按视频号取标题、封面与音频档位。

use serde_json::Value;

use super::{client, parser, valid_video_id};

/// 取视频详情并折算成歌曲对象。
///
/// 音频档位需要额外请求一次 `playurl`，取不到时保留空列表，
/// 界面会提示「暂无可用音质」而不是把整个解析判成失败。
pub async fn fetch_song_detail(id: &str) -> Result<String, String> {
    let id = id.trim();
    if !valid_video_id(id) {
        return Err("无效的 B 站视频号".into());
    }
    let view = fetch_view(id).await?;
    let bvid = view["bvid"].as_str().unwrap_or_default().trim().to_string();
    if bvid.is_empty() {
        return Err("B 站未返回视频号".into());
    }
    let cid = view["pages"]
        .as_array()
        .and_then(|pages| pages.first())
        .and_then(|page| page["cid"].as_u64())
        .ok_or("B 站未返回分P信息")?;
    let duration = view["duration"].as_u64().unwrap_or(0);
    let qualities = fetch_qualities(&bvid, cid, duration).await;
    Ok(parser::song(&view, &bvid, qualities).to_string())
}

/// `view` 接口同样支持 av 号，换算交给服务端。
async fn fetch_view(id: &str) -> Result<Value, String> {
    let url = match id.strip_prefix("av") {
        Some(aid) => format!("{}?aid={aid}", super::VIEW_API),
        None => format!("{}?bvid={id}", super::VIEW_API),
    };
    client::get_json(&url).await
}

async fn fetch_qualities(bvid: &str, cid: u64, duration: u64) -> Vec<Value> {
    let url = format!(
        "{}?bvid={bvid}&cid={cid}&fnval=16&fourk=1",
        super::PLAYURL_API
    );
    match client::get_json(&url).await {
        Ok(data) => parser::qualities_from_dash(&data, bvid, cid, duration),
        Err(_) => Vec::new(),
    }
}
