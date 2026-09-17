use crate::footer::utils::is_base64;
use crate::footer::{Data, FooterParseError, Metadata, MetadataParser};
use byteorder::{ByteOrder, BE};
use itertools::Itertools;

#[derive(Debug, Clone, PartialEq)]
pub struct QTagMetadata {
    /// The old, numeric id of the resource.
    pub resource_id: u64,
}

impl MetadataParser for QTagMetadata {
    fn from_byte_slice(buffer: &[u8]) -> Result<Option<Metadata>, FooterParseError> {
        if buffer.len() < 8 {
            Err(FooterParseError::BufferTooSmall(8))?;
        }

        if let Some(footer) = buffer.strip_suffix(b"QTag") {
            let (payload, payload_len) = footer.split_at(footer.len() - 4);
            let actual_payload_len = BE::read_u32(payload_len) as usize;
            if payload.len() < actual_payload_len {
                Err(FooterParseError::BufferTooSmall(actual_payload_len + 8))?;
            }

            // CSV: ekey,resource_id,version
            let payload = String::from_utf8_lossy(&payload[payload.len() - actual_payload_len..]);
            if let Some((ekey, resource_id, version)) = payload.split(',').collect_tuple() {
                if version != "2" {
                    Err(FooterParseError::QTagInvalidVersion(version.to_string()))?;
                }
                if !resource_id.as_bytes().iter().all(|&b| b.is_ascii_digit()) {
                    Err(FooterParseError::QTagInvalidId(resource_id.to_string()))?;
                }
                if !is_base64(ekey.as_bytes()) {
                    Err(FooterParseError::QTagInvalidEKey(ekey.to_string()))?;
                }

                return Ok(Some(Metadata {
                    ekey: Some(ekey.into()),
                    size: actual_payload_len + 8,
                    data: Data::AndroidQTag(QTagMetadata {
                        resource_id: resource_id.parse().map_err(|_| {
                            FooterParseError::StringToIntError(resource_id.to_string())
                        })?,
                    }),
                }));
            }

            Err(FooterParseError::STagInvalidCSV(payload.to_string()))?;
        }
        Ok(None)
    }
}
