//! Rust 直连汽水音乐（字节跳动 Soda Music）。
//!
//! 匿名可用的端点只有三组，会员曲目的整曲流需要官方客户端生成的
//! `x-helios` / `x-medusa` 签名头，这里不实现签名，拿不到时按试听片段处理：
//!
//! - `GET api.qishui.com/luna/pc/search/{track|playlist}`（`device_platform=web`）
//! - `GET api.qishui.com/luna/pc/track_v2`（`device_platform=web`，与 App 端 POST 同名但不是同一入口）
//! - `GET beta-luna.douyin.com/luna/h5/seo_track`（track_v2 的免签兜底）
//!
//! 带 `play_auth` 的流是加密 M4A，下载完成后由 [`crypto`] 原地解密。

pub mod client;
pub mod credentials;
pub mod crypto;
pub mod link;
pub mod lyrics;
pub mod parser;
pub mod search;
pub mod song;

use once_cell::sync::Lazy;
use regex::Regex;

/// 汽水 Web/PC 接口前缀，搜索与详情共用。
pub(crate) const API_BASE: &str = "https://api.qishui.com/luna";
/// 免签兜底接口，`track_v2` 失败或无法解析时使用。
pub(crate) const SEO_API: &str = "https://beta-luna.douyin.com/luna/h5/seo_track";
/// PC Web 与移动端共用的应用 ID。
pub(crate) const AID: &str = "386088";
/// 分享页与 H5 接口使用的浏览器 UA，与下载客户端保持一致。
pub(crate) const WEB_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
/// 移动端搜索兜底使用的 UA，参数与 UA 版本必须一致。
pub(crate) const ANDROID_UA: &str =
    "com.luna.music/100198030 (Linux; U; Android 15; zh_CN_#Hans; ABR-AL80; Build/V417IR;tt-ok/3.12.13.19)";

/// 曲目 ID 是 19 位数字，超出 JS 安全整数范围，全程按字符串保存。
static TRACK_ID_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?:^|[?&])track_id=(\d{10,})").expect("汽水 track_id 查询参数正则"),
        Regex::new(r#""track_id"\s*:\s*"?(\d{10,})"#).expect("汽水 track_id JSON 正则"),
        Regex::new(r"(?:/|%2F)(?:track|song)(?:/|%2F)(\d{10,})").expect("汽水曲目路径正则"),
    ]
});

/// 从分享文案、落地页 URL 或纯 ID 中取出曲目 ID。
///
/// 识别顺序与客户端一致：先认纯数字，再按查询参数、内嵌 JSON、路径依次匹配；
/// 命中不了说明还需要跟随短链跳转。
pub fn track_id_from_text(text: &str) -> Option<String> {
    let text = text.trim();
    if is_track_id(text) {
        return Some(text.to_string());
    }
    let decoded = percent_decode(text);
    for candidate in [text, decoded.as_str()] {
        for pattern in TRACK_ID_PATTERNS.iter() {
            if let Some(captured) = pattern.captures(candidate) {
                if let Some(value) = captured.get(1) {
                    return Some(value.as_str().to_string());
                }
            }
        }
    }
    None
}

/// 曲目 ID 只接受 10 位以上数字，避免把普通数字误判成 ID。
pub fn is_track_id(value: &str) -> bool {
    let value = value.trim();
    value.len() >= 10 && value.chars().all(|c| c.is_ascii_digit())
}

/// 分享文案里的链接可能带 `%2F`，解码后再匹配一次路径形式。
fn percent_decode(text: &str) -> String {
    if !text.contains('%') {
        return text.to_string();
    }
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
            {
                out.push(high * 16 + low);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_track_id, track_id_from_text};

    #[test]
    fn accepts_bare_track_ids() {
        assert_eq!(
            track_id_from_text("7512772704484722704"),
            Some("7512772704484722704".into())
        );
        assert!(is_track_id("7512772704484722704"));
        assert!(!is_track_id("12345"));
        assert!(!is_track_id("qishui"));
    }

    #[test]
    fn extracts_ids_from_share_links_and_share_text() {        assert_eq!(
            track_id_from_text(
                "https://music.douyin.com/qishui/share/track?track_id=7512772704484722704"
            ),
            Some("7512772704484722704".into())
        );
        assert_eq!(
            track_id_from_text("https://music.douyin.com/qishui/share/track/7512772704484722704"),
            Some("7512772704484722704".into())
        );
        assert_eq!(
            track_id_from_text(r#"<script>{"track_id":"7512772704484722704"}</script>"#),
            Some("7512772704484722704".into())
        );
        assert_eq!(
            track_id_from_text("分享一首歌给你 https://qishui.douyin.com/s/iX21ep91/"),
            None
        );
    }

    /// 手动运行：cargo test --manifest-path crates/hotdownloader-core/Cargo.toml live_track -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "需要连接汽水真实接口"]
    async fn live_track_detail() {
        let detail = super::song::fetch_song_detail("、", "7304719759323564095")
            .await
            .unwrap();
        println!("{detail}");
        let value: serde_json::Value = serde_json::from_str(&detail).unwrap();
        assert!(!value["title"].as_str().unwrap_or("").is_empty());
        assert_eq!(value["mid"], "7304719759323564095");
        assert!(
            !value["coverUrl"].as_str().unwrap_or("").is_empty(),
            "封面应能从 url_cover 对象里拼出来"
        );
        let lyric = super::lyrics::get_lyric("7304719759323564095")
            .await
            .unwrap();
        let lrc = lyric.lrc.unwrap_or_default();
        assert!(lrc.contains("[00:"), "歌词应已转成标准 LRC: {lrc}");
    }

    /// 手动运行：cargo test --manifest-path crates/hotdownloader-core/Cargo.toml live_share -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "需要连接汽水真实接口"]
    async fn live_share_link_resolves_to_song() {
        let link = crate::platforms::link_parse::resolve(
            "分享一首歌给你 https://qishui.douyin.com/s/iQeFw9cE/",
            None,
        )
        .await
        .unwrap();
        assert_eq!(link.platform, crate::platforms::Platform::Qishui);
        assert_eq!(link.kind, crate::platforms::link_parse::LinkKind::Song);
        let payload = crate::platforms::link_parse::fetch(&link, "、", "", None)
            .await
            .unwrap();
        println!("{payload}");
        assert!(payload.contains("\"song\""));
        assert!(payload.contains("\"mid\":\"7304719759323564095\""));
    }

    /// 手动运行：cargo test --manifest-path crates/hotdownloader-core/Cargo.toml live_download -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "需要连接汽水真实 CDN"]
    async fn live_download_link() {
        let (url, key) = super::link::get_download_link(
            &crate::platforms::CLIENT,
            "7304719759323564095",
            "128.m4a",
        )
        .await
        .unwrap();
        assert!(url.starts_with("https://"));
        println!("play_auth_len={}", key.len());
        let response = crate::platforms::CLIENT
            .get(&url)
            .header("Range", "bytes=0-31")
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success(), "{}", response.status());
        assert!(!response.bytes().await.unwrap().is_empty());
    }
}
