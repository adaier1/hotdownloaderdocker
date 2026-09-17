use std::fmt::Display;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum AudioError {
    #[error("Require at least {0} bytes of header.")]
    NeedMoreHeader(usize),
}

pub const MASK_LOSSLESS: u32 = 0x80000000;

#[repr(u32)]
#[derive(Debug, PartialEq, Eq)]
pub enum AudioType {
    Unknown = 0,

    // Lossy
    OGG = 1,
    AAC = 2,
    MP3 = 3,
    M4A = 4,
    M4B = 5,
    MP4 = 6,
    WMA = 7, // While possible, it is rare to find a lossless WMA file.
    MKV = 8, // Matroska (mkv, mka, webm etc.; can contain lossy or lossless audio tracks)

    // Lossless
    FLAC = MASK_LOSSLESS | 1,
    DFF = MASK_LOSSLESS | 2,
    WAV = MASK_LOSSLESS | 3,
    APE = MASK_LOSSLESS | 5,
}

impl AudioType {
    pub fn as_str(&self) -> &str {
        match self {
            AudioType::OGG => "ogg",
            AudioType::AAC => "aac",
            AudioType::MP3 => "mp3",
            AudioType::M4A => "m4a",
            AudioType::M4B => "m4b",
            AudioType::MP4 => "mp4",
            AudioType::WMA => "wma",
            AudioType::FLAC => "flac",
            AudioType::DFF => "dff",
            AudioType::WAV => "wav",
            AudioType::APE => "ape",
            AudioType::MKV => "mka",

            _ => "bin",
        }
    }
}

impl Display for AudioType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
