//! 歌手歌曲与专辑分页查询。
use super::{
    client::{api, page_offset, validate_id},
    parser,
};
use serde_json::json;

pub async fn fetch_artist_songs(
    separator: &str,
    id: String,
    page: u32,
    limit: u32,
    cookie: &str,
) -> Result<String, String> {
    let id = validate_id(&id)?;
    let offset = page_offset(page, limit)?;
    let data = api(
        "/api/v1/artist/songs",
        json!({
            "id": id,
            "private_cloud": "true",
            "work_type": 1,
            "order": "hot",
            "offset": offset,
            "limit": limit
        }),
        cookie,
    )
    .await?;
    let rows = data["songs"].as_array().ok_or("网易云歌手歌曲列表缺失")?;
    let total = data["total"].as_u64().unwrap_or(0);
    let songs: Vec<_> = rows.iter().map(|v| parser::song(v, separator)).collect();
    let has_more = data["more"]
        .as_bool()
        .unwrap_or(offset + (rows.len() as u64) < total);
    Ok(json!({
        "songs": songs,
        "total": total,
        "has_more": has_more
    })
    .to_string())
}

pub async fn fetch_artist_albums(
    separator: &str,
    id: String,
    page: u32,
    limit: u32,
    cookie: &str,
) -> Result<String, String> {
    let id = validate_id(&id)?;
    let offset = page_offset(page, limit)?;
    let data = api(
        &format!("/api/artist/albums/{id}"),
        json!({
            "offset": offset,
            "limit": limit,
            "total": true
        }),
        cookie,
    )
    .await?;
    let rows = data["hotAlbums"]
        .as_array()
        .ok_or("网易云歌手专辑列表缺失")?;
    let total = data["artist"]["albumSize"].as_u64().unwrap_or(0);
    let albums: Vec<_> = rows.iter().map(|v| parser::album(v, separator)).collect();
    let has_more = data["more"]
        .as_bool()
        .unwrap_or(offset + (rows.len() as u64) < total);
    Ok(json!({
        "albums": albums,
        "total": total,
        "name": data["artist"]["name"],
        "has_more": has_more
    })
    .to_string())
}
