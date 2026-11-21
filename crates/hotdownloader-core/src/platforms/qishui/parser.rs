//! 汽水详情解析：把 `track_v2` / `seo_track` 两种响应归一成同一套结构。
//!
//! 播放地址有两个来源：`track_player.video_model`（可能被多层 JSON 字符串转义）
//! 与 `track_player.url_player_info`（返回海外的 PlayInfoList）。两者都要收集，
//! 因为不同曲目只会在其中一个里给出可用流。

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{json, Map, Value};

use super::{client, is_track_id, API_BASE, SEO_API};

/// 单条可用流。`play_auth` 非空表示音频被加密，需要在下载完成后解密。
#[derive(Debug, Clone)]
pub struct Player {
    pub url: String,
    pub play_auth: String,
    pub format: String,
    /// 归一为 kbps。
    pub bitrate: u32,
    pub size: u64,
    /// 归一为秒。
    pub duration: u64,
    pub quality: String,
    pub preview: bool,
}

impl Player {
    /// 降级顺序里使用的品质标签。
    ///
    /// 试听片段同样给出档位标签：否则会员曲目连「下载试听」都无法创建任务。
    /// 片段身份由 [`Player::preview`] 与歌曲上的 `preview` 字段表达。
    pub fn quality_label(&self) -> Option<&'static str> {
        let quality = self.quality.to_lowercase();
        let format = self.format.to_lowercase();
        if quality.contains("spatial") || quality.contains("atmos") || quality.contains("dolby") {
            return Some("spatial");
        }
        // 汽水的无损按 quality 标记区分 Hi-Res 与普通无损，两者都是 flac 容器。
        if format.contains("flac") || format.contains("alac") || format.contains("wav") {
            return Some(if quality.contains("hires")
                || quality.contains("hi-res")
                || quality.contains("master")
            {
                "hires"
            } else {
                "flac"
            });
        }
        Some(match self.bitrate {
            bitrate if bitrate >= 256 => "320kaac",
            bitrate if bitrate >= 96 => "128kaac",
            bitrate if bitrate > 0 => "64kaac",
            _ => return None,
        })
    }

    /// 与项目 `{bitrate}.{format}` 的文件名约定保持一致，link 侧按它反查档位。
    pub fn filename(&self) -> String {
        match self.quality_label() {
            Some("hires") => "hires.flac".into(),
            Some("flac") => "flac.flac".into(),
            Some("spatial") => "spatial.m4a".into(),
            Some(label) => format!("{}.{}", label.trim_end_matches("kaac"), m4a_or(&self.format)),
            None => String::new(),
        }
    }
}

/// 非 flac 档位统一按 m4a 交付；个别曲目会给 mp4 容器，扩展名仍用 m4a 才能被播放器识别。
fn m4a_or(format: &str) -> &str {
    let format = format.to_lowercase();
    if format.contains("flac") {
        "flac"
    } else {
        "m4a"
    }
}

pub(crate) fn track_v2_url(track_id: &str) -> String {
    client::url_with(
        &format!("{API_BASE}/pc/track_v2"),
        &[
            ("track_id", track_id),
            ("media_type", "track"),
            ("aid", super::AID),
            ("device_platform", "web"),
            ("channel", "pc_web"),
        ],
    )
}

pub(crate) fn seo_track_url(track_id: &str) -> String {
    client::url_with(
        SEO_API,
        &[("track_id", track_id), ("device_platform", "web")],
    )
}

/// 取详情：先 requests 带 Cookie 的 `track_v2`，失败或结构不认识时回退免签接口。
pub async fn fetch_track_payload(track_id: &str) -> Result<Value, String> {
    let track_id = track_id.trim();
    if !is_track_id(track_id) {
        return Err("无效的汽水曲目 ID".into());
    }
    let primary = match client::get_json(&track_v2_url(track_id)).await {
        Ok(value) => check_status(value),
        Err(error) => Err(error),
    };
    match primary {
        Ok(value) if has_track(&value) => Ok(value),
        Ok(_) => fallback_to_seo(track_id, "详情响应缺少曲目字段").await,
        Err(error) => fallback_to_seo(track_id, &error).await,
    }
}

async fn fallback_to_seo(track_id: &str, reason: &str) -> Result<Value, String> {
    let value = client::get_json(&seo_track_url(track_id))
        .await
        .map_err(|error| format!("汽水详情接口失败（{reason}）；兜底接口失败: {error}"))?;
    let value = check_status(value)?;
    normalize_seo(value).ok_or_else(|| format!("汽水兜底接口未返回曲目信息（{reason}）"))
}

fn check_status(value: Value) -> Result<Value, String> {
    match value["status_code"].as_i64() {
        Some(0) | None => Ok(value),
        Some(code) => {
            let message = value["status_info"]["status_msg"]
                .as_str()
                .unwrap_or("未知错误");
            Err(format!("汽水接口返回错误（status_code={code}, {message}）"))
        }
    }
}

fn has_track(value: &Value) -> bool {
    value["track"]["id"].is_string() || value["track_info"]["id"].is_string()
}

/// `seo_track` 把曲目放在 `seo_track` 节点里，播放信息与歌词在顶层，这里压平成同一种结构。
fn normalize_seo(value: Value) -> Option<Value> {
    let mut root = Map::new();
    let seo = value.get("seo_track");
    let track = seo
        .and_then(|node| node.get("track"))
        .or_else(|| value.get("track"))
        .filter(|track| track["id"].is_string())?
        .clone();
    root.insert("track".into(), track);
    let player = value
        .get("track_player")
        .or_else(|| seo.and_then(|node| node.get("track_player")));
    if let Some(player) = player {
        root.insert("track_player".into(), player.clone());
    }
    let lyric = seo
        .and_then(|node| node.get("lyric"))
        .or_else(|| value.get("lyric"));
    if let Some(lyric) = lyric {
        root.insert("lyric".into(), lyric.clone());
    }
    Some(Value::Object(root))
}

/// 曲目详情优先取 `track`，`id` 为空时才用 `track_info`。
pub fn primary_track(payload: &Value) -> Option<&Value> {
    let primary = &payload["track"];
    if primary["id"].is_string() && !primary["id"].as_str().unwrap_or("").is_empty() {
        return Some(primary);
    }
    payload["track_info"].as_object().map(|_| &payload["track_info"])
}

/// 完整曲目时长（秒），用于识别试听片段。
pub fn track_duration(track: &Value) -> u64 {
    normalize_seconds(track["duration"].as_u64().unwrap_or(0))
}

/// 收集全部可用流：video_model 优先，url_player_info 作为补充。
pub async fn players(payload: &Value, full_duration: u64) -> Vec<Player> {
    let player_node = &payload["track_player"];
    let mut collected = Vec::new();
    if let Some(model) = player_node.get("video_model") {
        if let Some(value) = decode_nested_json(model) {
            collect_from_value(&value, 0, full_duration, &mut collected);
        }
    }
    if let Some(info_url) = player_node["url_player_info"]
        .as_str()
        .filter(|url| url.starts_with("http"))
    {
        if let Ok(body) = client::get_json(info_url).await {
            collect_from_play_info(&body, full_duration, &mut collected);
        }
    }
    dedup(collected)
}

/// `video_model` 可能是对象、数组，或被转义一至三层的 JSON 字符串。
fn decode_nested_json(value: &Value) -> Option<Value> {
    let mut current = value.clone();
    for _ in 0..3 {
        let Some(text) = current.as_str() else {
            return Some(current);
        };
        let text = text.trim();
        if text.is_empty() || text == "null" {
            return None;
        }
        current = serde_json::from_str(text).ok()?;
    }
    Some(current)
}

/// 递归收集所有带播放地址的对象，兼容 snake_case 与 camelCase 两套字段。
///
/// `video_duration` 只出现在 `video_model` 顶层，逐层向下传递，供试听判定使用。
fn collect_from_value(value: &Value, inherited_duration: u64, full_duration: u64, out: &mut Vec<Player>) {
    match value {
        Value::Object(map) => {
            let duration = match number_of(map, &["video_duration", "videoDuration"]) {
                0 => inherited_duration,
                value => value,
            };
            if let Some(player) = player_from_map(map, duration, full_duration) {
                out.push(player);
            }
            for child in map.values() {
                collect_from_value(child, duration, full_duration, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_from_value(item, inherited_duration, full_duration, out);
            }
        }
        _ => {}
    }
}

/// 播放变体的元信息集中在 `video_meta` 里，码率、容器和大小都要从这里读。
fn player_from_map(map: &Map<String, Value>, duration: u64, full_duration: u64) -> Option<Player> {
    let url = text_of(map, &["main_play_url", "mainPlayUrl", "main_url", "mainUrl"])
        .or_else(|| text_of(map, &["backup_play_url", "backupPlayUrl", "backup_url"]))?;
    if !url.starts_with("http") {
        return None;
    }
    let meta = map.get("video_meta").and_then(Value::as_object);
    let pick = |keys: &[&str]| -> Option<String> {
        text_of(map, keys).or_else(|| meta.and_then(|meta| text_of(meta, keys)))
    };
    let number = |keys: &[&str]| -> u64 {
        let value = number_of(map, keys);
        if value > 0 {
            value
        } else {
            meta.map_or(0, |meta| number_of(meta, keys))
        }
    };
    let play_auth = text_of(map, &["play_auth", "playAuth"])
        .or_else(|| {
            map.get("encrypt_info")
                .and_then(|info| text_of(info.as_object()?, &["spade_a", "spadeA"]))
        })
        .unwrap_or_default();
    let mut player = Player {
        url,
        play_auth,
        format: pick(&["format", "vtype", "file_format", "extension", "ext"]).unwrap_or_default(),
        bitrate: normalize_bitrate(number(&["bitrate", "bit_rate", "br", "bitRate"])),
        size: number(&["size", "file_size", "fileSize", "data_size"]),
        duration: normalize_seconds(duration),
        quality: pick(&["quality", "quality_level", "gear_name", "audio_quality"])
            .unwrap_or_default(),
        preview: false,
    };
    player.preview = is_preview(&player, full_duration);
    Some(player)
}

/// `url_player_info` 返回 PascalCase 结构，单独解析。
fn collect_from_play_info(body: &Value, full_duration: u64, out: &mut Vec<Player>) {
    let Some(list) = body["Result"]["Data"]["PlayInfoList"].as_array() else {
        return;
    };
    for item in list {
        let Some(map) = item.as_object() else {
            continue;
        };
        let Some(url) = text_of(map, &["MainPlayUrl", "BackupPlayUrl", "main_play_url"]) else {
            continue;
        };
        if !url.starts_with("http") {
            continue;
        }
        let mut player = Player {
            url,
            play_auth: text_of(map, &["PlayAuth", "play_auth"]).unwrap_or_default(),
            format: text_of(map, &["Format", "format"]).unwrap_or_default(),
            bitrate: normalize_bitrate(number_of(map, &["Bitrate", "bitrate"])),
            size: number_of(map, &["Size", "size"]),
            duration: normalize_seconds(number_of(map, &["Duration", "duration"])),
            quality: text_of(map, &["Quality", "quality"]).unwrap_or_default(),
            preview: false,
        };
        player.preview = is_preview(&player, full_duration);
        out.push(player);
    }
}

fn text_of(map: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|key| map.get(*key))
        .filter_map(Value::as_str)
        .map(str::trim)
        .find(|value| !value.is_empty() && *value != "null")
        .map(str::to_string)
}

fn number_of(map: &Map<String, Value>, keys: &[&str]) -> u64 {
    keys.iter()
        .filter_map(|key| map.get(*key))
        .find_map(|value| {
            value
                .as_u64()
                .or_else(|| value.as_f64().map(|number| number.round() as u64))
                .or_else(|| {
                    value
                        .as_str()
                        .and_then(|text| text.trim().parse::<f64>().ok())
                        .map(|number| number.round() as u64)
                })
        })
        .unwrap_or(0)
}

/// 毫秒转秒；接口同时存在秒与毫秒两种返回，用 10000 作为分界。
fn normalize_seconds(raw: u64) -> u64 {
    if raw > 10_000 {
        raw / 1000
    } else {
        raw
    }
}

/// bps 转 kbps，接口可能给 320 或 320000 两种量级。
fn normalize_bitrate(raw: u64) -> u32 {
    let value = if raw > 10_000 { raw / 1000 } else { raw };
    u32::try_from(value).unwrap_or(0)
}

/// 播放流明显短于曲目时长即视为试听；差值超过 5 秒且不足 90% 才判定，
/// 避免把时长字段不一致的正常曲目误标。
fn is_preview(player: &Player, full_duration: u64) -> bool {
    player.duration > 0
        && full_duration > 0
        && player.duration + 5 < full_duration
        && player.duration * 10 < full_duration * 9
}

/// 同一曲目可能从两个来源拿到重复流，按 URL 去重，并优先保留非试听、码率更高的那条。
fn dedup(players: Vec<Player>) -> Vec<Player> {
    let mut result: Vec<Player> = Vec::new();
    for player in players {
        match result.iter_mut().find(|item| item.url == player.url) {
            Some(existing) => {
                if existing.preview && !player.preview {
                    *existing = player;
                } else if existing.preview == player.preview && player.bitrate > existing.bitrate {
                    *existing = player;
                }
            }
            None => result.push(player),
        }
    }
    result.sort_by(|a, b| b.bitrate.cmp(&a.bitrate));
    result
}

/// 转成项目统一的歌曲对象。`id` 固定为 0：19 位 ID 超出 JS 安全整数，
/// 真实标识只放 `mid`，下载、歌词和去重都走它。
pub fn song(track: &Value, separator: &str, players: &[Player], track_id: &str) -> Value {
    let artists: Vec<Value> = track["artists"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let name = item["name"].as_str().unwrap_or("").trim();
                    if name.is_empty() {
                        return None;
                    }
                    Some(json!({
                        "id": item["id"].as_str().map(str::to_string).unwrap_or_else(|| item["id"].to_string()),
                        "mid": "",
                        "name": name,
                        "coverUrl": avatar(item),
                    }))
                })
                .collect()
        })
        .unwrap_or_default();
    let artist = artists
        .iter()
        .filter_map(|item| item["name"].as_str())
        .collect::<Vec<_>>()
        .join(separator);
    let album = &track["album"];
    let duration = track_duration(track);
    let preview = !players.is_empty() && players.iter().all(|item| item.preview);

    let mut seen = Vec::new();
    let mut qualities = Vec::new();
    for player in players {
        let Some(label) = player.quality_label() else {
            continue;
        };
        let filename = player.filename();
        if filename.is_empty() || seen.iter().any(|item| item == &filename) {
            continue;
        }
        let size = if player.size > 0 {
            player.size
        } else {
            u64::from(player.bitrate) * 125 * player.duration
        };
        seen.push(filename.clone());
        qualities.push(json!({
            "quality": label,
            "format": m4a_or(&player.format),
            "size": size,
            "filename": filename,
        }));
    }

    json!({
        "id": 0,
        "mid": track_id,
        "title": track["name"].as_str().unwrap_or_default(),
        "artist": artist,
        "artists": artists,
        "album": album["name"].as_str().unwrap_or_default(),
        "albumId": album["id"].as_str().map(str::to_string).unwrap_or_default(),
        "albumMid": "",
        "duration": duration,
        "coverUrl": cover(track),
        "mediaMid": track["vid"].as_str().unwrap_or(track_id),
        "qualities": qualities,
        "preview": preview,
    })
}

fn avatar(item: &Value) -> String {
    item.get("url_avatar")
        .or_else(|| item.get("avatar_url"))
        .or_else(|| item.get("urlAvatar"))
        .map(pick_url)
        .unwrap_or_default()
}

fn cover(track: &Value) -> String {
    let album = &track["album"];
    album
        .get("url_cover")
        .or_else(|| album.get("cover_url"))
        .or_else(|| track.get("url_cover"))
        .or_else(|| track.get("cover_url"))
        .map(pick_url)
        .unwrap_or_default()
}

/// 图片地址有三种形态：直接字符串、`urls` 数组，以及
/// `{uri, urls: [前缀], template_prefix}` 的对象形式。
///
/// 对象形式必须补上字节 CDN 的尺寸模板后缀，只拼 `前缀 + uri` 会返回
/// `400 fail to handle request`；`~c5_1080x1080.jpg` 是分享页同款模板的最大档。
fn pick_url(value: &Value) -> String {
    if let Some(text) = value.as_str() {
        return text.to_string();
    }
    if let Some(items) = value.as_array() {
        return items.first().map(pick_url).unwrap_or_default();
    }
    let Some(object) = value.as_object() else {
        return String::new();
    };
    let prefix = object
        .get("urls")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(Value::as_str)
        .unwrap_or("");
    let uri = object.get("uri").and_then(Value::as_str).unwrap_or("");
    if !prefix.is_empty() {
        if uri.is_empty() {
            return prefix.to_string();
        }
        // 已带扩展名的 uri 是完整路径，不再追加模板。
        if uri.contains('.') && !uri.ends_with('/') {
            return format!("{prefix}{uri}");
        }
        return format!("{prefix}{uri}~c5_1080x1080.jpg");
    }
    object
        .get("url")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// 歌词正文可能在顶层 `lyric.content`，也可能嵌在 `seo_track.track.lyric.content`。
pub fn lyric_text(payload: &Value) -> Option<String> {
    let candidates = [
        &payload["lyric"]["content"],
        &payload["seo_track"]["lyric"]["content"],
        &payload["track"]["lyric"]["content"],
        &payload["track_info"]["lyric"]["content"],
    ];
    candidates
        .iter()
        .filter_map(|value| value.as_str())
        .map(str::trim)
        .find(|text| !text.is_empty())
        .map(str::to_string)
}

/// 汽水返回逐字歌词：`[3150,5330]<0,410,0>隐<1160,410,0>去…`。
///
/// 标准 LRC 只保留行级时间轴，逐字标签必须去掉，否则播放器无法解析。
/// 已经是标准 LRC 的响应原样返回，避免二次处理破坏内容。
pub fn lyric_to_lrc(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if STANDARD_LYRIC_LINE.is_match(raw) {
        return Some(raw.to_string());
    }
    let mut output = String::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(captured) = WORD_LYRIC_LINE.captures(line) {
            let start = captured[1].parse::<u64>().unwrap_or(0);
            let text = LYRIC_WORD_TAG.replace_all(&captured[3], "");
            let text = text.trim();
            if text.is_empty() {
                continue;
            }
            output.push_str(&lrc_timestamp(start));
            output.push_str(text);
            output.push('\n');
        }
    }
    if output.is_empty() {
        return None;
    }
    Some(output)
}

fn lrc_timestamp(milliseconds: u64) -> String {
    let minutes = milliseconds / 60_000;
    let seconds = (milliseconds % 60_000) / 1000;
    let centiseconds = (milliseconds % 1000) / 10;
    format!("[{minutes:02}:{seconds:02}.{centiseconds:02}]")
}

static STANDARD_LYRIC_LINE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^\[\d{1,2}:\d{2}(?:[.:]\d+)?\]").expect("标准 LRC 行正则"));
static WORD_LYRIC_LINE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\[(\d+),(\d+)\](.*)$").expect("汽水逐字歌词行正则"));
static LYRIC_WORD_TAG: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<\d+,\d+,\d+>").expect("汽水逐字标签正则"));

#[cfg(test)]
mod tests {
    use super::*;

    fn player(duration: u64, bitrate: u32, format: &str, quality: &str) -> Player {
        Player {
            url: "https://cdn.example/audio".into(),
            play_auth: String::new(),
            format: format.into(),
            bitrate,
            size: 0,
            duration,
            quality: quality.into(),
            preview: false,
        }
    }

    #[test]
    fn durations_accept_seconds_and_milliseconds() {
        assert_eq!(normalize_seconds(242), 242);
        assert_eq!(normalize_seconds(242_000), 242);
        assert_eq!(normalize_bitrate(320), 320);
        assert_eq!(normalize_bitrate(320_000), 320);
    }

    #[test]
    fn quality_labels_follow_format_and_bitrate() {
        assert_eq!(player(200, 900, "flac", "lossless").quality_label(), Some("flac"));
        assert_eq!(player(200, 1200, "flac", "hires").quality_label(), Some("hires"));
        assert_eq!(player(200, 320, "m4a", "").quality_label(), Some("320kaac"));
        assert_eq!(player(200, 128, "m4a", "").quality_label(), Some("128kaac"));
        assert_eq!(player(200, 64, "m4a", "").quality_label(), Some("64kaac"));
        assert_eq!(player(200, 192, "m4a", "spatial").quality_label(), Some("spatial"));
    }

    #[test]
    fn filenames_match_the_task_quality_contract() {
        assert_eq!(player(200, 320, "m4a", "").filename(), "320.m4a");
        assert_eq!(player(200, 900, "flac", "lossless").filename(), "flac.flac");
        assert_eq!(player(200, 1200, "flac", "hires").filename(), "hires.flac");
    }

    #[test]
    fn short_streams_are_marked_as_preview() {
        let mut short = player(30, 128, "m4a", "");
        short.preview = is_preview(&short, 242);
        assert!(short.preview);
        // 试听片段仍给出档位，任务才能创建；片段身份由 preview 字段表达。
        assert_eq!(short.quality_label(), Some("128kaac"));

        let mut full = player(242, 128, "m4a", "");
        full.preview = is_preview(&full, 242);
        assert!(!full.preview);
    }

    #[test]
    fn nested_video_model_is_unwrapped() {
        let escaped = json!(json!({"video_list":[{"main_url":"https://cdn.example/a.m4a"}]}).to_string());
        let decoded = decode_nested_json(&escaped).unwrap();
        let mut players = Vec::new();
        collect_from_value(&decoded, 0, 200, &mut players);
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].url, "https://cdn.example/a.m4a");
    }

    #[test]
    fn video_meta_carries_bitrate_and_size() {
        let model = json!({
            "video_duration": 60.001,
            "video_list": [{
                "main_url": "https://cdn.example/a.m4a",
                "video_meta": {
                    "quality": "highest",
                    "vtype": "m4a",
                    "bitrate": 257695,
                    "size": 1932750
                }
            }]
        });
        let mut players = Vec::new();
        collect_from_value(&model, 0, 180, &mut players);
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].bitrate, 257);
        assert_eq!(players[0].quality, "highest");
        assert_eq!(players[0].format, "m4a");
        assert_eq!(players[0].size, 1932750);
        // 时长只出现在 video_model 顶层，需要向下继承，试听判定才能生效。
        assert_eq!(players[0].duration, 60);
        assert!(players[0].preview);
    }

    #[test]
    fn word_lyrics_are_converted_to_standard_lrc() {
        let raw = "[3150,5330]<0,410,0>隐<1160,410,0>去了离别\n[9000,2000]<0,300,0>第二行";
        let lrc = lyric_to_lrc(raw).unwrap();
        assert!(lrc.starts_with("[00:03.15]隐去了离别"), "{lrc}");
        assert!(lrc.contains("[00:09.00]第二行"), "{lrc}");
    }

    #[test]
    fn standard_lrc_is_returned_untouched() {
        let raw = "[00:01.00]已经是标准 LRC\n[00:02.50]第二行";
        assert_eq!(lyric_to_lrc(raw).unwrap(), raw);
    }
}
