use super::{
    client::{number, request, validate_id, validate_page},
    parser,
};
use serde_json::{json, Value};

pub async fn fetch_artist_songs(
    separator: &str,
    id: String,
    page: u32,
    limit: u32,
) -> Result<String, String> {
    validate_page(page, limit)?;
    validate_id(&id)?;
    let v = request(
        "/openapi/kmr/v2/audio_group/author",
        "",
        json!({
            "author_id": id,
            "area_code": "all",
            "sort": 1,
            "page": page,
            "pagesize": limit,
            "replace_api_version": 1,
            "mvdata_need": 1,
            "show_audio_honor": 1,
            "show_audio_tag": 1,
            "replace_need": 1
        }),
        None,
    )
    .await?;
    let list = v["data"]["songs"]
        .as_array()
        .ok_or("酷狗歌手响应缺少歌曲列表")?;
    let songs: Vec<Value> = list
        .iter()
        .filter_map(|s| parser::parse_song(s, separator))
        .collect();
    Ok(json!({
        "songs": songs,
        "total": number(&v["data"]["total"]),
        "has_more": u64::from(page)*u64::from(limit)<number(&v["data"]["total"])
    })
    .to_string())
}
pub async fn fetch_artist_albums(
    separator: &str,
    id: String,
    page: u32,
    limit: u32,
) -> Result<String, String> {
    validate_page(page, limit)?;
    validate_id(&id)?;
    let v = request(
        "/kmr/v1/author/albums",
        "openapi.kugou.com",
        json!({}),
        Some(json!({
            "author_id": id,
            "area_code": "all",
            "sort": 1,
            "category": 1,
            "page": page,
            "pagesize": limit
        })),
    )
    .await?;
    let list = v["data"].as_array().ok_or("酷狗歌手响应缺少专辑列表")?;
    let albums: Vec<Value> = list.iter().map(|s| parser::album(s, separator)).collect();
    Ok(json!({
        "albums": albums,
        "total": number(&v["total"]),
        "has_more": u64::from(page)*u64::from(limit)<number(&v["total"])
    })
    .to_string())
}
