//! 单曲详情：按 songmid 或数字 songid 取 track_info，复用搜索使用的统一歌曲解析器。

use crate::platforms::CLIENT;
use serde_json::{json, Value};

use super::credentials::QqAuth;
use super::parser::parse_song;

/// 用 songmid 取单曲详情。
///
/// QQ 的分享链接给的是 songmid（如 `0039MnYb0qxYhV`），
/// `music.pf_song_detail_svr` 返回的 `track_info` 与搜索接口字段同构，
/// 因此直接交给 [`parse_song`] 解析，音质列表也由同一套 `file.size_*` 推导。
///
/// 会员与部分版权歌曲在匿名请求下会直接返回错误，所以 `comm` 与 `param`
/// 都要带上登录态；未登录时两者留空，行为与匿名请求一致。
pub async fn fetch_song_detail(
    separator: &str,
    songmid: &str,
    auth: Option<&QqAuth>,
) -> Result<String, String> {
    let songmid = songmid.trim();
    if songmid.is_empty() || !songmid.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("无效的歌曲 MID".into());
    }
    request_detail(
        separator,
        json!({ "song_mid": songmid, "song_type": 0, "song_id": 0, "song_uuid": 0 }),
        &format!("songmid={songmid}"),
        auth,
    )
    .await
}

/// 用数字 songid 取单曲详情。
///
/// 电脑端「分享 → 复制链接」给的是短链，展开后是
/// `i2.y.qq.com/n3/other/pages/playsong/index.html?songid=599234991`，
/// 里面只有数字 ID，没有 songmid，只能按 `song_id` 查询。
pub async fn fetch_song_detail_by_id(
    separator: &str,
    song_id: u64,
    auth: Option<&QqAuth>,
) -> Result<String, String> {
    if song_id == 0 {
        return Err("无效的歌曲 ID".into());
    }
    request_detail(
        separator,
        json!({ "song_id": song_id, "song_type": 0 }),
        &format!("songid={song_id}"),
        auth,
    )
    .await
}

async fn request_detail(
    separator: &str,
    param: Value,
    identifier: &str,
    auth: Option<&QqAuth>,
) -> Result<String, String> {
    let uin = auth.map(|value| value.uin.as_str()).unwrap_or("");
    let authst = auth.map(|value| value.authst.as_str()).unwrap_or("");

    let body = json!({
        "comm": {
            "format": "json",
            "ct": 24,
            "cv": 0,
            "tmeAppID": "qqmusic",
            "uin": uin,
            "qq": uin,
            "authst": authst
        },
        "req_0": {
            "module": "music.pf_song_detail_svr",
            "method": "get_song_detail",
            "param": param
        }
    });

    let data: Value = CLIENT
        .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
        .header("Referer", "https://y.qq.com")
        .json(&body)
        .send()
        .await
        .map_err(|error| format!("QQ 音乐详情请求失败: {error}"))?
        .error_for_status()
        .map_err(|error| format!("QQ 音乐详情请求失败: {error}"))?
        .json()
        .await
        .map_err(|error| format!("QQ 音乐详情解析失败: {error}"))?;

    let code = data["req_0"]["code"].as_i64().unwrap_or(-1);
    if data["code"] != 0 || code != 0 {
        // 400 表示这个标识在曲库里不存在：多半是链接里的串属于
        // MV、歌手或电台，或者从分享文案里取错了段。
        if code == 400 {
            return Err(format!(
                "QQ 音乐未找到该歌曲（{identifier} 无效，可能分享的不是单曲页面）"
            ));
        }
        let message = data["req_0"]["data"]["msg"]
            .as_str()
            .or_else(|| data["req_0"]["msg"].as_str())
            .unwrap_or("");
        return Err(format!(
            "QQ 音乐详情接口返回错误（code={code}）{}{}",
            if message.is_empty() { "" } else { "：" },
            message
        ));
    }

    let track = &data["req_0"]["data"]["track_info"];
    if track.is_null() {
        return Err("QQ 音乐未返回该歌曲信息".into());
    }
    parse_song(track, separator)
        .map(|song| song.to_string())
        .ok_or_else(|| "QQ 音乐无法解析该歌曲信息".to_string())
}
