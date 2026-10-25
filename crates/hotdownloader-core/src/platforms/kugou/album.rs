use super::{
    client::{number, request, validate_id},
    parser,
};
use serde_json::{json, Value};

pub async fn fetch_album_songs(separator: &str, id: String) -> Result<String, String> {
    validate_id(&id)?;
    let detail = request(
        "/kmr/v2/albums",
        "openapi.kugou.com",
        json!({}),
        Some(json!({
            "data": [{
                "album_id": id
            }],
            "is_buy": 0,
            "fields": "album_id,album_name,publish_date,sizable_cover,intro,authors,author_name"
        })),
    )
    .await?;
    let raw = detail["data"]
        .as_array()
        .and_then(|a| a.first())
        .ok_or("酷狗专辑不存在")?;
    let mut album = parser::album(raw, separator);
    let mut songs = Vec::<Value>::new();
    for page in 1..=1000 {
        let v = request(
            "/v1/album_audio/lite",
            "openapi.kugou.com",
            json!({}),
            Some(json!({
                "album_id": id,
                "is_buy": "",
                "page": page,
                "pagesize": 30
            })),
        )
        .await?;
        let list = v["data"]["songs"]
            .as_array()
            .ok_or("酷狗专辑响应缺少歌曲列表")?;
        songs.extend(list.iter().filter_map(|s| parser::parse_song(s, separator)));
        let total = number(&v["data"]["total"]);
        if page * 30 >= total {
            album["songCount"] = json!(total);
            return Ok(json!({
                "album": album,
                "songs": songs
            })
            .to_string());
        }
        if list.is_empty() {
            return Err("酷狗专辑分页提前结束".into());
        }
    }
    Err("酷狗专辑超过分页上限".into())
}
