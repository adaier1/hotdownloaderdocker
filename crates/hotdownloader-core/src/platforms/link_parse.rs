//! 分享链接解析：从用户粘贴的整段分享文案里挑出链接，判断平台与内容类型。
//!
//! 识别完全离线进行，只按 host 精确匹配（不接受「文本中包含某域名」这种宽松判断），
//! 避免把普通文字误判成平台链接。短链（如 `163cn.tv`）离线无法得知指向歌单还是
//! 单曲，这里归为 [`LinkKind::Unknown`]，交给平台自身的歌单解析去展开。

use std::time::Duration;

use once_cell::sync::Lazy;
use regex::Regex;

use super::{bilibili, kugou, kuwo, netease, qishui, qqmusic, Platform};

/// 分享链接指向的内容类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    /// 单曲。
    Song,
    /// 歌单。
    Playlist,
    /// 专辑。
    Album,
    /// 短链或无法离线判断，交给歌单解析兜底。
    Unknown,
}

impl LinkKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Song => "song",
            Self::Playlist => "playlist",
            Self::Album => "album",
            Self::Unknown => "unknown",
        }
    }
}

/// 识别结果。平台与内容类型都来自链接本身，不依赖用户手选。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLink {
    pub platform: Platform,
    pub kind: LinkKind,
    /// 原始链接。歌单解析直接复用，短链展开由平台实现负责。
    pub url: String,
}

/// 分享文案里常混着中文标点和引号，链接在它们之前结束。
static URL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"https?://[^\s<>"'）】」》，。！？、]+"#).expect("分享链接正则编译失败")
});

const UNSUPPORTED: &str = "暂不支持该链接，目前支持网易云、QQ 音乐、酷狗、酷我、汽水";

/// 从整段分享文案中取出第一个链接。
pub fn extract_url(text: &str) -> Option<&str> {
    URL_RE.find(text).map(|matched| matched.as_str())
}

/// 按 host 精确匹配平台；子域名前缀（如 `t1.kugou.com`）同样命中。
pub fn detect_platform(url: &str) -> Option<Platform> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let host = normalize_host(parsed.host_str()?);
    let bare = host.strip_prefix("www.").unwrap_or(&host);
    Some(match bare {
        "music.163.com" | "y.music.163.com" | "163cn.tv" | "163cn.com" => Platform::Netease,
        _ if host.ends_with("y.qq.com") => Platform::QqMusic,
        "kugou.com" => Platform::Kugou,
        "kuwo.cn" => Platform::Kuwo,
        _ if host.ends_with(".kugou.com") => Platform::Kugou,
        _ if host.ends_with(".kuwo.cn") => Platform::Kuwo,
        _ if host.ends_with("bilibili.com") || host == "b23.tv" => Platform::Bilibili,
        // 汽水的分享短链与抖音主站同域，只认 qishui 子域和 /qishui/ 分享页。
        _ if bare == "qishui.douyin.com" => Platform::Qishui,
        _ if bare == "music.douyin.com" && url.to_ascii_lowercase().contains("/qishui/") => {
            Platform::Qishui
        }
        _ => return None,
    })
}

/// 根据路径与 fragment 判断内容类型。网易云把路由放在 `#` 之后，需要一并看。
pub fn detect_kind(platform: Platform, url: &str) -> LinkKind {
    let Some((host, route)) = route_of(url) else {
        return LinkKind::Unknown;
    };
    match platform {
        Platform::Netease => {
            // 短链只能靠展开后才知道类型，交给歌单路径处理。
            if host.ends_with("163cn.tv") || host.ends_with("163cn.com") {
                return LinkKind::Unknown;
            }
            classify(&route, &["/playlist"], &["/album"], &["/song"])
        }
        Platform::QqMusic => classify(
            &route,
            &["playlist", "taoge"],
            &["albumdetail"],
            &["songdetail", "songmid"],
        ),
        Platform::Kugou => classify(
            &route,
            &["/special/single", "global_collection_id"],
            &["/album/"],
            &["/song/", "/mixsong/"],
        ),
        Platform::Kuwo => classify(
            &route,
            &["/playlist_detail", "/playlist/"],
            &["/album_detail", "/album/"],
            &["/play_detail", "/play/"],
        ),
        // B 站只有视频：BV 号本身就是单曲，短链（b23.tv）展开后同样是视频。
        Platform::Bilibili => LinkKind::Song,
        // 汽水分享短链离线无法区分单曲与歌单，统一按单曲处理：
        // 单曲占分享的绝大多数，歌单会在取 ID 时给出明确错误。
        Platform::Qishui => {
            if route.contains("/share/playlist") || route.contains("playlist_id") {
                LinkKind::Playlist
            } else if route.contains("/share/album") {
                LinkKind::Album
            } else {
                LinkKind::Song
            }
        }
    }
}

/// 解析用户输入：有链接时自动识别，只有 ID 时使用界面选定的平台。
pub fn parse(text: &str, fallback_platform: Option<Platform>) -> Result<ParsedLink, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("请输入分享链接或歌单 ID".into());
    }
    match extract_url(text) {
        Some(url) => {
            let platform = detect_platform(url).ok_or_else(|| UNSUPPORTED.to_string())?;
            let kind = detect_kind(platform, url);
            Ok(ParsedLink {
                platform,
                kind,
                url: url.to_owned(),
            })
        }
        None => {
            let platform = fallback_platform.ok_or_else(|| {
                "没有识别到链接。请粘贴完整分享链接，或选择平台后输入歌单 ID".to_string()
            })?;
            Ok(ParsedLink {
                platform,
                kind: LinkKind::Playlist,
                url: text.to_owned(),
            })
        }
    }
}

/// 取回解析结果，返回平台原始 JSON（歌单为 `{"playlist":…,"songs":…}`，专辑为 `{"album":…,"songs":…}`）。
pub async fn fetch(
    link: &ParsedLink,
    separator: &str,
    netease_cookie: &str,
    qq_auth: Option<&super::qqmusic::credentials::QqAuth>,
) -> Result<String, String> {
    match link.kind {
        LinkKind::Song => fetch_song(link, separator, netease_cookie, qq_auth).await,
        LinkKind::Album => {
            let id = album_id(link.platform, &link.url)
                .ok_or_else(|| "未能从专辑链接中提取专辑 ID".to_string())?;
            match link.platform {
                Platform::Netease => {
                    netease::album::fetch_album_songs(separator, id, netease_cookie).await
                }
                Platform::QqMusic => qqmusic::album::fetch_album_songs(separator, id).await,
                Platform::Kugou => kugou::album::fetch_album_songs(separator, id).await,
                Platform::Kuwo => kuwo::album::fetch_album_songs(separator, id).await,
                // B 站没有专辑概念。
                Platform::Bilibili => Err("B 站链接不支持专辑解析".to_string()),
                Platform::Qishui => Err("汽水链接暂不支持专辑解析".to_string()),
            }
        }
        // 歌单接口本身就接受链接，短链展开也由它内部完成。
        LinkKind::Playlist => fetch_playlist(link, separator, netease_cookie).await,
        // 各端分享格式差异大（PC 客户端、旧版 H5、短链跳转后仍不认识路径）：
        // 只要还能提取出单曲标识，就按单曲处理，不因为路径陌生就判成歌单。
        LinkKind::Unknown => {
            if song_id(link.platform, &link.url).is_some() {
                return fetch_song(link, separator, netease_cookie, qq_auth).await;
            }
            fetch_playlist(link, separator, netease_cookie)
                .await
                .map_err(|error| {
                    // 平台给出的明确原因（如「B 站链接不支持歌单解析」）比通用文案有用。
                    if error.contains("不支持") {
                        error
                    } else {
                        "该链接无法解析：请用客户端「分享 → 复制链接」的完整分享内容重试"
                            .to_string()
                    }
                })
        }
    }
}

/// 单曲解析：从链接取标识，再交给对应平台的详情接口。
async fn fetch_song(
    link: &ParsedLink,
    separator: &str,
    netease_cookie: &str,
    qq_auth: Option<&super::qqmusic::credentials::QqAuth>,
) -> Result<String, String> {
    // 汽水短链需要跟随跳转并从落地页取 ID，其余平台的标识都能离线解析。
    let id = match link.platform {
        Platform::Qishui => qishui::song::resolve_track_id(&link.url).await?,
        _ => song_id(link.platform, &link.url)
            .ok_or_else(|| "未能从歌曲链接中提取歌曲 ID".to_string())?,
    };
    // QQ 分享链接里常同时出现歌曲、专辑、MV 等多个 14 位串，取错会得到 code=400；
    // 按可信度逐个尝试，第一个能解析的即采用。
    let candidates = match link.platform {
        Platform::QqMusic => qq_songmid_candidates(&link.url, &id),
        _ => vec![id],
    };
    let mut last_error = String::new();
    for candidate in candidates {
        let result = match link.platform {
            Platform::Netease => {
                netease::song::fetch_song_detail(separator, &candidate, netease_cookie).await
            }
            Platform::QqMusic => {
                // 纯数字的是 songid，其余按 songmid 查询。
                if is_numeric_id(&candidate) {
                    match candidate.parse::<u64>() {
                        Ok(song_id) => {
                            qqmusic::song::fetch_song_detail_by_id(separator, song_id, qq_auth)
                                .await
                        }
                        Err(_) => Err("无效的 QQ 歌曲 ID".to_string()),
                    }
                } else {
                    qqmusic::song::fetch_song_detail(separator, &candidate, qq_auth).await
                }
            }
            Platform::Kugou => kugou::song::fetch_song_detail(separator, &candidate).await,
            Platform::Kuwo => kuwo::song::fetch_song_detail(separator, &candidate).await,
            Platform::Bilibili => bilibili::song::fetch_song_detail(&candidate).await,
            Platform::Qishui => qishui::song::fetch_song_detail(separator, &candidate).await,
        };
        match result {
            Ok(raw) => return finalize_song(link, &raw),
            Err(error) => last_error = error,
        }
    }
    Err(last_error)
}

/// 与歌单 / 专辑保持同一层结构，前端按 kind 取对应字段即可。
fn finalize_song(link: &ParsedLink, raw: &str) -> Result<String, String> {
    let mut song: serde_json::Value =
        serde_json::from_str(raw).map_err(|error| format!("解析单曲详情失败: {error}"))?;
    // 平台标识由识别结果补齐，调用方拿到的对象可直接用于创建下载任务。
    if let Some(object) = song.as_object_mut() {
        object.insert(
            "platform".into(),
            serde_json::json!(link.platform.as_str()),
        );
    }
    Ok(serde_json::json!({ "song": song }).to_string())
}

/// 收集链接里所有像 songmid 的标识：查询串里的 `songmid` 最可信，其次是路径中的 14 位串。
fn qq_songmid_candidates(url: &str, primary: &str) -> Vec<String> {
    const LIMIT: usize = 3;
    let mut list = vec![primary.to_string()];
    let Ok(parsed) = reqwest::Url::parse(url) else {
        return list;
    };
    let push = |list: &mut Vec<String>, value: &str| {
        if list.len() < LIMIT && is_songmid(value) && !list.iter().any(|item| item == value) {
            list.push(value.to_string());
        }
    };
    for (key, value) in parsed.query_pairs() {
        if key.eq_ignore_ascii_case("songmid") {
            push(&mut list, &value);
        }
    }
    for segment in parsed.path().split('/') {
        push(&mut list, segment);
    }
    list
}

async fn fetch_playlist(
    link: &ParsedLink,
    separator: &str,
    netease_cookie: &str,
) -> Result<String, String> {
    let url = link.url.clone();
    match link.platform {
        Platform::Netease => {
            netease::playlist::fetch_playlist_songs(separator, url, netease_cookie).await
        }
        Platform::QqMusic => qqmusic::playlist::fetch_playlist_songs(separator, url).await,
        Platform::Kugou => kugou::playlist::fetch_playlist_songs(separator, url).await,
        Platform::Kuwo => kuwo::playlist::fetch_playlist_songs(separator, url).await,
        // B 站的合集/收藏夹不在支持范围。
        Platform::Bilibili => Err("B 站链接不支持歌单解析".to_string()),
        // 汽水歌单分享链接当前不展开，只支持单曲。
        Platform::Qishui => Err("汽水链接暂不支持歌单解析".to_string()),
    }
}

fn is_hash(value: &str) -> bool {
    value.len() == 32 && value.chars().all(|c| c.is_ascii_hexdigit())
}

/// 酷狗的歌曲标识是 32 位 hash，可能在查询串、fragment（`#hash=xxx`）或路径里。
fn hash_of(parsed: &reqwest::Url) -> Option<String> {
    for raw in [parsed.query(), parsed.fragment()].into_iter().flatten() {
        for pair in raw.split('&') {
            if let Some(value) = pair.strip_prefix("hash=") {
                let value = value.to_lowercase();
                if is_hash(&value) {
                    return Some(value);
                }
            }
        }
    }
    // PC 客户端分享的是 `/mixsong/<hash>.html`，路径段带扩展名，先剥掉再判断。
    parsed
        .path()
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_lowercase())
        .map(|segment| match segment.rsplit_once('.') {
            Some((stem, _)) => stem.to_string(),
            None => segment,
        })
        .find(|stem| is_hash(stem))
}

/// 短链域名：仅当类型无法离线判断时才展开，避免对普通链接多发一次请求。
const SHORT_LINK_HOSTS: &[&str] = &[
    "163cn.tv",
    "163cn.com",
    "t1.kugou.com",
    "c6.y.qq.com",
    "c.y.qq.com",
    "b23.tv",
];

fn is_short_link(url: &str) -> bool {
    route_of(url).is_some_and(|(host, _)| {
        SHORT_LINK_HOSTS
            .iter()
            .any(|allowed| host == *allowed || host.ends_with(&format!(".{allowed}")))
    })
}

/// 解析入口：类型未知的短链先跟随跳转拿到真实链接，再重新识别一次。
///
/// 展开失败时保留原判断，交给平台自己的歌单解析兜底，不把链接直接判成错误。
pub async fn resolve(text: &str, fallback: Option<Platform>) -> Result<ParsedLink, String> {
    let link = parse(text, fallback)?;
    if link.kind != LinkKind::Unknown || !is_short_link(&link.url) {
        return Ok(link);
    }
    let Some(expanded) = expand_short_link(&link.url).await else {
        return Ok(link);
    };
    if expanded == link.url {
        return Ok(link);
    }
    let Some(platform) = detect_platform(&expanded) else {
        return Ok(link);
    };
    Ok(ParsedLink {
        platform,
        kind: detect_kind(platform, &expanded),
        url: expanded,
    })
}

/// 跟随短链跳转。限制跳数与目标域名，且不携带任何账号凭据。
async fn expand_short_link(url: &str) -> Option<String> {
    let response = SHORT_LINK_CLIENT.get(url).send().await.ok()?;
    Some(response.url().to_string())
}

fn allowed_redirect(url: &reqwest::Url) -> bool {
    matches!(url.scheme(), "https" | "http")
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && url.host_str().map(normalize_host).is_some_and(|host| {
            [
                "163.com",
                "163cn.tv",
                "163cn.com",
                "qq.com",
                "kugou.com",
                "kuwo.cn",
                "bilibili.com",
                "b23.tv",
                "douyin.com",
            ]
            .iter()
            .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
        })
}

static SHORT_LINK_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() < 5 && allowed_redirect(attempt.url()) {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .expect("分享短链客户端初始化失败")
});

fn classify(route: &str, playlist: &[&str], album: &[&str], song: &[&str]) -> LinkKind {
    if playlist.iter().any(|marker| route.contains(marker)) {
        LinkKind::Playlist
    } else if album.iter().any(|marker| route.contains(marker)) {
        LinkKind::Album
    } else if song.iter().any(|marker| route.contains(marker)) {
        LinkKind::Song
    } else {
        LinkKind::Unknown
    }
}

fn normalize_host(host: &str) -> String {
    host.trim_end_matches('.').to_ascii_lowercase()
}

/// 返回 (host, 归一化路由)。路由包含查询串与 fragment：
/// 网易云把 `#/playlist?id=` 整个放在 fragment 里，酷狗的歌单标识在查询串里。
fn route_of(url: &str) -> Option<(String, String)> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let host = normalize_host(parsed.host_str()?);
    let path = parsed.path().to_ascii_lowercase();
    let query = parsed.query().unwrap_or_default().to_ascii_lowercase();
    let fragment = parsed.fragment().unwrap_or_default().to_ascii_lowercase();
    let mut route = path;
    if !query.is_empty() {
        route.push('?');
        route.push_str(&query);
    }
    if !fragment.is_empty() {
        route.push('#');
        route.push_str(&fragment);
    }
    Some((host, route))
}

/// 网易云详情页把真实路由放在 `#` 之后，需要重新解析才能拿到 `id` 参数。
fn fragment_url(parsed: &reqwest::Url) -> Option<reqwest::Url> {
    let fragment = parsed.fragment()?;
    reqwest::Url::parse(&format!("https://{}/{}", parsed.host_str()?, fragment)).ok()
}

/// 单曲接口只接受歌曲自身标识（数字 ID、songmid 或 hash），从链接里取出来。
fn song_id(platform: Platform, url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let path = parsed.path().trim_end_matches('/');
    match platform {
        Platform::Netease => {
            let detail = fragment_url(&parsed).unwrap_or_else(|| parsed.clone());
            detail
                .query_pairs()
                .find(|(key, _)| key == "id")
                .map(|(_, value)| value.into_owned())
                .or_else(|| trailing_id(detail.path().trim_end_matches('/')))
        }
        Platform::QqMusic => parsed
            .query_pairs()
            .find(|(key, _)| key.eq_ignore_ascii_case("songmid"))
            .map(|(_, value)| value.into_owned())
            // 分享文案里可能带 `songmid=` 但值其实是别的 ID，只认合法 mid。
            .filter(|value| is_songmid(value))
            // 电脑端短链展开后是 `playsong/index.html?songid=599234991`，只有数字 ID。
            .or_else(|| {
                parsed
                    .query_pairs()
                    .find(|(key, _)| key.eq_ignore_ascii_case("songid"))
                    .map(|(_, value)| value.into_owned())
                    .filter(|value| is_numeric_id(value))
            })
            .or_else(|| segment_after(path, "songdetail"))
            // PC 端还有 `/share/song/<songmid>` 这类路径，末段就是 songmid。
            .or_else(|| {
                path.rsplit('/')
                    .find(|segment| is_songmid(segment))
                    .map(str::to_owned)
            }),
        Platform::Kugou => hash_of(&parsed),
        Platform::Kuwo => segment_after(path, "play_detail")
            .or_else(|| segment_after(path, "play/detail"))
            .or_else(|| trailing_id(path)),
        // B 站的标识是 BV / av 号，可能出现在路径里，也可能只是整段文案中的一段。
        Platform::Bilibili => super::bilibili::video_id(url),
        // 汽水只用 19 位曲目 ID；短链需要联网展开，由 `fetch_song` 单独处理。
        Platform::Qishui => super::qishui::track_id_from_text(url),
    }
}

/// 专辑接口只接受 ID，需要先从链接里取出来。
fn album_id(platform: Platform, url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url).ok()?;
    match platform {
        Platform::Netease => {
            let detail = fragment_url(&parsed).unwrap_or_else(|| parsed.clone());
            detail
                .query_pairs()
                .find(|(key, _)| key == "id")
                .map(|(_, value)| value.into_owned())
                .or_else(|| trailing_id(detail.path().trim_end_matches('/')))
        }
        _ => {
            let path = parsed.path().trim_end_matches('/');
            match platform {
                Platform::QqMusic => segment_after(path, "albumdetail"),
                Platform::Kugou => segment_after(path, "/album/"),
                Platform::Kuwo => segment_after(path, "album_detail"),
                Platform::Netease => unreachable!("网易云已在上一分支处理"),
                Platform::Bilibili => None,
                Platform::Qishui => None,
            }
        }
    }
}

/// 取标记之后的第一段；QQ 的 `albumDetail` 大小写混合，所以按小写定位、按原串切片。
fn segment_after(path: &str, marker: &str) -> Option<String> {
    let lower = path.to_ascii_lowercase();
    let start = lower.find(marker)? + marker.len();
    let rest = path[start..].trim_start_matches('/');
    rest.split('/')
        .next()
        .map(|segment| segment.trim_end_matches(".html").to_owned())
        .filter(|segment| !segment.is_empty())
}

/// 末段纯数字（如 `/album/123`、`/album_detail/123`）。
fn trailing_id(path: &str) -> Option<String> {
    path.rsplit('/')
        .find(|segment| !segment.is_empty() && segment.chars().all(|c| c.is_ascii_digit()))
        .map(|segment| segment.to_owned())
}

/// QQ 的 songmid 是 14 位字母数字混合串（如 `0039MnYb0qxYhV`），纯数字的不是。
fn is_songmid(segment: &str) -> bool {
    segment.len() == 14
        && segment.chars().all(|c| c.is_ascii_alphanumeric())
        && segment.chars().any(|c| c.is_ascii_alphabetic())
}

/// 纯数字 ID（QQ 的 songid、酷我的 rid 等）。
fn is_numeric_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 20 && value.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_link_from_share_copy() {
        let text = "分享TaylorSwift的单曲《Love Story》: https://y.music.163.com/m/song?id=1 （来自@网易云音乐）";
        assert_eq!(
            extract_url(text),
            Some("https://y.music.163.com/m/song?id=1")
        );
    }

    #[test]
    fn ignores_plain_text_without_link() {
        assert!(extract_url("周杰伦 晴天").is_none());
    }

    #[test]
    fn detects_platform_by_host_not_by_substring() {
        assert_eq!(
            detect_platform("https://music.163.com/playlist?id=1"),
            Some(Platform::Netease)
        );
        assert_eq!(
            detect_platform("https://t1.kugou.com/song.html?id=1"),
            Some(Platform::Kugou)
        );
        assert_eq!(
            detect_platform("https://www.kuwo.cn/playlist_detail/1"),
            Some(Platform::Kuwo)
        );
        // 域名只出现在路径或查询串里不算，避免把普通文字误判成平台链接。
        assert_eq!(detect_platform("https://example.com/?from=163.com"), None);
        assert_eq!(detect_platform("https://fakekugou.com/song/1"), None);
    }

    #[test]
    fn classifies_netease_fragment_routes() {
        let kind = |url| detect_kind(Platform::Netease, url);
        assert_eq!(kind("https://music.163.com/#/playlist?id=1"), LinkKind::Playlist);
        assert_eq!(kind("https://music.163.com/playlist?id=1"), LinkKind::Playlist);
        assert_eq!(kind("https://music.163.com/#/album?id=1"), LinkKind::Album);
        assert_eq!(kind("https://music.163.com/song?id=1"), LinkKind::Song);
        assert_eq!(kind("https://163cn.tv/abc"), LinkKind::Unknown);
    }

    #[test]
    fn classifies_other_platforms() {
        assert_eq!(
            detect_kind(Platform::QqMusic, "https://y.qq.com/n/ryqq/playlist/8123"),
            LinkKind::Playlist
        );
        assert_eq!(
            detect_kind(Platform::QqMusic, "https://y.qq.com/n/ryqq/albumDetail/0025NhlN2yWrP4"),
            LinkKind::Album
        );
        assert_eq!(
            detect_kind(Platform::Kugou, "https://www.kugou.com/album/123.html"),
            LinkKind::Album
        );
        assert_eq!(
            detect_kind(
                Platform::Kugou,
                "https://m.kugou.com/share/?global_collection_id=collection_3_1_2_0"
            ),
            LinkKind::Playlist
        );
        assert_eq!(
            detect_kind(Platform::Kuwo, "https://www.kuwo.cn/play_detail/123"),
            LinkKind::Song
        );
    }

    #[test]
    fn album_ids_are_extracted_per_platform() {
        assert_eq!(
            album_id(Platform::Netease, "https://music.163.com/#/album?id=35000000"),
            Some("35000000".into())
        );
        assert_eq!(
            album_id(Platform::Netease, "https://music.163.com/album/35000000"),
            Some("35000000".into())
        );
        assert_eq!(
            album_id(
                Platform::QqMusic,
                "https://y.qq.com/n/ryqq/albumDetail/0025NhlN2yWrP4"
            ),
            Some("0025NhlN2yWrP4".into())
        );
        assert_eq!(
            album_id(Platform::Kugou, "https://www.kugou.com/album/123.html"),
            Some("123".into())
        );
        assert_eq!(
            album_id(Platform::Kuwo, "https://www.kuwo.cn/album_detail/456"),
            Some("456".into())
        );
    }

    #[test]
    fn song_ids_are_extracted_per_platform() {
        let id = |platform, url| song_id(platform, url);
        assert_eq!(
            id(Platform::Netease, "https://music.163.com/song?id=1901371647"),
            Some("1901371647".into())
        );
        assert_eq!(
            id(Platform::Netease, "https://music.163.com/#/song?id=1901371647"),
            Some("1901371647".into())
        );
        assert_eq!(
            id(
                Platform::QqMusic,
                "https://y.qq.com/n/ryqq/songDetail/0039MnYb0qxYhV"
            ),
            Some("0039MnYb0qxYhV".into())
        );
        assert_eq!(
            id(
                Platform::Kugou,
                "https://www.kugou.com/song/#hash=0123456789abcdef0123456789abcdef&album_id=123"
            ),
            Some("0123456789abcdef0123456789abcdef".into())
        );
        assert_eq!(
            id(Platform::Kuwo, "https://www.kuwo.cn/play_detail/123"),
            Some("123".into())
        );
    }

    #[test]
    fn song_links_are_classified_as_song() {
        assert_eq!(
            detect_kind(Platform::Netease, "https://music.163.com/#/song?id=1"),
            LinkKind::Song
        );
        assert_eq!(
            detect_kind(
                Platform::Kugou,
                "https://www.kugou.com/song/#hash=0123456789abcdef0123456789abcdef"
            ),
            LinkKind::Song
        );
    }

    #[test]
    fn short_link_hosts_are_recognized() {
        assert!(is_short_link("https://163cn.tv/abc"));
        assert!(is_short_link("https://t1.kugou.com/song.html?id=1"));
        assert!(is_short_link("https://c6.y.qq.com/base/fcgi-bin/u?__=abc"));
        assert!(!is_short_link("https://music.163.com/song?id=1"));
    }

    #[test]
    fn falls_back_to_selected_platform_for_bare_ids() {
        let parsed = parse(" 8123 ", Some(Platform::QqMusic)).unwrap();
        assert_eq!(parsed.platform, Platform::QqMusic);
        assert_eq!(parsed.kind, LinkKind::Playlist);
        assert_eq!(parsed.url, "8123");
    }

    #[test]
    fn rejects_bare_ids_without_platform() {
        let error = parse("8123", None).unwrap_err();
        assert!(error.contains("选择平台"), "{error}");
    }

    #[test]
    fn rejects_unsupported_hosts() {
        let error = parse("https://example.com/song/1", None).unwrap_err();
        assert_eq!(error, UNSUPPORTED);
    }
}
