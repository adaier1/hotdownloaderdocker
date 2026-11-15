pub use super::search::search_playlists;
use super::{
    client::{number, request, text, validate_id},
    login::Store,
    parser,
};
use serde_json::{json, Value};
use std::collections::HashSet;

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
    fetch_songs(separator, input, json!({})).await
}

fn account_params(store: &Store) -> Result<Value, String> {
    let params = store.request_params()?;
    validate_id(&text(&params["userid"])).map_err(|_| "请先登录酷狗音乐")?;
    if text(&params["token"]).is_empty() {
        return Err("请先登录酷狗音乐".into());
    }
    Ok(params)
}

/// 当前账号的创建与收藏歌单；凭据从宿主运行时的独立登录存储取得。
pub async fn fetch_created_playlists(store: &Store) -> Result<String, String> {
    let params = account_params(store)?;
    fetch_personal_lists(params, |query, body| {
        request(
            "/v7/get_all_list",
            "cloudlist.service.kugou.com",
            query,
            Some(body),
        )
    })
    .await
}

async fn fetch_personal_lists<F, Fut>(params: Value, mut fetch: F) -> Result<String, String>
where
    F: FnMut(Value, Value) -> Fut,
    Fut: std::future::Future<Output = Result<Value, String>>,
{
    let mut playlists = Vec::new();
    let mut seen = HashSet::new();
    let mut received = 0;
    const PAGE_SIZE: u64 = 30;
    for page in 1..=1000u64 {
        // 与上游 user_playlist.js 的标准版协议一致。
        let response = fetch(
            with_params(&params, json!({"plat": 1})),
            json!({
                "userid": number(&params["userid"]),
                "token": params["token"],
                "total_ver": 979,
                "type": 2,
                "page": page,
                "pagesize": PAGE_SIZE,
            }),
        )
        .await?;
        let (rows, total) = personal_page(&response)?;
        received += rows.len() as u64;
        let before = playlists.len();
        for row in rows {
            if let Some(item) = personal_playlist(row) {
                if seen.insert(text(&item["id"])) {
                    playlists.push(item);
                }
            }
        }
        if rows.is_empty() {
            if total.is_some_and(|count| received < count) {
                return Err("酷狗个人歌单分页提前结束".into());
            }
            return Ok(json!({"playlists": playlists}).to_string());
        }
        if playlists.len() == before {
            return Err("酷狗个人歌单缺少有效标识或分页未前进".into());
        }
        if total.map_or(rows.len() < PAGE_SIZE as usize, |count| received >= count) {
            return Ok(json!({"playlists": playlists}).to_string());
        }
    }
    Err("酷狗个人歌单超过分页上限".into())
}

fn personal_page(response: &Value) -> Result<(&Vec<Value>, Option<u64>), String> {
    let rows = response["data"]["info"]
        .as_array()
        .ok_or("酷狗个人歌单响应缺少列表")?;
    let total = ["total", "count"].iter().find_map(|key| {
        response["data"]
            .get(*key)
            .filter(|v| v.is_number() || v.as_str().is_some_and(|s| s.parse::<u64>().is_ok()))
            .map(number)
    });
    Ok((rows, total))
}

fn personal_playlist(row: &Value) -> Option<Value> {
    let mut item = parser::playlist(row);
    if text(&item["id"]).is_empty() {
        item["id"] = json!(text(&row["list_create_gid"]));
    }
    let id = text(&item["id"]);
    if playlist_id(&id).ok().as_deref() != Some(id.as_str()) {
        return None;
    }
    if text(&item["name"]).is_empty() {
        item["name"] = json!(text(&row["listname"]));
    }
    item["dirid"] = json!("0");
    item["createdAt"] = json!(number(&row["create_time"]));
    item["updatedAt"] = json!(number(&row["update_time"]));
    Some(item)
}

fn with_params(credentials: &Value, extra: Value) -> Value {
    let mut params = credentials.as_object().cloned().unwrap_or_default();
    if let Some(extra) = extra.as_object() {
        params.extend(extra.clone());
    }
    Value::Object(params)
}

pub async fn fetch_created_playlist_songs(
    store: &Store,
    separator: &str,
    id: String,
) -> Result<String, String> {
    fetch_songs(separator, id, account_params(store)?).await
}

async fn fetch_songs(separator: &str, input: String, credentials: Value) -> Result<String, String> {
    fetch_song_pages(separator, input, credentials, |query| {
        request("/pubsongs/v2/get_other_list_file_nofilt", "", query, None)
    })
    .await
}

async fn fetch_song_pages<F, Fut>(
    separator: &str,
    input: String,
    credentials: Value,
    mut fetch: F,
) -> Result<String, String>
where
    F: FnMut(Value) -> Fut,
    Fut: std::future::Future<Output = Result<Value, String>>,
{
    let id = playlist_id(&input)?;
    let mut songs = Vec::<Value>::new();
    let mut playlist = Value::Null;
    for page in 0..1000u64 {
        let v = fetch(with_params(
            &credentials,
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
        ))
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

    #[tokio::test]
    async fn personal_lists_keep_device_auth_private_and_fetch_every_page() {
        let mut calls = Vec::new();
        let result = fetch_personal_lists(
            json!({"userid":"123", "token":"private-token", "mid":"same-device", "dfid":"fingerprint"}),
            |query, body| {
                calls.push((query, body.clone()));
                let rows = if body["page"] == 1 { json!([
                    {"global_collection_id":"collection_3_123_1_0", "listname":"创建的歌单", "count":4, "pic_url":"http://example.org/{size}.jpg"},
                    {"list_create_gid":"collection_3_456_2_0", "listname":"收藏的歌单", "count":7}
                ]) } else { json!([
                    {"global_collection_id":"collection_3_456_2_0", "listname":"收藏的歌单"},
                    {"global_collection_id":"collection_3_123_3_0", "name":"第三个歌单"}
                ]) };
                std::future::ready(Ok(json!({"data":{"info":rows, "total":4}})))
            },
        ).await.unwrap();
        let result: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(calls.len(), 2);
        for (query, body) in &calls {
            assert_eq!(query["mid"], "same-device");
            assert_eq!(query["dfid"], "fingerprint");
            assert_eq!(query["token"], "private-token");
            assert_eq!(body["userid"], 123);
            assert_eq!(body["token"], "private-token");
            assert_eq!(body["pagesize"], 30);
        }
        assert_eq!(result["playlists"].as_array().unwrap().len(), 3);
        assert_eq!(result["playlists"][0]["name"], "创建的歌单");
        assert_eq!(result["playlists"][0]["songCount"], 4);
        assert_eq!(
            result["playlists"][0]["coverUrl"],
            "https://example.org/300.jpg"
        );
        assert!(!result.to_string().contains("private-token"));
        assert!(!result.to_string().contains("same-device"));
    }

    #[tokio::test]
    async fn personal_lists_distinguish_empty_data_from_invalid_or_repeated_pages() {
        let result = fetch_personal_lists(json!({}), |_, _| {
            std::future::ready(Ok(json!({"data":{"info":[],"total":0}})))
        })
        .await
        .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&result).unwrap()["playlists"],
            json!([])
        );
        assert!(
            fetch_personal_lists(json!({}), |_, _| std::future::ready(Ok(json!({"data":{}}))))
                .await
                .is_err()
        );
        assert!(
            fetch_personal_lists(json!({}), |_, _| std::future::ready(Ok(
                json!({"data":{"info":[],"total":2}})
            )))
            .await
            .is_err()
        );
        assert!(
            fetch_personal_lists(json!({}), |_, _| std::future::ready(Ok(
                json!({"data":{"info":[{"listname":"缺少标识"}],"total":1}})
            )))
            .await
            .is_err()
        );
        let repeated = fetch_personal_lists(json!({}), |_, _| {
            std::future::ready(Ok(
                json!({"data":{"info":[{"global_collection_id":"collection_3_1_1_0"}],"total":5}}),
            ))
        })
        .await;
        assert!(repeated.unwrap_err().contains("分页未前进"));
    }

    #[tokio::test]
    async fn personal_songs_require_login_and_keep_auth_on_subsequent_pages() {
        let path = std::env::temp_dir().join(format!(
            "hotdownloader-kugou-list-{}.json",
            crate::platforms::get_guid()
        ));
        let store = Store::new(path);
        assert!(fetch_created_playlists(&store)
            .await
            .unwrap_err()
            .contains("请先登录"));
        assert!(
            fetch_created_playlist_songs(&store, "、", "collection_3_1_1_0".into())
                .await
                .unwrap_err()
                .contains("请先登录")
        );
        let mut calls = 0;
        let result = fetch_song_pages("、", "collection_3_1_1_0".into(), json!({"userid":"123","token":"private-token","mid":"device"}), |query| {
            calls += 1;
            assert_eq!(query["token"], "private-token");
            assert_eq!(query["mid"], "device");
            assert_eq!(query["begin_idx"], (calls-1)*100);
            let song = json!({"hash": "0123456789ABCDEF0123456789ABCDEF", "FileName":"歌手 - 歌曲", "FileSize":1024});
            std::future::ready(Ok(json!({"data":{"songs":[song], "count":101, "list_info":{"name":"测试歌单"}}})))
        }).await.unwrap();
        assert_eq!(calls, 2);
        let result: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(result["playlist"]["id"], "collection_3_1_1_0");
        assert_eq!(result["songs"].as_array().unwrap().len(), 2);
        assert_eq!(result["songs"][0]["title"], "歌曲");
        assert_eq!(result["songs"][0]["artist"], "歌手");
        assert_eq!(result["songs"][0]["qualities"][0]["quality"], "128kmp3");
        assert!(!result.to_string().contains("private-token"));
    }
}
