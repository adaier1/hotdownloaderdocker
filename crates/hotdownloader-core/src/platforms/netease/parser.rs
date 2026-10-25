use serde_json::{json, Value};

pub(super) fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}
pub(super) fn list(v: &Value) -> &[Value] {
    v.as_array().map(Vec::as_slice).unwrap_or(&[])
}
fn artist_list(v: &Value) -> &[Value] {
    list(
        v.get("ar")
            .or_else(|| v.get("artists"))
            .unwrap_or(&Value::Null),
    )
}
fn artist_names(v: &Value, separator: &str) -> String {
    artist_list(v)
        .iter()
        .filter_map(|a| a["name"].as_str())
        .collect::<Vec<_>>()
        .join(separator)
}
fn references(v: &Value) -> Vec<Value> {
    artist_list(v)
        .iter()
        .map(|a| {
            json!({
                "id": text(&a["id"]),
                "name": a["name"],
                "coverUrl": a["picUrl"].as_str().unwrap_or("")
            })
        })
        .collect()
}

// Platform-specific level names are kept in filenames so retries and persisted tasks
// always resolve the same requested quality.
pub(super) const LEVELS: &[(&str, &str, &str, &str)] = &[
    ("l", "128kmp3", "standard", "mp3"),
    ("h", "320kmp3", "exhigh", "mp3"),
    ("sq", "flac", "lossless", "flac"),
    ("hr", "hires", "hires", "flac"),
    ("je", "高清环绕声", "jyeffect", "flac"),
    ("sk", "沉浸环绕声", "sky", "flac"),
    ("jm", "超清母带", "jymaster", "flac"),
];

pub(super) fn song(v: &Value, separator: &str) -> Value {
    let album = v
        .get("al")
        .or_else(|| v.get("album"))
        .unwrap_or(&Value::Null);
    let id = text(&v["id"]);
    let qualities: Vec<_> = LEVELS
        .iter()
        .filter_map(|(field, quality, level, ext)| {
            let size = v[*field]["size"].as_u64().unwrap_or(0);
            (size > 0).then(|| {
                json!({
                    "quality": quality,
                    "filename": format!("{id}_{level}.{ext}"),
                    "size": size
                })
            })
        })
        .collect();
    json!({
        "platform": "netease",
        "id": v["id"],
        "mid": id,
        "mediaMid": id,
        "title": v["name"],
        "artist": artist_names(v,separator),
        "artists": references(v),
        "album": album["name"].as_str().unwrap_or(""),
        "albumId": text(&album["id"]),
        "coverUrl": album["picUrl"].as_str().unwrap_or(""),
        "qualities": qualities
    })
}

pub(super) fn album(v: &Value, separator: &str) -> Value {
    let mut artists = references(v);
    if artists.is_empty() && v["artist"].is_object() {
        artists.push(json!({
            "id": text(&v["artist"]["id"]),
            "name": v["artist"]["name"]
        }));
    }
    let names = artists
        .iter()
        .filter_map(|a| a["name"].as_str())
        .collect::<Vec<_>>()
        .join(separator);
    json!({
        "id": text(&v["id"]),
        "name": v["name"],
        "artist": names,
        "artists": artists,
        "coverUrl": v["picUrl"].as_str().unwrap_or(""),
        "publishDate": date(v["publishTime"].as_i64().unwrap_or(0)),
        "songCount": v.get("size").or_else(||v.get("songCount")).and_then(Value::as_u64).unwrap_or(0)
    })
}

// Gregorian civil date from Unix days, without adding a date/time runtime dependency.
fn date(millis: i64) -> String {
    if millis <= 0 {
        return String::new();
    }
    let z = millis / 86_400_000 + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

pub(super) fn artist(v: &Value) -> Value {
    json!({
        "id": text(&v["id"]),
        "name": v["name"],
        "coverUrl": v["picUrl"].as_str().or(v["img1v1Url"].as_str()).unwrap_or(""),
        "alias": list(&v["alias"]).iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" / "),
        "region": "",
        "songCount": v["musicSize"].as_u64().unwrap_or(0),
        "albumCount": v["albumSize"].as_u64().unwrap_or(0)
    })
}
pub(super) fn playlist(v: &Value) -> Value {
    json!({
        "id": text(&v["id"]),
        "name": v["name"],
        "platform": "netease",
        "creator": v["creator"]["nickname"].as_str().unwrap_or(""),
        "coverUrl": v["coverImgUrl"].as_str().unwrap_or(""),
        "songCount": v["trackCount"].as_u64().unwrap_or(0),
        "playCount": v["playCount"].as_u64().unwrap_or(0),
        "createdAt": v["createTime"].as_u64().unwrap_or(0)/1000,
        "updatedAt": v["updateTime"].as_u64().unwrap_or(0)/1000,
        "introduction": v["description"].as_str().unwrap_or("")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn songs_preserve_relations_and_only_advertise_reported_qualities() {
        let parsed = song(
            &json!({
                "id": 123,
                "name": "歌",
                "ar": [{"id": 7,
                    "name": "甲"},{"id": 8,
                    "name": "乙"}],
                "al": {"id": 9,
                    "name": "专辑",
                    "picUrl": "https://p1.music.126.net/a.jpg"},
                "l": {"size": 1234},
                "sq": {"size": 6789}
            }),
            " / ",
        );
        assert_eq!(parsed["artist"], "甲 / 乙");
        assert_eq!(parsed["albumId"], "9");
        assert_eq!(parsed["artists"][1]["id"], "8");
        assert_eq!(parsed["qualities"][1]["filename"], "123_lossless.flac");
        assert_eq!(parsed["qualities"].as_array().unwrap().len(), 2);
        assert_eq!(date(1704067200000), "2024-01-01");
    }
}
