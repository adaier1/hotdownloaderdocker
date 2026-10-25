//! 专辑详情及歌曲列表。
use super::{
    client::{api, validate_id},
    parser::{self, list},
};
use serde_json::json;

pub async fn fetch_album_songs(
    separator: &str,
    id: String,
    cookie: &str,
) -> Result<String, String> {
    let id = validate_id(&id)?;
    let data = api(&format!("/api/v1/album/{id}"), json!({}), cookie).await?;
    let detail = data.get("album").ok_or("网易云专辑详情缺失")?;
    let songs: Vec<_> = list(&data["songs"])
        .iter()
        .map(|v| parser::song(v, separator))
        .collect();
    Ok(json!({
        "album": parser::album(detail,separator),
        "songs": songs
    })
    .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn invalid_id_fails_before_request() {
        assert!(fetch_album_songs("、", "../1".into(), "").await.is_err());
    }
}
