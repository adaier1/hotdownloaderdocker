mod aac;
mod audio_type;
mod metadata;
mod mp3;

use crate::aac::SYNC_FRAME_TEST_SIZE;
use aac::is_aac;
pub use audio_type::{AudioError, AudioType};

const MAGIC_FLAC: [u8; 4] = *b"fLaC";
const MAGIC_OGG: [u8; 4] = *b"OggS";
const MAGIC_DFF: [u8; 4] = *b"FRM8";
const MAGIC_WMA: [u8; 4] = [0x30, 0x26, 0xB2, 0x75];
const MAGIC_WAV: [u8; 4] = *b"RIFF";
const MAGIC_APE: [u8; 4] = *b"MAC ";
const MAGIC_MKV: [u8; 4] = [0x1A, 0x45, 0xDF, 0xA3];

pub fn detect_audio_type(buffer: &[u8]) -> Result<AudioType, AudioError> {
    let offset = metadata::get_header_metadata_size(buffer, 0)?;
    if buffer.len() < offset + 0x10 {
        Err(AudioError::NeedMoreHeader(offset + 0x10))?;
    }
    let buffer = &buffer[offset..];
    let mut magic = [0u8; 4];
    magic.copy_from_slice(&buffer[..4]);
    match magic {
        MAGIC_FLAC => return Ok(AudioType::FLAC),
        MAGIC_OGG => return Ok(AudioType::OGG),
        MAGIC_DFF => return Ok(AudioType::DFF),
        MAGIC_WMA => return Ok(AudioType::WMA),
        MAGIC_WAV => return Ok(AudioType::WAV),
        MAGIC_APE => return Ok(AudioType::APE),
        MAGIC_MKV => return Ok(AudioType::MKV),
        _ => {}
    }
    let magic = u32::from_be_bytes(magic);
    if is_aac(magic) {
        return Ok(AudioType::AAC);
    } else if mp3::is_mp3(buffer) {
        return Ok(AudioType::MP3);
    }

    // MP4 Containers
    if &buffer[0x04..0x08] == b"ftyp" {
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&buffer[0x08..0x0c]);
        match &magic {
            // MSNV: SonyPSP
            // isom / iso2: MP4 (Generic?)
            b"isom" | b"iso2" | b"MSNV" => return Ok(AudioType::MP4),
            b"NDAS" => return Ok(AudioType::M4A), // Nero Digital AAC Audio
            _ => {}
        };

        let mut magic = [0u8; 3];
        magic.copy_from_slice(&buffer[0x08..0x0b]);
        match &magic {
            b"M4A" => return Ok(AudioType::M4A), // iTunes AAC-LC Audio
            b"M4B" => return Ok(AudioType::M4B), // iTunes AAC-LC Audio
            b"mp4" => return Ok(AudioType::MP4), // QQMusic
            _ => {}
        };
    }

    // Ask for more data to test for MP3
    if buffer.len() < SYNC_FRAME_TEST_SIZE {
        return Err(AudioError::NeedMoreHeader(offset + SYNC_FRAME_TEST_SIZE));
    }

    Ok(AudioType::Unknown)
}

#[cfg(test)]
mod tests {
    use crate::{detect_audio_type, AudioType};

    #[test]
    fn test_mp3() {
        let mp3_data = include_bytes!("__fixtures__/ffmpeg_silent.mp3");
        let result = detect_audio_type(mp3_data).expect("failed to parse mp3");
        assert_eq!(result, AudioType::MP3);
    }

    #[test]
    fn test_mp3_multiple_id3() {
        let mp3_data = include_bytes!("__fixtures__/mp3_with_id3v2_x3.bin");
        let result = detect_audio_type(mp3_data).expect("failed to parse mp3");
        assert_eq!(result, AudioType::MP3);
    }

    #[test]
    fn test_mp3_large_id3() {
        let mp3_data = include_bytes!("__fixtures__/mp3_id3v2_with_junk.bin");
        let result = detect_audio_type(mp3_data).expect("failed to parse mp3");
        assert_eq!(result, AudioType::MP3);
    }

    #[test]
    fn test_mp3_invalid_data() {
        let mut mp3_data = [0; 4096];
        mp3_data[0..4].copy_from_slice(&[0xff; 4]);
        let result = detect_audio_type(&mp3_data).expect("failed to parse mp3");
        assert_eq!(result, AudioType::Unknown);
    }

    #[test]
    fn test_mp3_invalid_2() {
        let mp3_data = include_bytes!("__fixtures__/junk.bin");
        let result = detect_audio_type(mp3_data).expect("failed to parse mp3");
        assert_eq!(result, AudioType::Unknown);
    }
}
