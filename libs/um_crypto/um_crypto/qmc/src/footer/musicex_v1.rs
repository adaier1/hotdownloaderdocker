use crate::footer::pc_v2_musicex::PcV2MusicEx;
use crate::footer::utils::from_ascii_utf16;
use crate::footer::{Data, FooterParseError, Metadata};
use byteorder::{ByteOrder, ReadBytesExt, LE};
use std::io::{Cursor, Read};

#[derive(Debug, Clone, PartialEq)]
pub struct MusicExV1 {
    /// unused & unknown
    unknown_0: u32,
    /// unused & unknown
    unknown_1: u32,
    /// unused & unknown
    unknown_2: u32,

    /// Media ID
    mid: [u8; 30 * 2],
    /// Media file name
    media_filename: [u8; 50 * 2],

    /// unused; uninitialized memory?
    unknown_3: u32,
}

impl Default for MusicExV1 {
    fn default() -> Self {
        MusicExV1 {
            unknown_0: 0,
            unknown_1: 0,
            unknown_2: 0,
            mid: [0; 30 * 2],
            media_filename: [0; 50 * 2],
            unknown_3: 0,
        }
    }
}

impl MusicExV1 {
    #[allow(clippy::field_reassign_with_default)]
    fn from_bytes_inner(buffer: &[u8]) -> Result<MusicExV1, std::io::Error> {
        assert_eq!(buffer.len(), 0xC0 - 0x10);

        let mut cursor = Cursor::new(&buffer);
        let mut result = MusicExV1::default();
        result.unknown_0 = cursor.read_u32::<LE>()?;
        result.unknown_1 = cursor.read_u32::<LE>()?;
        result.unknown_2 = cursor.read_u32::<LE>()?;
        cursor.read_exact(&mut result.mid)?;
        cursor.read_exact(&mut result.media_filename)?;
        result.unknown_3 = cursor.read_u32::<LE>()?;

        Ok(result)
    }

    pub fn from_bytes(buffer: &[u8]) -> Result<MusicExV1, FooterParseError> {
        Self::from_bytes_inner(buffer).map_err(FooterParseError::MusicEx1ParseError)
    }
}

pub fn parse_v1(footer: &[u8]) -> Result<Option<Metadata>, FooterParseError> {
    let (payload, payload_len) = footer.split_at(footer.len() - 4);
    let payload_len = LE::read_u32(payload_len) as usize;
    if payload_len != 0xC0 {
        Err(FooterParseError::PCv2MusicExUnsupportedPayloadSize(
            payload_len,
        ))?;
    }

    let payload = &payload[payload.len() - (payload_len - 0x10)..];
    let payload = MusicExV1::from_bytes(payload)?;
    let mid = from_ascii_utf16(&payload.mid);
    let media_filename = from_ascii_utf16(&payload.media_filename);

    Ok(Some(Metadata {
        ekey: None,
        size: payload_len,
        data: Data::PCv2MusicEx(PcV2MusicEx {
            mid,
            media_filename,
        }),
    }))
}
