use super::client::request;
use serde_json::{json, Value};

pub async fn fetch_hot_keywords() -> Result<String, String> {
    let v = request(
        "/api/v3/search/hot_tab",
        "msearch.kugou.com",
        json!({
            "navid": 1,
            "plat": 2
        }),
        None,
    )
    .await?;
    let list = v["data"]["list"][0]["keywords"]
        .as_array()
        .ok_or("酷狗热搜响应缺少列表")?;
    Ok(json!(list
        .iter()
        .filter_map(|v| v["keyword"].as_str())
        .collect::<Vec<_>>())
    .to_string())
}
pub async fn fetch_suggestions(keyword: String) -> Result<String, String> {
    let mut result = json!({
        "song": [],
        "singer": [],
        "album": [],
        "mv": []
    });
    if keyword.trim().is_empty() {
        return Ok(result.to_string());
    }
    let v = request(
        "/v2/getSearchTip",
        "searchtip.kugou.com",
        json!({
            "keyword": keyword,
            "AlbumTipCount": 10,
            "CorrectTipCount": 10,
            "MVTipCount": 10,
            "MusicTipCount": 10,
            "radiotip": 1
        }),
        None,
    )
    .await?;
    let groups = v["data"].as_array().ok_or("酷狗搜索建议响应缺少列表")?;
    for group in groups {
        let key = match group["LableName"].as_str().unwrap_or("") {
            "MV" => "mv",
            "专辑" => "album",
            "歌手" => "singer",
            _ => "song",
        };
        if let Some(items) = group["RecordDatas"].as_array() {
            let items: Vec<Value> = items
                .iter()
                .filter_map(|v| {
                    v["HintInfo"].as_str().map(|s| {
                        json!({
                            "name": s
                        })
                    })
                })
                .collect();
            result[key].as_array_mut().unwrap().extend(items);
        }
    }
    Ok(result.to_string())
}
