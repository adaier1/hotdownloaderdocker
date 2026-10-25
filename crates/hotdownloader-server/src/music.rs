//! Web 音乐查询入口。只做请求参数校验和平台分派，解析与网络请求复用共享核心。

use hotdownloader_core::platforms;
use hotdownloader_core::platforms::qqmusic::login::LoginCredentialStore;
use hotdownloader_core::platforms::Platform;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicRequest {
    platform: String,
    #[serde(default)]
    keyword: String,
    #[serde(default)]
    input: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    dirid: String,
    #[serde(default)]
    song_id: u64,
    #[serde(default)]
    song_mid: String,
    #[serde(default = "first_page")]
    page: u32,
    #[serde(default = "default_limit")]
    limit: u32,
}

fn first_page() -> u32 {
    1
}

fn default_limit() -> u32 {
    20
}

pub async fn execute(
    action: &str,
    request: MusicRequest,
    artist_separator: &str,
    login_store: &dyn LoginCredentialStore,
    netease_store: &platforms::netease::login::Store,
) -> Result<Value, String> {
    let platform = request.platform.parse::<Platform>()?;
    if request.page == 0 || request.limit == 0 || request.limit > 100 {
        return Err("无效的分页参数".into());
    }
    let netease_cookie = if platform == Platform::Netease {
        netease_store.cookie()?
    } else {
        String::new()
    };
    let result = match (platform, action) {
        (Platform::Netease, "songs/search") => {
            platforms::netease::search::search_songs(
                artist_separator,
                request.keyword,
                request.page,
                request.limit,
                &netease_cookie,
            )
            .await?
        }
        (Platform::Netease, "albums/search") => {
            platforms::netease::search::search_albums(
                artist_separator,
                request.keyword,
                request.page,
                request.limit,
                &netease_cookie,
            )
            .await?
        }
        (Platform::Netease, "artists/search") => {
            platforms::netease::search::search_artists(
                request.keyword,
                request.page,
                request.limit,
                &netease_cookie,
            )
            .await?
        }
        (Platform::Netease, "playlists/search") => {
            platforms::netease::playlist::search_playlists(
                request.keyword,
                request.page,
                request.limit,
                &netease_cookie,
            )
            .await?
        }
        (Platform::Netease, "hot-keywords") => {
            platforms::netease::suggest::fetch_hot_keywords(&netease_cookie).await?
        }
        (Platform::Netease, "suggestions") => {
            platforms::netease::suggest::fetch_suggestions(
                artist_separator,
                request.keyword,
                &netease_cookie,
            )
            .await?
        }
        (Platform::Netease, "playlists/fetch") => {
            platforms::netease::playlist::fetch_playlist_songs(
                artist_separator,
                request.input,
                &netease_cookie,
            )
            .await?
        }
        (Platform::Netease, "playlists/created") => {
            platforms::netease::playlist::fetch_created_playlists(&netease_cookie).await?
        }
        (Platform::Netease, "playlists/created/fetch") => {
            platforms::netease::playlist::fetch_playlist_songs(
                artist_separator,
                request.id,
                &netease_cookie,
            )
            .await?
        }
        (Platform::Netease, "albums/fetch") => {
            platforms::netease::album::fetch_album_songs(
                artist_separator,
                request.id,
                &netease_cookie,
            )
            .await?
        }
        (Platform::Netease, "artists/songs") => {
            platforms::netease::artist::fetch_artist_songs(
                artist_separator,
                request.id,
                request.page,
                request.limit,
                &netease_cookie,
            )
            .await?
        }
        (Platform::Netease, "artists/albums") => {
            platforms::netease::artist::fetch_artist_albums(
                artist_separator,
                request.id,
                request.page,
                request.limit,
                &netease_cookie,
            )
            .await?
        }
        (Platform::Netease, "cover") => {
            return platforms::netease::cover::fetch_cover(request.song_id, &netease_cookie)
                .await
                .map(Value::String)
        }
        (Platform::Netease, "lyrics") => {
            return serde_json::to_value(
                platforms::netease::lyrics::get_lyric_by_id(request.song_id).await?,
            )
            .map_err(|e| e.to_string());
        }

        (Platform::QqMusic, "songs/search") => {
            platforms::qqmusic::search::search_songs(
                artist_separator,
                request.keyword,
                request.page,
                request.limit,
            )
            .await?
        }
        (Platform::Kuwo, "songs/search") => {
            platforms::kuwo::search::search_songs(
                artist_separator,
                request.keyword,
                request.page,
                request.limit,
            )
            .await?
        }
        (Platform::Kugou, "songs/search") => {
            platforms::kugou::search::search_songs(
                artist_separator,
                request.keyword,
                request.page,
                request.limit,
            )
            .await?
        }
        (Platform::QqMusic, "hot-keywords") => {
            platforms::qqmusic::suggest::fetch_hot_keywords().await?
        }
        (Platform::Kuwo, "hot-keywords") => platforms::kuwo::suggest::fetch_hot_keywords().await?,
        (Platform::Kugou, "hot-keywords") => {
            platforms::kugou::suggest::fetch_hot_keywords().await?
        }
        (Platform::QqMusic, "suggestions") => {
            platforms::qqmusic::suggest::fetch_suggestions(request.keyword).await?
        }
        (Platform::Kuwo, "suggestions") => {
            platforms::kuwo::suggest::fetch_suggestions(request.keyword).await?
        }
        (Platform::Kugou, "suggestions") => {
            platforms::kugou::suggest::fetch_suggestions(request.keyword).await?
        }
        (Platform::QqMusic, "playlists/fetch") => {
            platforms::qqmusic::playlist::fetch_playlist_songs(artist_separator, request.input)
                .await?
        }
        // 个人歌单只使用服务端持有的登录存储，不接受客户端传入 UIN。
        (Platform::QqMusic, "playlists/created") => {
            platforms::qqmusic::playlist::fetch_created_playlists(login_store).await?
        }
        (Platform::QqMusic, "playlists/created/fetch") => {
            platforms::qqmusic::playlist::fetch_created_playlist_songs(
                login_store,
                artist_separator,
                request.id,
                request.dirid,
            )
            .await?
        }
        (Platform::Kuwo, "playlists/fetch") => {
            platforms::kuwo::playlist::fetch_playlist_songs(artist_separator, request.input).await?
        }
        (Platform::Kugou, "playlists/fetch") => {
            platforms::kugou::playlist::fetch_playlist_songs(artist_separator, request.input)
                .await?
        }
        (Platform::QqMusic, "playlists/search") => {
            platforms::qqmusic::playlist::search_playlists(
                request.keyword,
                request.page,
                request.limit,
            )
            .await?
        }
        (Platform::Kuwo, "playlists/search") => {
            platforms::kuwo::playlist::search_playlists(
                request.keyword,
                request.page,
                request.limit,
            )
            .await?
        }
        (Platform::Kugou, "playlists/search") => {
            platforms::kugou::playlist::search_playlists(
                request.keyword,
                request.page,
                request.limit,
            )
            .await?
        }
        (Platform::QqMusic, "albums/search") => {
            platforms::qqmusic::search::search_albums(
                artist_separator,
                request.keyword,
                request.page,
                request.limit,
            )
            .await?
        }
        (Platform::Kuwo, "albums/search") => {
            platforms::kuwo::search::search_albums(
                artist_separator,
                request.keyword,
                request.page,
                request.limit,
            )
            .await?
        }
        (Platform::Kugou, "albums/search") => {
            platforms::kugou::search::search_albums(
                artist_separator,
                request.keyword,
                request.page,
                request.limit,
            )
            .await?
        }
        (Platform::QqMusic, "albums/fetch") => {
            platforms::qqmusic::album::fetch_album_songs(artist_separator, request.id).await?
        }
        (Platform::Kuwo, "albums/fetch") => {
            platforms::kuwo::album::fetch_album_songs(artist_separator, request.id).await?
        }
        (Platform::Kugou, "albums/fetch") => {
            platforms::kugou::album::fetch_album_songs(artist_separator, request.id).await?
        }
        (Platform::QqMusic, "artists/search") => {
            platforms::qqmusic::search::search_artists(request.keyword, request.page, request.limit)
                .await?
        }
        (Platform::Kuwo, "artists/search") => {
            platforms::kuwo::search::search_artists(request.keyword, request.page, request.limit)
                .await?
        }
        (Platform::Kugou, "artists/search") => {
            platforms::kugou::search::search_artists(request.keyword, request.page, request.limit)
                .await?
        }
        (Platform::QqMusic, "artists/songs") => {
            platforms::qqmusic::artist::fetch_artist_songs(
                artist_separator,
                request.id,
                request.page,
                request.limit,
            )
            .await?
        }
        (Platform::Kuwo, "artists/songs") => {
            platforms::kuwo::artist::fetch_artist_songs(
                artist_separator,
                request.id,
                request.page,
                request.limit,
            )
            .await?
        }
        (Platform::Kugou, "artists/songs") => {
            platforms::kugou::artist::fetch_artist_songs(
                artist_separator,
                request.id,
                request.page,
                request.limit,
            )
            .await?
        }
        (Platform::QqMusic, "artists/albums") => {
            platforms::qqmusic::artist::fetch_artist_albums(
                artist_separator,
                request.id,
                request.page,
                request.limit,
            )
            .await?
        }
        (Platform::Kuwo, "artists/albums") => {
            platforms::kuwo::artist::fetch_artist_albums(
                artist_separator,
                request.id,
                request.page,
                request.limit,
            )
            .await?
        }
        (Platform::Kugou, "artists/albums") => {
            platforms::kugou::artist::fetch_artist_albums(
                artist_separator,
                request.id,
                request.page,
                request.limit,
            )
            .await?
        }
        (Platform::Kuwo, "cover") => {
            return Ok(Value::String(
                platforms::kuwo::cover::fetch_cover(request.song_id).await?,
            ));
        }
        (Platform::Kugou, "cover") => return Err("酷狗封面已包含在搜索结果中，无需单独获取".into()),
        (Platform::QqMusic, "cover") => {
            return Err("QQ 音乐封面已包含在搜索结果中，无需单独获取".into());
        }
        (Platform::QqMusic, "lyrics") => {
            return serde_json::to_value(
                platforms::qqmusic::lyrics::get_lyric_by_id(request.song_id).await?,
            )
            .map_err(|error| error.to_string());
        }
        (Platform::Kuwo, "lyrics") => {
            return serde_json::to_value(
                platforms::kuwo::lyrics::get_lyric_by_id(request.song_id).await?,
            )
            .map_err(|error| error.to_string());
        }
        (Platform::Kugou, "lyrics") => {
            return serde_json::to_value(
                platforms::kugou::lyrics::get_lyric(&request.song_mid).await?,
            )
            .map_err(|e| e.to_string())
        }
        _ => return Err("未知音乐操作".into()),
    };
    serde_json::from_str(&result).map_err(|error| format!("解析平台响应失败: {error}"))
}
