//! 网易云音源解析及实际音质、格式和试听限制校验。
use super::{
    client::{self, request, validate_id},
    parser::{list, LEVELS},
};
use serde_json::{json, Value};

pub async fn get_download_link(
    song_id: &str,
    filename: &str,
    cookie: &str,
) -> Result<(String, String), String> {
    let numeric = validate_id(song_id)?;
    let level = LEVELS
        .iter()
        .find(|(_, _, level, ext)| filename == format!("{numeric}_{level}.{ext}"))
        .map(|(_, _, level, _)| *level)
        .ok_or("无效的网易云音质文件名")?;
    let guest_cookie;
    let cookie = if client::cookie_value(cookie, "MUSIC_U").is_empty()
        && client::cookie_value(cookie, "MUSIC_A").is_empty()
    {
        guest_cookie = client::anonymous_cookie().await?;
        guest_cookie.as_str()
    } else {
        cookie
    };
    let mut params = json!({
        "ids": json!([numeric]).to_string(),
        "level": level,
        "encodeType": "flac"
    });
    if level == "sky" {
        params["immerseType"] = json!("c51");
    }
    let data = request("/api/song/enhance/player/url/v1", params, cookie, true)
        .await?
        .0;
    let item = list(&data["data"])
        .iter()
        .find(|v| v["id"].as_u64() == Some(numeric))
        .ok_or("网易云未返回歌曲音源")?;
    validate_source(item, level, filename)
}

fn validate_source(item: &Value, level: &str, filename: &str) -> Result<(String, String), String> {
    if item["freeTrialInfo"].is_object()
        || item["freeTrialPrivilege"]["listenType"].as_u64() == Some(1)
    {
        return Err("网易云仅返回试听片段，请使用有歌曲权限的账号".into());
    }
    let url = item["url"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or("网易云歌曲不可用或当前账号无权限")?;
    if item["level"].as_str() != Some(level) {
        return Err("网易云未提供所选音质，请选择其他音质或启用自动降级".into());
    }
    let format = item["type"].as_str().unwrap_or("").to_ascii_lowercase();
    if !filename.ends_with(&format!(".{format}")) || !matches!(format.as_str(), "mp3" | "flac") {
        return Err("网易云返回的音频格式与所选音质不符".into());
    }
    let parsed = reqwest::Url::parse(url).map_err(|_| "网易云音源链接无效")?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err("网易云音源链接无效".into());
    }
    Ok((url.to_string(), String::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_trial_and_silent_quality_downgrades() {
        let mut item = json!({
            "url": "https://m801.music.126.net/audio",
            "level": "exhigh",
            "type": "mp3"
        });
        assert!(validate_source(&item, "exhigh", "1_exhigh.mp3").is_ok());
        assert!(validate_source(&item, "hires", "1_hires.flac").is_err());
        item["freeTrialInfo"] = json!({
            "start": 0,
            "end": 30
        });
        assert!(validate_source(&item, "exhigh", "1_exhigh.mp3").is_err());
    }

    #[tokio::test]
    async fn invalid_filename_fails_before_request() {
        assert!(get_download_link("1", "2_standard.mp3", "").await.is_err());
    }
}
