//! 试听链接复用平台解析与登录凭据，不创建下载任务或写入音频文件。

use std::path::Path;
use std::time::Duration;

use once_cell::sync::Lazy;
use reqwest::{Client, Response, Url};
use serde::{Deserialize, Serialize};

use crate::download::link::DownloadLinkProvider;
use crate::platforms::Platform;
use crate::task::contract::QualityItem;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackRequest {
    pub platform: String,
    pub song_mid: String,
    pub qualities: Vec<QualityItem>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSource {
    pub id: String,
    pub url: String,
    pub quality: String,
    pub content_type: String,
}

fn playable_type(filename: &str) -> Option<&'static str> {
    match Path::new(filename).extension()?.to_str()? {
        "mp3" => Some("audio/mpeg"),
        "m4a" => Some("audio/mp4"),
        "aac" => Some("audio/aac"),
        "ogg" => Some("audio/ogg"),
        "flac" => Some("audio/flac"),
        _ => None,
    }
}

/// 优先通用的 MP3，然后尝试其他明文格式；加密文件和 APE 不交给浏览器。
pub async fn resolve(
    provider: &dyn DownloadLinkProvider,
    request: PlaybackRequest,
) -> Result<PlaybackSource, String> {
    let platform = request.platform.parse::<Platform>()?;
    if request.song_mid.is_empty()
        || request.song_mid.len() > 128
        || !request
            .song_mid
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric())
        || (matches!(platform, Platform::Kuwo | Platform::Netease)
            && request.song_mid.parse::<u64>().unwrap_or(0) == 0)
        || request.qualities.len() > 64
    {
        return Err("无效的试听歌曲参数".into());
    }
    let order = [
        "320kmp3", "128kmp3", "192kaac", "96kaac", "48kaac", "300kogg", "192kogg", "100kogg",
        "96kogg", "flac",
    ];
    let mut qualities: Vec<_> = request
        .qualities
        .iter()
        .filter(|item| {
            item.size > 0
                && item.filename.len() <= 256
                && !item.filename.contains(['/', '\\'])
                && playable_type(&item.filename).is_some()
                && order.contains(&item.quality.as_str())
        })
        .collect();
    qualities.sort_by_key(|item| order.iter().position(|quality| *quality == item.quality));
    qualities.dedup_by(|a, b| a.filename == b.filename);
    let mut error = "该歌曲暂无可在线播放的音质".to_string();
    for quality in qualities {
        match provider
            .fetch(platform, &request.song_mid, &quality.filename)
            .await
        {
            Ok((url, key)) if key.is_empty() => {
                let parsed = Url::parse(&url).map_err(|_| "音源返回了无效的试听链接")?;
                if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
                    return Err("音源返回了无效的试听链接".into());
                }
                let id = rand::random::<[u8; 32]>()
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect();
                return Ok(PlaybackSource {
                    id,
                    url,
                    quality: quality.quality.clone(),
                    content_type: playable_type(&quality.filename).unwrap().into(),
                });
            }
            Ok(_) => error = "音源返回了加密音频，无法直接试听".into(),
            Err(reason) => error = format!("获取试听链接失败：{reason}"),
        }
    }
    Err(error)
}

static STREAM_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(30))
        .no_gzip()
        .build()
        .expect("Failed to create playback HTTP client")
});

/// Web 端流式转发，保留 Range 语义；超时按每次读取计算，允许播放较长音频。
pub async fn open_stream(
    source: &PlaybackSource,
    range: Option<&str>,
    head: bool,
) -> Result<Response, String> {
    let mut request = if head {
        STREAM_CLIENT.head(&source.url)
    } else {
        STREAM_CLIENT.get(&source.url)
    };
    request = request.header("Accept-Encoding", "identity");
    if let Some(range) = range {
        request = request.header("Range", range);
    }
    request
        .send()
        .await
        .map_err(|error| format!("读取试听音频失败: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::future::BoxFuture;
    use std::sync::Mutex;

    struct Links(Mutex<Vec<String>>);
    impl DownloadLinkProvider for Links {
        fn fetch<'a>(
            &'a self,
            _: Platform,
            _: &'a str,
            filename: &'a str,
        ) -> BoxFuture<'a, Result<(String, String), String>> {
            self.0.lock().unwrap().push(filename.into());
            Box::pin(async move {
                if filename == "320.mp3" {
                    Err("该音质不可用".into())
                } else {
                    Ok(("https://cdn.example/audio".into(), String::new()))
                }
            })
        }
    }

    fn request() -> PlaybackRequest {
        PlaybackRequest {
            platform: "kuwo".into(),
            song_mid: "123".into(),
            qualities: [
                ("flac", "2000.mflac"),
                ("128kmp3", "128.mp3"),
                ("320kmp3", "320.mp3"),
            ]
            .into_iter()
            .map(|(quality, filename)| QualityItem {
                quality: quality.into(),
                filename: filename.into(),
                size: 123,
            })
            .collect(),
        }
    }

    #[tokio::test]
    async fn mp3_fallback_skips_encrypted_audio_and_keeps_session_ids_unique() {
        let links = Links(Mutex::new(Vec::new()));
        let source = resolve(&links, request()).await.unwrap();
        assert_eq!(source.quality, "128kmp3");
        assert_eq!(source.content_type, "audio/mpeg");
        assert_eq!(*links.0.lock().unwrap(), ["320.mp3", "128.mp3"]);
        assert_eq!(source.id.len(), 64);
        assert_ne!(source.id, resolve(&links, request()).await.unwrap().id);
    }

    #[tokio::test]
    async fn invalid_songs_and_unplayable_formats_do_not_request_links() {
        let links = Links(Mutex::new(Vec::new()));
        let mut input = request();
        input.song_mid = "../123".into();
        assert!(resolve(&links, input).await.is_err());
        let mut input = request();
        input
            .qualities
            .retain(|quality| quality.filename.ends_with("mflac"));
        assert!(resolve(&links, input).await.is_err());
        assert!(links.0.lock().unwrap().is_empty());
    }
}
