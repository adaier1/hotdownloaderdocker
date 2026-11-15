//! 普通歌词、逐字歌词转换和纯音乐识别。
use super::client::request;
use crate::platforms::lyric::LyricData;
use serde_json::{json, Value};

pub async fn get_lyric_by_id(song_id: u64) -> Result<LyricData, String> {
    if song_id == 0 {
        return Err("无效的网易云歌曲 ID".into());
    }
    let data = request(
        "/api/song/lyric/v1",
        json!({
            "id": song_id,
            "cp": false,
            "tv": 0,
            "lv": 0,
            "rv": 0,
            "kv": 0,
            "yv": 0,
            "ytv": 0,
            "yrv": 0
        }),
        "",
        true,
    )
    .await?
    .0;
    Ok(parse_lyric(&data))
}

fn parse_lyric(data: &Value) -> LyricData {
    let lrc = data["lrc"]["lyric"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string);
    let yrc = data["yrc"]["lyric"]
        .as_str()
        .filter(|s| !s.trim().is_empty());
    let instrumental = data["nolyric"].as_bool().unwrap_or(false)
        || lrc.as_deref().is_some_and(|s| s.contains("纯音乐，请欣赏"));
    let elrc = yrc.map(yrc_to_elrc).filter(|s| !s.is_empty());
    LyricData {
        lrc,
        elrc,
        raw: yrc.map(str::to_string),
        instrumental,
    }
}

fn timestamp(ms: u64, enhanced: bool) -> String {
    let time = format!("{:02}:{:02}.{:03}", ms / 60000, (ms / 1000) % 60, ms % 1000);
    if enhanced {
        format!("<{time}>")
    } else {
        format!("[{time}]")
    }
}
fn yrc_to_elrc(raw: &str) -> String {
    let line = regex::Regex::new(r"^\[(\d+),(\d+)\]").unwrap();
    let word = regex::Regex::new(r"\((\d+),(\d+),\d+\)([^()]*)").unwrap();
    raw.lines()
        .filter_map(|text| {
            let header = line.captures(text)?;
            let mut output = timestamp(header[1].parse().ok()?, false);
            for part in word.captures_iter(&text[header[0].len()..]) {
                output.push_str(&timestamp(part[1].parse().ok()?, true));
                output.push_str(&part[3]);
            }
            Some(output)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn english_and_translation_preserve_absolute_yrc_word_times() {
        let raw = "[0,1000](0,500,0)First (500,500,0)line\n[0,1000](0,1000,0)第一行\n[1000,1000](1000,500,0)Second (1500,500,0)line\n[1000,1000](1000,1000,0)第二行";
        assert_eq!(yrc_to_elrc(raw), "[00:00.000]<00:00.000>First <00:00.500>line\n[00:00.000]<00:00.000>第一行\n[00:01.000]<00:01.000>Second <00:01.500>line\n[00:01.000]<00:01.000>第二行");
    }

    #[test]
    fn lyric_converts_word_times_to_enhanced_lrc() {
        let lyric = parse_lyric(&json!({
            "lrc": {"lyric": "[00:01.000]你好"},
            "yrc": {"lyric": "[1000,1000](1000,500,0)你(1500,500,0)好"}
        }));
        assert_eq!(
            lyric.elrc.as_deref(),
            Some("[00:01.000]<00:01.000>你<00:01.500>好")
        );
        assert!(!lyric.instrumental);
    }
}
