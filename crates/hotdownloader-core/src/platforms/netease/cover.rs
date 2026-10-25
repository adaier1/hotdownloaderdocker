//! 按歌曲 ID 查询封面，详情补全复用歌曲模块。
use super::song;

pub async fn fetch_cover(song_id: u64, cookie: &str) -> Result<String, String> {
    if song_id == 0 {
        return Err("无效的网易云歌曲 ID".into());
    }
    let songs = song::fetch_details(&[song_id], "、", cookie).await?;
    Ok(songs
        .first()
        .and_then(|song| song["coverUrl"].as_str())
        .unwrap_or("")
        .to_string())
}
