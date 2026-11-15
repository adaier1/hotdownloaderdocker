use super::client::{number, text};
use serde_json::{json, Value};

fn field<'a>(v: &'a Value, paths: &[&str]) -> &'a Value {
    paths
        .iter()
        .filter_map(|p| v.pointer(p))
        .find(|v| !v.is_null() && v.as_str() != Some(""))
        .unwrap_or(&Value::Null)
}
fn string(v: &Value, paths: &[&str]) -> String {
    text(field(v, paths))
}
pub(super) fn cover(v: &Value) -> String {
    string(
        v,
        &[
            "/Image",
            "/sizable_cover",
            "/img",
            "/Avatar",
            "/avatar",
            "/pic",
            "/pic_url",
            "/album_info/cover",
            "/trans_param/union_cover",
            "/cover",
        ],
    )
    .replace("{size}", "300")
    .replace("http://", "https://")
}
pub(super) fn artists(v: &Value) -> Vec<Value> {
    if let Some(items) = field(v, &["/Singers", "/singers", "/authors", "/singerinfo"]).as_array() {
        return items
            .iter()
            .filter_map(|item| {
                let item = item.get("base").unwrap_or(item);
                let name = string(item, &["/name", "/author_name"]);
                if name.is_empty() {
                    return None;
                }
                Some(json!({
                    "id": string(item, &["/id", "/author_id"]),
                    "mid": "",
                    "name": name,
                    "coverUrl": cover(item)
                }))
            })
            .collect();
    }
    let name = string(
        v,
        &[
            "/SingerName",
            "/singer",
            "/author_name",
            "/base/author_name",
        ],
    );
    name.split('、')
        .filter(|s| !s.is_empty())
        .map(|s| {
            json!({
                "id": "",
                "mid": "",
                "name": s,
                "coverUrl": ""
            })
        })
        .collect()
}
pub(super) fn valid_hash(hash: &str) -> bool {
    hash.len() == 32 && hash.bytes().all(|b| b.is_ascii_hexdigit())
}

/// filename 持久化选中音质的 hash、专辑 ID 和 album_audio_id；mid 保留原始 hash 供歌词使用。
pub fn parse_song(v: &Value, separator: &str) -> Option<Value> {
    let hash = string(
        v,
        &[
            "/FileHash",
            "/hash",
            "/audio_info/hash",
            "/audio_info/hash_128",
        ],
    );
    if !valid_hash(&hash) {
        return None;
    }
    let id = number(field(
        v,
        &[
            "/MixSongID",
            "/mixsongid",
            "/album_audio_id",
            "/base/album_audio_id",
            "/ID",
        ],
    ));
    let album_id = string(
        v,
        &["/AlbumID", "/album_id", "/base/album_id", "/albuminfo/id"],
    );
    let album_id = album_id.parse::<u64>().unwrap_or(0).to_string();
    let mut refs = artists(v);
    let mut title = string(v, &["/SongName", "/audio_name", "/base/audio_name"]);
    if title.is_empty() {
        let name = string(v, &["/FileName", "/name"]);
        if let Some((artist, song)) = name.split_once(" - ") {
            title = song.to_owned();
            if refs.is_empty() {
                refs.push(json!({
                    "id": "",
                    "mid": "",
                    "name": artist,
                    "coverUrl": ""
                }));
            }
        } else {
            title = name;
        }
    }
    let artist = refs
        .iter()
        .filter_map(|a| a["name"].as_str())
        .collect::<Vec<_>>()
        .join(separator);
    let mut qualities = Vec::new();
    let mut add = |hash: String, size: u64, quality: &str, format: &str| {
        if valid_hash(&hash)
            && size > 0
            && !qualities.iter().any(|v: &Value| v["quality"] == quality)
        {
            qualities.push(json!({
                "quality": quality,
                "format": format,
                "size": size,
                "filename": format!("{}_{}_{}_{}.{}", hash.to_lowercase(), album_id, id, quality, format)
            }));
        }
    };
    for (hash_paths, size_paths, quality, format) in [
        (
            vec![
                "/FileHash",
                "/hash",
                "/audio_info/hash",
                "/audio_info/hash_128",
            ],
            vec![
                "/FileSize",
                "/size",
                "/audio_info/filesize",
                "/audio_info/filesize_128",
            ],
            "128kmp3",
            "mp3",
        ),
        (
            vec!["/HQFileHash", "/audio_info/hash_320"],
            vec!["/HQFileSize", "/audio_info/filesize_320"],
            "320kmp3",
            "mp3",
        ),
        (
            vec!["/SQFileHash", "/audio_info/hash_flac"],
            vec!["/SQFileSize", "/audio_info/filesize_flac"],
            "flac",
            "flac",
        ),
        (
            vec!["/ResFileHash", "/audio_info/hash_high"],
            vec!["/ResFileSize", "/audio_info/filesize_high"],
            "hires",
            "flac",
        ),
    ] {
        add(
            string(v, &hash_paths),
            number(field(v, &size_paths)),
            quality,
            format,
        );
    }
    if let Some(items) = v["relate_goods"].as_array() {
        for item in items {
            let (quality, format) = match number(&item["level"]) {
                2 => ("128kmp3", "mp3"),
                4 => ("320kmp3", "mp3"),
                5 => ("flac", "flac"),
                6 => ("hires", "flac"),
                _ => continue,
            };
            add(text(&item["hash"]), number(&item["size"]), quality, format);
        }
    }
    Some(json!({
        "id": id,
        "mid": hash.to_lowercase(),
        "title": title,
        "artist": artist,
        "artists": refs,
        "album": string(v,&["/AlbumName","/album_info/album_name","/albuminfo/name"]),
        "albumId": album_id,
        "albumMid": "",
        "coverUrl": cover(v),
        "mediaMid": hash.to_lowercase(),
        "qualities": qualities
    }))
}

pub(super) fn album(v: &Value, separator: &str) -> Value {
    let refs = artists(v);
    json!({
        "id": string(v,&["/albumid","/album_id"]),
        "name": string(v,&["/albumname","/album_name"]),
        "artist": refs.iter().filter_map(|a| a["name"].as_str()).collect::<Vec<_>>().join(separator),
        "artists": refs,
        "coverUrl": cover(v),
        "publishDate": string(v,&["/publish_time","/publish_date"]),
        "songCount": number(field(v,&["/songcount","/song_count","/audio_total"])),
        "description": string(v,&["/intro"])
    })
}
pub(super) fn artist(v: &Value) -> Value {
    json!({
        "id": string(v,&["/AuthorId","/author_id"]),
        "name": string(v,&["/AuthorName","/author_name"]),
        "coverUrl": cover(v),
        "alias": "",
        "region": "",
        "songCount": number(&v["AudioCount"]),
        "albumCount": number(&v["AlbumCount"])
    })
}
pub(super) fn playlist(v: &Value) -> Value {
    json!({
        "id": string(v,&["/gid","/global_collection_id"]),
        "name": string(v,&["/specialname","/name","/listname"]),
        "creator": string(v,&["/nickname","/list_create_username"]),
        "coverUrl": cover(v),
        "songCount": number(field(v,&["/song_count","/count"])),
        "playCount": number(field(v,&["/total_play_count","/play_count","/heat"]))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detail_song_keeps_hash_and_related_entities() {
        let v = json!({
            "base": {
                "album_audio_id": 123,
                "album_id": 456,
                "audio_name": "测试"
            },
            "audio_info": {
                "hash": "0123456789ABCDEF0123456789ABCDEF",
                "filesize": 1024,
                "hash_flac": "ABCDEF0123456789ABCDEF0123456789",
                "filesize_flac": 2048
            },
            "authors": [{
                "author_id": 12,
                "author_name": "甲"
            },
            {
                "author_id": 13,
                "author_name": "乙"
            }],
            "album_info": {
                "album_name": "专辑",
                "cover": "http://imge.kugou.com/{size}/cover.jpg"
            }
        });
        let song = parse_song(&v, " / ").unwrap();
        assert_eq!(song["artist"], "甲 / 乙");
        assert_eq!(song["id"], 123);
        assert_eq!(song["albumId"], "456");
        assert_eq!(
            song["qualities"][1]["filename"],
            "abcdef0123456789abcdef0123456789_456_123_flac.flac"
        );
        assert_eq!(song["coverUrl"], "https://imge.kugou.com/300/cover.jpg");
        assert!(parse_song(
            &json!({
                "hash": "bad"
            }),
            "、"
        )
        .is_none());
    }
}
