//! 歌单搜索、分享链接导入及当前用户创建的歌单。
use super::{
    client::{api, validate_id},
    parser::playlist,
    search, song,
};
use serde_json::json;

fn playlist_id(input: &str) -> Result<u64, String> {
    if let Ok(value) = validate_id(input.trim()) {
        return Ok(value);
    }
    // Extract only known playlist URL forms; arbitrary URLs are never fetched.
    let re = regex::Regex::new(r#"https?://(?:y\.)?music\.163\.com/[^\s<>"']+"#).unwrap();
    for matched in re.find_iter(input) {
        let url = reqwest::Url::parse(matched.as_str()).map_err(|_| "网易云歌单链接无效")?;
        let route = if let Some(fragment) = url.fragment() {
            reqwest::Url::parse(&format!("https://music.163.com/{fragment}"))
                .map_err(|_| "网易云歌单链接无效")?
        } else {
            url
        };
        if route.path().trim_end_matches('/').ends_with("/playlist") {
            if let Some((_, value)) = route.query_pairs().find(|(key, _)| key == "id") {
                return validate_id(&value);
            }
        }
        if let Some(value) = route.path().strip_prefix("/playlist/") {
            return validate_id(value.trim_end_matches('/'));
        }
    }
    Err("请输入网易云歌单 ID 或 music.163.com 歌单完整链接".into())
}

async fn resolve_playlist_id(input: &str) -> Result<u64, String> {
    if let Ok(value) = playlist_id(input) {
        return Ok(value);
    }
    static SHORT_CLIENT: once_cell::sync::Lazy<reqwest::Client> =
        once_cell::sync::Lazy::new(|| {
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .redirect(reqwest::redirect::Policy::custom(|attempt| {
                    let url = attempt.url();
                    if attempt.previous().len() < 5
                        && matches!(url.scheme(), "https" | "http")
                        && matches!(
                            url.host_str(),
                            Some("163cn.tv" | "163cn.com" | "music.163.com" | "y.music.163.com")
                        )
                        && url.username().is_empty()
                        && url.password().is_none()
                        && url.port().is_none()
                    {
                        attempt.follow()
                    } else {
                        attempt.stop()
                    }
                }))
                .build()
                .expect("网易云短链接客户端初始化失败")
        });
    let pattern =
        regex::Regex::new(r"https?://(?:163cn\.tv|163cn\.com)/[A-Za-z0-9/_?=&%-]+").unwrap();
    if let Some(short) = pattern.find(input) {
        // No account Cookie is sent while expanding share links.
        let response = SHORT_CLIENT
            .get(short.as_str())
            .send()
            .await
            .map_err(|e| format!("解析网易云短链接失败: {e}"))?;
        return playlist_id(response.url().as_str());
    }
    Err("请输入网易云歌单 ID 或歌单分享链接".into())
}

pub async fn search_playlists(
    keyword: String,
    page: u32,
    limit: u32,
    cookie: &str,
) -> Result<String, String> {
    let (rows, has_more) = search::search_page(
        &keyword,
        page,
        limit,
        1000,
        "playlists",
        "playlistCount",
        cookie,
    )
    .await?;
    let playlists: Vec<_> = rows.iter().map(playlist).collect();
    Ok(json!({
        "playlists": playlists,
        "has_more": has_more
    })
    .to_string())
}

pub async fn fetch_playlist_songs(
    separator: &str,
    input: String,
    cookie: &str,
) -> Result<String, String> {
    let id = resolve_playlist_id(&input).await?;
    let data = api(
        "/api/v6/playlist/detail",
        json!({
            "id": id,
            "n": 100000,
            "s": 0
        }),
        cookie,
    )
    .await?;
    let detail = data
        .get("playlist")
        .filter(|v| v.is_object())
        .ok_or("网易云歌单详情缺失")?;
    // tracks may contain only a preview; trackIds describes the complete playlist.
    let tracks = detail["trackIds"]
        .as_array()
        .ok_or("网易云歌单缺少完整歌曲 ID 列表，请检查登录或访问权限")?;
    let ids: Vec<_> = tracks.iter().filter_map(|v| v["id"].as_u64()).collect();
    let songs = song::fetch_details(&ids, separator, cookie).await?;
    Ok(json!({
        "playlist": playlist(detail),
        "songs": songs
    })
    .to_string())
}

pub async fn fetch_created_playlists(cookie: &str) -> Result<String, String> {
    let account = api("/api/w/nuser/account/get", json!({}), cookie).await?;
    let uid = account["profile"]["userId"]
        .as_u64()
        .ok_or("请先登录网易云音乐")?;
    let mut playlists = Vec::new();
    let mut offset = 0;
    loop {
        let data = api(
            "/api/user/playlist",
            json!({
                "uid": uid,
                "limit": 100,
                "offset": offset,
                "includeVideo": true
            }),
            cookie,
        )
        .await?;
        let rows = data["playlist"]
            .as_array()
            .ok_or("网易云用户歌单列表缺失")?;
        playlists.extend(
            rows.iter()
                .filter(|v| v["creator"]["userId"].as_u64() == Some(uid))
                .map(playlist),
        );
        if !data["more"].as_bool().unwrap_or(false) {
            break;
        }
        if rows.is_empty() {
            return Err("网易云用户歌单分页未返回数据".into());
        }
        offset += rows.len();
    }
    Ok(json!({
        "playlists": playlists
    })
    .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn playlist_links_accept_known_forms_and_reject_other_entities() {
        for url in [
            "123",
            "https://music.163.com/#/playlist?id=123",
            "分享 https://music.163.com/playlist?id=123&userid=9",
            "https://y.music.163.com/m/playlist?id=123",
        ] {
            assert_eq!(playlist_id(url).unwrap(), 123);
        }
        for url in [
            "0",
            "https://example.com/playlist?id=123",
            "https://music.163.com/song?id=123",
        ] {
            assert!(playlist_id(url).is_err());
        }
    }
}
