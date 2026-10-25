//! 手动运行的联网冒烟测试；默认测试不访问网易云。
use super::{
    album, artist,
    client::request,
    link, login, lyrics,
    parser::{list, text},
    playlist, search, suggest,
};
use serde_json::{json, Value};

#[tokio::test]
#[ignore = "manual public API smoke test; needs external network"]
async fn live_public_queries() {
    for (action, result) in [
        (
            "songs/search",
            search::search_songs("、", "周杰伦".into(), 1, 2, "").await,
        ),
        (
            "albums/search",
            search::search_albums("、", "周杰伦".into(), 1, 2, "").await,
        ),
        (
            "artists/search",
            search::search_artists("周杰伦".into(), 1, 2, "").await,
        ),
        (
            "playlists/search",
            playlist::search_playlists("周杰伦".into(), 1, 2, "").await,
        ),
        ("hot-keywords", suggest::fetch_hot_keywords("").await),
        (
            "suggestions",
            suggest::fetch_suggestions("、", "周杰伦".into(), "").await,
        ),
        (
            "albums/fetch",
            album::fetch_album_songs("、", "32311".into(), "").await,
        ),
        (
            "artists/songs",
            artist::fetch_artist_songs("、", "6452".into(), 1, 2, "").await,
        ),
        (
            "artists/albums",
            artist::fetch_artist_albums("、", "6452".into(), 1, 2, "").await,
        ),
    ] {
        let result: Value =
            serde_json::from_str(&result.unwrap_or_else(|e| panic!("{action}: {e}"))).unwrap();
        let field = match action {
            "songs/search" | "artists/songs" | "albums/fetch" => "songs",
            "albums/search" | "artists/albums" => "albums",
            "artists/search" => "artists",
            "playlists/search" => "playlists",
            "suggestions" => "song",
            _ => "",
        };
        let rows = if field.is_empty() {
            result.as_array()
        } else {
            result[field].as_array()
        }
        .expect(action);
        assert!(!rows.is_empty(), "{action}: empty result");
        if field == "songs" {
            assert!(
                rows[0]["qualities"]
                    .as_array()
                    .is_some_and(|a| !a.is_empty()),
                "{action}: missing qualities"
            );
        }
        eprintln!("{action}: {} items", rows.len());
    }
    let detail: Value = serde_json::from_str(
        &playlist::fetch_playlist_songs("、", "3778678".into(), "")
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        detail["playlist"]["songCount"].as_u64().unwrap() as usize,
        detail["songs"].as_array().unwrap().len()
    );
    assert!(lyrics::get_lyric_by_id(186016).await.unwrap().lrc.is_some());
}

#[tokio::test]
#[ignore = "manual audio/QR smoke test; needs external network"]
async fn live_source_and_qr() {
    let search = request(
        "/api/cloudsearch/pc",
        json!({
            "s": "歌唱祖国",
            "type": 1,
            "limit": 5,
            "offset": 0
        }),
        "",
        true,
    )
    .await
    .unwrap()
    .0;
    let mut source = None;
    for row in list(&search["result"]["songs"]) {
        let id = text(&row["id"]);
        match link::get_download_link(&id, &format!("{id}_standard.mp3"), "").await {
            Ok(found) => {
                source = Some(found);
                break;
            }
            Err(error) => eprintln!("guest audio {id}: {error}"),
        }
    }
    let source = source.expect("no full guest audio returned for smoke samples");
    assert!(source.1.is_empty());
    let response = crate::platforms::CLIENT
        .get(&source.0)
        .header("Range", "bytes=0-1023")
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    // Drop without downloading the complete song.
    drop(response);
    let path = std::env::temp_dir().join(format!("netease-live-{}.json", rand::random::<u64>()));
    let store = login::Store::new(&path);
    let qr = login::execute(&store, "qr", "", None).await.unwrap();
    assert!(qr["qr_image"]
        .as_str()
        .unwrap()
        .starts_with("data:image/svg+xml;base64,"));
    let status = login::execute(&store, "check", qr["qrcode_id"].as_str().unwrap(), None)
        .await
        .unwrap();
    assert_eq!(status["status"], "waiting");
    login::execute(&store, "logout", "", None).await.unwrap();
    std::fs::remove_file(path).unwrap();
}
