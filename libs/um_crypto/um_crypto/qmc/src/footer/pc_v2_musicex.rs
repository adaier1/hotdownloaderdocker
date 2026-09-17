use crate::footer::{musicex_v1, FooterParseError, Metadata, MetadataParser};
use byteorder::{ByteOrder, LE};

#[derive(Debug, Clone, PartialEq)]
pub struct PcV2MusicEx {
    /// Resource identifier (`.mid`)
    pub mid: String,

    /// The actual file name used for `ekey` lookup (`.file.media_mid` + extension).
    pub media_filename: String,
}

impl MetadataParser for PcV2MusicEx {
    fn from_byte_slice(payload: &[u8]) -> Result<Option<Metadata>, FooterParseError> {
        if payload.len() < 16 {
            Err(FooterParseError::BufferTooSmall(16))?;
        }

        if let Some(payload) = payload.strip_suffix(b"musicex\x00") {
            let (payload, version) = payload.split_at(payload.len() - 4);
            let version = LE::read_u32(version);

            return match version {
                1 => musicex_v1::parse_v1(payload),
                _ => Err(FooterParseError::PCv2InvalidVersion(version))?,
            };
        }

        Ok(None)
    }
}
