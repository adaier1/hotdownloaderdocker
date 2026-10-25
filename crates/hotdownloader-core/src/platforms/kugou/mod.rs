//! Rust 直连酷狗；协议参考 MakcRe/KuGouMusicApi（MIT）。
//! 搜索、详情、歌词及下载由桌面端和 Web 服务端共用。

pub mod album;
pub mod artist;
mod client;
pub mod link;
pub mod login;
pub mod lyrics;
pub mod parser;
pub mod playlist;
pub mod search;
pub mod suggest;

#[cfg(test)]
mod live_tests {
    use super::*;
    use serde_json::Value;

    fn parsed(value: String) -> Value {
        serde_json::from_str(&value).unwrap()
    }

    /// 手动运行：cargo test --manifest-path crates/hotdownloader-core/Cargo.toml live_queries -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "需要连接酷狗真实服务"]
    async fn live_queries() {
        let songs = parsed(
            search::search_songs("、", "晴天".into(), 1, 2)
                .await
                .unwrap(),
        );
        assert!(!songs["songs"].as_array().unwrap().is_empty());
        let song = &songs["songs"][0];
        assert!(!song["qualities"].as_array().unwrap().is_empty());
        let lyric = lyrics::get_lyric(song["mid"].as_str().unwrap())
            .await
            .unwrap();
        assert!(lyric.lrc.is_some());
        let albums = parsed(
            search::search_albums("、", "叶惠美".into(), 1, 1)
                .await
                .unwrap(),
        );
        let album = parsed(
            album::fetch_album_songs("、", albums["albums"][0]["id"].as_str().unwrap().into())
                .await
                .unwrap(),
        );
        assert!(!album["songs"].as_array().unwrap().is_empty());
        let artists = parsed(search::search_artists("周杰伦".into(), 1, 1).await.unwrap());
        let id = artists["artists"][0]["id"].as_str().unwrap();
        let artist_songs = parsed(
            artist::fetch_artist_songs("、", id.into(), 1, 2)
                .await
                .unwrap(),
        );
        assert!(!artist_songs["songs"].as_array().unwrap().is_empty());
        let artist_albums = parsed(
            artist::fetch_artist_albums("、", id.into(), 1, 2)
                .await
                .unwrap(),
        );
        assert!(!artist_albums["albums"].as_array().unwrap().is_empty());
        let lists = parsed(
            playlist::search_playlists("周杰伦".into(), 1, 1)
                .await
                .unwrap(),
        );
        let playlist = parsed(
            playlist::fetch_playlist_songs(
                "、",
                lists["playlists"][0]["id"].as_str().unwrap().into(),
            )
            .await
            .unwrap(),
        );
        assert!(!playlist["songs"].as_array().unwrap().is_empty());
        assert!(!parsed(suggest::fetch_hot_keywords().await.unwrap())
            .as_array()
            .unwrap()
            .is_empty());
        let suggestions = parsed(suggest::fetch_suggestions("周".into()).await.unwrap());
        assert!(!suggestions["song"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    #[ignore = "需要酷狗允许当前网络获取音源"]
    async fn live_audio_link() {
        let songs = parsed(
            search::search_songs("、", "蓝心羽 晴天".into(), 1, 1)
                .await
                .unwrap(),
        );
        let song = &songs["songs"][0];
        let (url, _) = link::get_download_link(
            &crate::platforms::CLIENT,
            song["mid"].as_str().unwrap(),
            song["qualities"][0]["filename"].as_str().unwrap(),
        )
        .await
        .unwrap();
        let response = crate::platforms::CLIENT
            .get(url)
            .header("Range", "bytes=0-31")
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
    }
}
