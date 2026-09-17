use crate::footer::utils::is_base64;
use crate::footer::{Data, FooterParseError, Metadata, MetadataParser};
use byteorder::{ByteOrder, LE};

pub const MAX_ALLOWED_EKEY_LEN: usize = 0x500;

#[derive(Debug, Clone, PartialEq)]
pub struct PcV1Legacy;

impl MetadataParser for PcV1Legacy {
    fn from_byte_slice(buffer: &[u8]) -> Result<Option<Metadata>, FooterParseError> {
        if buffer.len() < 8 {
            Err(FooterParseError::BufferTooSmall(8))?;
        }

        let (payload, payload_len) = buffer.split_at(buffer.len() - 4);
        let payload_len = LE::read_u32(payload_len) as usize;

        // EKey payload is too large, probably not a valid V1 footer.
        if payload_len > MAX_ALLOWED_EKEY_LEN {
            Err(FooterParseError::PCv1EKeyTooLarge(payload_len))?;
        }
        if payload.len() < payload_len {
            Err(FooterParseError::BufferTooSmall(payload_len + 4))?;
        }

        let payload = &payload[payload.len() - payload_len..];
        let ekey = payload
            .iter()
            .take_while(|&&b| b != 0)
            .copied()
            .collect::<Vec<_>>();
        let ekey = String::from_utf8_lossy(ekey.as_slice());
        if !is_base64(ekey.as_bytes()) {
            Err(FooterParseError::PCv1EKeyInvalid)?;
        }

        Ok(Some(Metadata {
            ekey: Some(ekey.into()),
            size: payload_len + 4,
            data: Data::PCv1Legacy(PcV1Legacy),
        }))
    }
}
