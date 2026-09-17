use crate::footer::{Data, FooterParseError, Metadata, MetadataParser};
use byteorder::{ByteOrder, BE};
use itertools::Itertools;

#[derive(Debug, Clone, PartialEq)]
pub struct STagMetadata {
    /// Resource identifier (aka. `file.media_mid`).
    pub media_mid: String,

    /// Resource id (numeric)
    pub resource_id: u64,
}

impl MetadataParser for STagMetadata {
    fn from_byte_slice(buffer: &[u8]) -> Result<Option<Metadata>, FooterParseError> {
        if buffer.len() < 8 {
            Err(FooterParseError::BufferTooSmall(8))?;
        }

        if let Some(footer) = buffer.strip_suffix(b"STag") {
            let (payload, payload_len) = footer.split_at(footer.len() - 4);
            let actual_payload_len = BE::read_u32(payload_len) as usize;
            if payload.len() < actual_payload_len {
                Err(FooterParseError::BufferTooSmall(actual_payload_len + 8))?;
            }

            let payload = String::from_utf8_lossy(&payload[payload.len() - actual_payload_len..]);
            if let Some((id, version, media_mid)) = payload.split(',').collect_tuple() {
                if version != "2" {
                    Err(FooterParseError::STagInvalidVersion(version.to_string()))?;
                }
                if !id.as_bytes().iter().all(|&b| b.is_ascii_digit()) {
                    Err(FooterParseError::STagInvalidId(id.to_string()))?;
                }

                return Ok(Some(Metadata {
                    ekey: None,
                    size: actual_payload_len + 8,
                    data: Data::AndroidSTag(STagMetadata {
                        resource_id: id
                            .parse()
                            .map_err(|_| FooterParseError::StringToIntError(id.to_string()))?,
                        media_mid: media_mid.to_string(),
                    }),
                }));
            }

            Err(FooterParseError::STagInvalidCSV(payload.to_string()))?;
        }
        Ok(None)
    }
}
