//! 歌曲详情批量补全，供搜索、歌单和封面查询复用。
use super::{client::api, parser::song};
use serde_json::{json, Value};

pub(super) async fn fetch_details(
    ids: &[u64],
    separator: &str,
    cookie: &str,
) -> Result<Vec<Value>, String> {
    let mut result = Vec::new();
    for chunk in ids.chunks(500) {
        let c: Vec<_> = chunk
            .iter()
            .map(|id| {
                json!({
                    "id": id
                })
            })
            .collect();
        let data = api(
            "/api/v3/song/detail",
            json!({
                "c": json!(c).to_string()
            }),
            cookie,
        )
        .await?;
        let rows = data["songs"].as_array().ok_or("网易云歌曲详情缺少 songs")?;
        let songs: std::collections::HashMap<_, _> = rows
            .iter()
            .filter_map(|v| v["id"].as_u64().map(|id| (id, v)))
            .collect();
        // Detail responses may reorder IDs; preserve playlist/search order.
        result.extend(
            chunk
                .iter()
                .filter_map(|id| songs.get(id))
                .map(|v| song(v, separator)),
        );
    }
    Ok(result)
}
