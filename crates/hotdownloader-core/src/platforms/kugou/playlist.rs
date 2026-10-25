pub use super::search::search_playlists;
use super::{
    client::{number, request},
    parser,
};
use serde_json::{json, Value};

fn playlist_id(input: &str) -> Result<String, String> {
    let pattern = regex::Regex::new(r"^collection_\d+_\d+_\d+_\d+$").unwrap();
    let input = input.trim();
    if pattern.is_match(input) {
        return Ok(input.into());
    }
    if let Ok(url) = reqwest::Url::parse(input) {
        if url
            .host_str()
            .is_some_and(|h| h == "kugou.com" || h.ends_with(".kugou.com"))
        {
            for (key, value) in url.query_pairs() {
                if ["global_collection_id", "gid"].contains(&key.as_ref())
                    && pattern.is_match(&value)
                {
                    return Ok(value.into_owned());
                }
            }
        }
    }
    Err("请输入酷狗歌单的 collection_… 标识或包含 global_collection_id 的分享链接".into())
}

pub async fn fetch_playlist_songs(separator: &str, input: String) -> Result<String, String> {
    let id = playlist_id(&input)?;
    let mut songs = Vec::<Value>::new();
    let mut playlist = Value::Null;
    for page in 0..1000u64 {
        let v = request(
            "/pubsongs/v2/get_other_list_file_nofilt",
            "",
            json!({
                "area_code": 1,
                "begin_idx": page*100,
                "plat": 1,
                "type": 1,
                "mode": 1,
                "personal_switch": 1,
                "extend_fields": "abtags,hot_cmt,popularization",
                "pagesize": 100,
                "global_collection_id": id
            }),
            None,
        )
        .await?;
        let list = v["data"]["songs"]
            .as_array()
            .ok_or("酷狗歌单响应缺少歌曲列表")?;
        if page == 0 {
            playlist = parser::playlist(&v["data"]["list_info"]);
            playlist["id"] = json!(id);
        }
        songs.extend(list.iter().filter_map(|s| parser::parse_song(s, separator)));
        if (page + 1) * 100 >= number(&v["data"]["count"]) {
            return Ok(json!({
                "playlist": playlist,
                "songs": songs
            })
            .to_string());
        }
        if list.is_empty() {
            return Err("酷狗歌单分页提前结束".into());
        }
    }
    Err("酷狗歌单超过分页上限".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_share_links() {
        assert_eq!(
            playlist_id("https://www.kugou.com/share?global_collection_id=collection_3_1_2_0")
                .unwrap(),
            "collection_3_1_2_0"
        );
        assert!(playlist_id("https://kugou.com.example.org/?gid=collection_3_1_2_0").is_err());
    }
}
