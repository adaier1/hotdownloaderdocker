use super::{
    client::{number, request, validate_page},
    parser,
};
use serde_json::{json, Value};

async fn search(
    separator: &str,
    keyword: String,
    page: u32,
    limit: u32,
    kind: &str,
    key: &str,
) -> Result<String, String> {
    validate_page(page, limit)?;
    let path = if kind == "song" {
        "/v2/search/song".to_owned()
    } else {
        format!("/v1/search/{kind}")
    };
    let v = request(
        &path,
        "complexsearch.kugou.com",
        json!({
            "keyword": keyword,
            "page": page,
            "pagesize": limit,
            "platform": "AndroidFilter",
            "iscorrection": 1,
            "privilegefilter": 0,
            "area_code": 1,
            "dopicfull": 1,
            "albumhide": 0,
            "nocollect": 0
        }),
        None,
    )
    .await?;
    let list = v["data"]["lists"]
        .as_array()
        .ok_or("酷狗搜索响应缺少列表")?;
    let items: Vec<Value> = list
        .iter()
        .filter_map(|v| match kind {
            "song" => parser::parse_song(v, separator),
            "album" => Some(parser::album(v, separator)),
            "author" => Some(parser::artist(v)),
            _ => Some(parser::playlist(v)),
        })
        .collect();
    Ok(json!({
        key: items,
        "has_more": u64::from(page)*u64::from(limit)<number(&v["data"]["total"])
    })
    .to_string())
}
pub async fn search_songs(
    separator: &str,
    keyword: String,
    page: u32,
    limit: u32,
) -> Result<String, String> {
    search(separator, keyword, page, limit, "song", "songs").await
}
pub async fn search_albums(
    separator: &str,
    keyword: String,
    page: u32,
    limit: u32,
) -> Result<String, String> {
    search(separator, keyword, page, limit, "album", "albums").await
}
pub async fn search_artists(keyword: String, page: u32, limit: u32) -> Result<String, String> {
    search("、", keyword, page, limit, "author", "artists").await
}
pub async fn search_playlists(keyword: String, page: u32, limit: u32) -> Result<String, String> {
    search("、", keyword, page, limit, "special", "playlists").await
}
