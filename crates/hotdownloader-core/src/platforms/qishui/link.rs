//! 下载链接：重新取一次详情，按品质文件名挑出对应播放变体。
//!
//! 汽水没有「按档位请求地址」的能力，一次响应就给出全部可用流，
//! 因此这里沿用项目既有的 `{bitrate}.{format}` 文件名约定反查档位。

use reqwest::Client;

use super::parser::{self, Player};

/// 返回 `(播放地址, play_auth)`；`play_auth` 非空表示下载完成后需要解密。
pub async fn get_download_link(
    _client: &Client,
    song_mid: &str,
    filename: &str,
) -> Result<(String, String), String> {
    let song_mid = song_mid.trim();
    if !super::is_track_id(song_mid) {
        return Err("无效的汽水曲目 ID".into());
    }
    let (stem, _) = split_filename(filename)?;
    let payload = parser::fetch_track_payload(song_mid).await?;
    let full_duration = parser::primary_track(&payload)
        .map(parser::track_duration)
        .unwrap_or(0);
    let players = parser::players(&payload, full_duration).await;
    let player = pick(&players, &stem).ok_or_else(|| {
        if players.is_empty() {
            "汽水未返回可播放音源".to_string()
        } else {
            format!("该音质不可用: {filename}")
        }
    })?;
    if player.preview {
        log::warn!("汽水曲目 {song_mid} 仅返回试听片段");
    }
    Ok((player.url.clone(), player.play_auth.clone()))
}

fn split_filename(filename: &str) -> Result<(String, String), String> {
    let path = std::path::Path::new(filename);
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("filename 缺少扩展名: {filename}"))?;
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("filename 缺少档位: {filename}"))?;
    Ok((stem.to_string(), extension.to_string()))
}

/// 优先非试听流；数字档位只在 AAC 变体里找最接近的码率，避免把无损当成 320k。
fn pick<'a>(players: &'a [Player], stem: &str) -> Option<&'a Player> {
    let playable: Vec<&Player> = players.iter().filter(|player| !player.preview).collect();
    let pool: Vec<&Player> = if playable.is_empty() {
        players.iter().collect()
    } else {
        playable
    };
    match stem.parse::<u32>() {
        Ok(bitrate) => pool
            .iter()
            .filter(|player| {
                player
                    .quality_label()
                    .is_some_and(|label| label.ends_with("kaac"))
            })
            .min_by_key(|player| (i64::from(player.bitrate) - i64::from(bitrate)).abs())
            .copied(),
        Err(_) => pool
            .iter()
            .find(|player| player.quality_label() == Some(stem))
            .copied(),
    }
}

#[cfg(test)]
mod tests {
    use super::{pick, split_filename};
    use crate::platforms::qishui::parser::Player;

    fn player(bitrate: u32, format: &str, quality: &str) -> Player {
        Player {
            url: format!("https://cdn.example/{bitrate}"),
            play_auth: String::new(),
            format: format.into(),
            bitrate,
            size: 0,
            duration: 200,
            quality: quality.into(),
            preview: false,
        }
    }

    fn players() -> Vec<Player> {
        vec![
            player(900, "flac", "lossless"),
            player(320, "m4a", "high"),
            player(128, "m4a", "medium"),
        ]
    }

    #[test]
    fn filenames_follow_the_bitrate_contract() {
        assert_eq!(split_filename("320.m4a").unwrap(), ("320".into(), "m4a".into()));
        assert_eq!(
            split_filename("flac.flac").unwrap(),
            ("flac".into(), "flac".into())
        );
        assert!(split_filename("noext").is_err());
    }

    #[test]
    fn numeric_tiers_never_select_lossless() {
        let pool = players();
        let selected = pick(&pool, "320").unwrap();
        assert_eq!(selected.bitrate, 320);
        let selected = pick(&pool, "128").unwrap();
        assert_eq!(selected.bitrate, 128);
        // 请求 64k 时最接近的是 128k，但仍必须是 AAC 变体而不是无损。
        assert_eq!(pick(&pool, "64").unwrap().bitrate, 128);
    }

    #[test]
    fn label_tiers_match_flac_and_hires() {
        let pool = players();
        assert_eq!(pick(&pool, "flac").unwrap().bitrate, 900);
        assert!(pick(&pool, "hires").is_none());
        assert!(pick(&pool, "spatial").is_none());
    }
}
