//! 歌曲、专辑和歌手搜索，共用网易云搜索请求及分页规则。
use super::{
    client::{page_offset, request},
    parser::{self, list},
    song,
};
use serde_json::{json, Value};

pub(super) async fn search_page(
    keyword: &str,
    page: u32,
    limit: u32,
    kind: u32,
    field: &str,
    count_field: &str,
    cookie: &str,
) -> Result<(Vec<Value>, bool), String> {
    let offset = page_offset(page, limit)?;
    let (data, _) = request(
        "/api/cloudsearch/pc",
        json!({
            "s": keyword,
            "type": kind,
            "limit": limit,
            "offset": offset,
            "total": true
        }),
        cookie,
        true,
    )
    .await?;
    let result = data.get("result").ok_or("网易云搜索响应缺少 result")?;
    let rows = list(&result[field]);
    let has_more = offset + (rows.len() as u64) < result[count_field].as_u64().unwrap_or(0);
    Ok((rows.to_vec(), has_more))
}

pub async fn search_songs(
    separator: &str,
    keyword: String,
    page: u32,
    limit: u32,
    cookie: &str,
) -> Result<String, String> {
    let (rows, has_more) =
        search_page(&keyword, page, limit, 1, "songs", "songCount", cookie).await?;
    // Search may omit file metadata. Hydrate only the current page.
    let ids: Vec<_> = rows.iter().filter_map(|v| v["id"].as_u64()).collect();
    let songs = song::fetch_details(&ids, separator, cookie).await?;
    Ok(json!({
        "songs": songs,
        "has_more": has_more
    })
    .to_string())
}

pub async fn search_albums(
    separator: &str,
    keyword: String,
    page: u32,
    limit: u32,
    cookie: &str,
) -> Result<String, String> {
    let (rows, has_more) =
        search_page(&keyword, page, limit, 10, "albums", "albumCount", cookie).await?;
    let albums: Vec<_> = rows.iter().map(|v| parser::album(v, separator)).collect();
    Ok(json!({
        "albums": albums,
        "has_more": has_more
    })
    .to_string())
}

pub async fn search_artists(
    keyword: String,
    page: u32,
    limit: u32,
    cookie: &str,
) -> Result<String, String> {
    let (rows, has_more) =
        search_page(&keyword, page, limit, 100, "artists", "artistCount", cookie).await?;
    let artists: Vec<_> = rows.iter().map(parser::artist).collect();
    Ok(json!({
        "artists": artists,
        "has_more": has_more
    })
    .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn invalid_page_fails_before_request() {
        assert!(search_songs("、", String::new(), 0, 20, "").await.is_err());
    }
}
