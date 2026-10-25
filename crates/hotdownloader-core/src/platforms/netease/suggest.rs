//! 热搜关键词与分类搜索建议。
use super::{
    client::api,
    parser::{list, text},
};
use serde_json::json;

pub async fn fetch_hot_keywords(cookie: &str) -> Result<String, String> {
    let data = api("/api/hotsearchlist/get", json!({}), cookie).await?;
    let keywords: Vec<_> = list(&data["data"])
        .iter()
        .filter_map(|v| v["searchWord"].as_str())
        .take(30)
        .collect();
    Ok(json!(keywords).to_string())
}

pub async fn fetch_suggestions(
    separator: &str,
    keyword: String,
    cookie: &str,
) -> Result<String, String> {
    let data = api(
        "/api/search/suggest/web",
        json!({
            "s": keyword
        }),
        cookie,
    )
    .await?;
    let result = &data["result"];
    let mut output = json!({});
    for (input, key) in [
        ("songs", "song"),
        ("artists", "singer"),
        ("albums", "album"),
        ("mvs", "mv"),
    ] {
        let items: Vec<_> = list(&result[input])
            .iter()
            .map(|item| {
                let singer = list(&item["artists"])
                    .iter()
                    .filter_map(|artist| artist["name"].as_str())
                    .collect::<Vec<_>>()
                    .join(separator);
                json!({
                    "id": text(&item["id"]),
                    "name": item["name"],
                    "singer": singer
                })
            })
            .collect();
        output[key] = json!(items);
    }
    Ok(output.to_string())
}
