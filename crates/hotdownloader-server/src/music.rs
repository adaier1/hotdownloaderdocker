//! Web 音乐查询入口。只做请求参数校验和平台分派，解析与网络请求复用共享核心。

use hotdownloader_core::platforms;
use hotdownloader_core::platforms::qqmusic::login::LoginCredentialStore;
use hotdownloader_core::platforms::Platform;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicRequest {
    /// 解析入口不需要平台，其余操作必须提供；缺省为空串时由各操作自行校验。
    #[serde(default)]
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
    // 解析入口不要求用户先选平台：平台与内容类型都由链接自身决定。
    if action == "links/parse" {
        let fallback = request.platform.parse::<Platform>().ok();
        // QQ 详情接口对会员/版权歌曲要求登录态，这里带上当前凭据（未登录时为 None）。
        let qq_auth = platforms::qqmusic::login::auth_from_store(login_store);
        return parse_share_link(
            &request.input,
            fallback,
            artist_separator,
            netease_store,
            qq_auth.as_ref(),
        )
        .await;
    }
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
        (Platform::Netease, "playlists/fetch") => {            platforms::netease::playlist::fetch_playlist_songs(
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
        (Platform::Qishui, "songs/search") => {
            platforms::qishui::search::search_songs(
                artist_separator,
                request.keyword,
                request.page,
                request.limit,
            )
            .await?
        }
        // 汽水没有热搜与联想接口，返回空结果让前端保持可用。
        (Platform::Qishui, "hot-keywords") => return Ok(Value::Array(Vec::new())),
        (Platform::Qishui, "suggestions") => {
            return Ok(serde_json::json!({"song": [], "singer": [], "album": [], "mv": []}))
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
        (Platform::Qishui, "cover") => {
            return Err("汽水封面已包含在搜索结果中，无需单独获取".into())
        }
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
        // 汽水曲目 ID 超出前端 JS 安全整数，只能从字符串 mid 读取。
        (Platform::Qishui, "lyrics") => {
            return serde_json::to_value(platforms::qishui::lyrics::get_lyric(&request.song_mid).await?)
                .map_err(|error| error.to_string())
        }
        _ => return Err("未知音乐操作".into()),
    };
    let mut value: Value =
        serde_json::from_str(&result).map_err(|error| format!("解析平台响应失败: {error}"))?;
    // 各平台解析器不写平台标识，这里统一补齐，返回的歌曲可直接用于创建下载任务。
    inject_platform(&mut value, platform);
    Ok(value)
}

fn inject_platform(value: &mut Value, platform: Platform) {
    let label = serde_json::json!(platform.as_str());
    if let Some(songs) = value.get_mut("songs").and_then(Value::as_array_mut) {
        for song in songs {
            if let Some(object) = song.as_object_mut() {
                object
                    .entry("platform")
                    .or_insert_with(|| label.clone());
            }
        }
    }
    if let Some(song) = value.get_mut("song").and_then(Value::as_object_mut) {
        song.entry("platform").or_insert_with(|| label.clone());
    }
}

/// 解析分享链接：识别平台与内容类型，再复用各平台已有的歌单 / 专辑接口取数据。
async fn parse_share_link(
    input: &str,
    fallback: Option<Platform>,
    artist_separator: &str,
    netease_store: &platforms::netease::login::Store,
    qq_auth: Option<&platforms::qqmusic::credentials::QqAuth>,
) -> Result<Value, String> {
    let link = platforms::link_parse::resolve(input, fallback).await?;
    let cookie = if link.platform == Platform::Netease {
        netease_store.cookie()?
    } else {
        String::new()
    };
    let payload = platforms::link_parse::fetch(&link, artist_separator, &cookie, qq_auth).await?;
    let mut data: Value = serde_json::from_str(&payload)
        .map_err(|error| format!("解析平台响应失败: {error}"))?;
    // 歌单/专辑返回的是平台原始歌曲数组，缺少平台标识；单曲已由核心补齐，这里幂等补齐两种结构。
    inject_platform(&mut data, link.platform);
    Ok(serde_json::json!({
        "platform": link.platform.as_str(),
        "kind": link.kind.as_str(),
        "data": data,
    }))
}
