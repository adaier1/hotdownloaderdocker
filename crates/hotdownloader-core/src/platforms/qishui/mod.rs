//! Rust 直连汽水音乐（字节跳动 Soda Music）。
//!
//! 免签端点只给「浏览器级」能力，会员曲目的整曲流、`hi_res` 档位与搜索都需要官方客户端
//! 级别的应用签名（`x-helios` / `x-medusa`）：
//!
//! - `GET api.qishui.com/luna/pc/search/{track|playlist}`（`device_platform=web`，免签，实测会空响应）
//! - `GET api.qishui.com/luna/pc/track_v2`（`device_platform=web`，与 App 端 POST 同名但不是同一入口）
//! - `GET beta-luna.douyin.com/luna/h5/seo_track`（track_v2 的免签兜底）
//! - `POST api.qishui.com/luna/pc/track_v2`（App 端，需 Cookie + [`signer`]，见 [`app`]）
//!
//! 签名由用户机器上的汽水客户端 `bdms.node` 生成（[`signer`]），拿不到时自动退回免签端点，
//! 此时会员曲目只有试听片段。带 `play_auth` 的流是加密 M4A，下载完成后由 [`crypto`] 原地解密。

pub mod app;
pub mod client;
pub mod credentials;
pub mod crypto;
pub mod link;
pub mod lyrics;
pub mod parser;
pub mod search;
pub mod signer;
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

    /// 手动运行（需本机汽水客户端 + 已登录 Cookie）：
    /// `QISHUI_COOKIE_FILE=data/qishui-credentials.json cargo test ... live_signed_track -- --ignored --nocapture`
    #[tokio::test]
    #[ignore = "需要本机汽水客户端提供签名 + 已登录 Cookie"]
    async fn live_signed_track_has_full_variants() {
        let path = std::env::var("QISHUI_COOKIE_FILE").expect("需要 QISHUI_COOKIE_FILE 指向凭据文件");
        let raw = std::fs::read_to_string(path).expect("读取凭据文件失败");
        let cookie = serde_json::from_str::<serde_json::Value>(&raw).unwrap()["cookie"]
            .as_str()
            .unwrap_or("")
            .to_string();
        assert!(!cookie.is_empty(), "凭据文件里没有 cookie");
        super::credentials::set_cached_cookie(&cookie);

        let bundle = super::parser::fetch_track("7689447702921349121").await.unwrap();
        println!("signed={} players={}", bundle.signed, bundle.players.len());
        for player in &bundle.players {
            println!(
                "  {:?} {}kbps {}s preview={} size={} quality={} format={}",
                player.quality_label(),
                player.bitrate,
                player.duration,
                player.preview,
                player.size,
                player.quality,
                player.format
            );
        }
        assert!(bundle.signed, "应走 App 端签名链路");
        assert!(
            bundle
                .players
                .iter()
                .any(|player| !player.preview && player.duration > 180),
            "应拿到完整时长的变体"
        );
    }

    /// 手动运行（需本机汽水客户端 + 已登录 Cookie）：
    /// `QISHUI_COOKIE_FILE=... cargo test ... live_signed_search -- --ignored --nocapture`
    #[tokio::test]
    #[ignore = "需要本机汽水客户端提供签名 + 已登录 Cookie"]
    async fn live_signed_search_returns_tracks() {
        let path = std::env::var("QISHUI_COOKIE_FILE").expect("需要 QISHUI_COOKIE_FILE 指向凭据文件");
        let raw = std::fs::read_to_string(path).expect("读取凭据文件失败");
        let cookie = serde_json::from_str::<serde_json::Value>(&raw).unwrap()["cookie"]
            .as_str()
            .unwrap_or("")
            .to_string();
        super::credentials::set_cached_cookie(&cookie);

        let payload = super::search::search_songs("、", "周杰伦".to_string(), 1, 10)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&payload).unwrap();
        let songs = value["songs"].as_array().cloned().unwrap_or_default();
        println!("搜索结果 {} 条", songs.len());
        for song in songs.iter().take(3) {
            println!("  {} - {}", song["title"], song["artist"]);
        }
        assert!(!songs.is_empty(), "签名搜索应返回结果");
    }
}
