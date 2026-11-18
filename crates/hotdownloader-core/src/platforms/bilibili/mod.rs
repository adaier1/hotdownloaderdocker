//! Rust 直连 B 站：分享链接解析与音频流获取。
//!
//! 只服务「单曲」场景——B 站没有歌曲搜索与歌单概念（合集/收藏夹不在此范围），
//! 因此本模块不参与 `songs/search` 等查询，只提供视频详情与 dash 音频流。

mod client;
pub mod link;
pub mod parser;
pub mod song;

use once_cell::sync::Lazy;
use regex::Regex;

/// 视频详情接口，bv 号与 av 号都支持。
pub(crate) const VIEW_API: &str = "https://api.bilibili.com/x/web-interface/view";
/// 音频流接口，`fnval=16` 表示返回 dash 格式。
pub(crate) const PLAYURL_API: &str = "https://api.bilibili.com/x/player/playurl";

/// 视频号：BV 号（12 位）或 av 号。
static VIDEO_ID_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(BV[0-9A-Za-z]{10}|av\d+)\b").expect("B 站视频号正则"));

/// 从链接或纯文本里取出视频号，统一返回 BV 号形式。
pub fn video_id(text: &str) -> Option<String> {
    let matched = VIDEO_ID_RE.captures(text)?;
    let raw = matched.get(1)?.as_str();
    if raw.to_ascii_lowercase().starts_with("av") {
        // av 号需要一次接口换算，这里保留原样，由详情接口自行处理。
        return Some(raw.to_ascii_lowercase());
    }
    Some(raw.to_string())
}

/// 视频号是否合法：BV 号 12 位，或 `av` + 数字。
pub fn valid_video_id(id: &str) -> bool {
    let id = id.trim();
    if let Some(rest) = id.strip_prefix("av") {
        return !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit());
    }
    id.len() == 12
        && id.starts_with("BV")
        && id[2..].chars().all(|c| c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::{valid_video_id, video_id};

    #[test]
    fn extracts_bv_from_share_links() {
        assert_eq!(
            video_id("https://www.bilibili.com/video/BV1GJ411x7h7?share_source=copy"),
            Some("BV1GJ411x7h7".into())
        );
        assert_eq!(
            video_id("【官方 MV】Never Gonna Give You Up https://b23.tv/abc123"),
            None
        );
        assert_eq!(
            video_id("https://www.bilibili.com/video/av80433022"),
            Some("av80433022".into())
        );
    }

    #[test]
    fn validates_video_ids() {
        assert!(valid_video_id("BV1GJ411x7h7"));
        assert!(valid_video_id("av80433022"));
        assert!(!valid_video_id("BV123"));
        assert!(!valid_video_id("av"));
    }
}
