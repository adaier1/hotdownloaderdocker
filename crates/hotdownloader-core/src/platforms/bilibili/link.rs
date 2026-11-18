//! 下载链接：用文件名里编码的 bvid / cid / 音频流 id 重新换取 dash 音频地址。

use super::client;

/// 文件名约定 `{bvid}_{cid}_{音频流 id}.m4a`，与酷狗一致——任务契约无需新增字段。
pub async fn get_download_link(
    _client: &reqwest::Client,
    _song_mid: &str,
    filename: &str,
) -> Result<(String, String), String> {
    let (bvid, cid, audio_id) = parse_filename(filename)?;
    let url = format!(
        "{}?bvid={bvid}&cid={cid}&fnval=16&fourk=1",
        super::PLAYURL_API
    );
    let data = client::get_json(&url).await?;
    let audio = data["dash"]["audio"]
        .as_array()
        .ok_or("B 站未返回音频流（可能是会员专享或已下架）")?;
    // 优先选请求的那一档，取不到就退到任意可用音频流。
    let item = audio
        .iter()
        .find(|item| item["id"].as_u64() == Some(audio_id))
        .or_else(|| audio.first())
        .ok_or("B 站无可下载音频流")?;
    let address = item["baseUrl"]
        .as_str()
        .or_else(|| item["base_url"].as_str())
        .filter(|value: &&str| !value.is_empty())
        .ok_or("B 站音频流地址为空")?;
    Ok((address.to_string(), String::new()))
}

fn parse_filename(filename: &str) -> Result<(String, u64, u64), String> {
    let stem = filename
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(filename);
    let parts: Vec<&str> = stem.split('_').collect();
    if parts.len() != 3 {
        return Err("无效的 B 站音源文件名".into());
    }
    let cid: u64 = parts[1].parse().map_err(|_| "无效的 B 站分P ID")?;
    let audio_id: u64 = parts[2].parse().map_err(|_| "无效的 B 站音频流 ID")?;
    if parts[0].is_empty() || cid == 0 || audio_id == 0 {
        return Err("无效的 B 站音源标识".into());
    }
    Ok((parts[0].to_string(), cid, audio_id))
}

/// 供下载引擎按地址补 Referer 时判断域名。
pub fn is_stream_host(url: &str) -> bool {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_ascii_lowercase))
        .is_some_and(|host| {
            host.ends_with("bilivideo.com") || host.ends_with("bilibili.com")
        })
}

#[cfg(test)]
mod tests {
    use super::parse_filename;

    #[test]
    fn filename_carries_video_and_audio_id() {
        assert_eq!(
            parse_filename("BV1GJ411x7h7_137649199_30280.m4a").unwrap(),
            ("BV1GJ411x7h7".to_string(), 137649199, 30280)
        );
        assert!(parse_filename("bad.m4a").is_err());
        assert!(parse_filename("BV1_0_30280.m4a").is_err());
    }
}
