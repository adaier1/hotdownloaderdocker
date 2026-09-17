pub mod android_qtag;
pub mod android_stag;
mod musicex_v1;
pub mod pc_v1_legacy;
pub mod pc_v2_musicex;
mod utils;

use crate::footer::{
    android_qtag::QTagMetadata, android_stag::STagMetadata, pc_v1_legacy::PcV1Legacy,
    pc_v2_musicex::PcV2MusicEx,
};
use thiserror::Error;

pub const INITIAL_DETECTION_LEN: usize = 1024;

#[derive(Error, Debug)]
pub enum FooterParseError {
    #[error("Footer: Buffer too small, require at least {0} bytes")]
    BufferTooSmall(usize),
    #[error("PCv1/EKey: Buffer too large, might not be valid EKey (len={0})")]
    PCv1EKeyTooLarge(usize),
    #[error("PCv1/EKey: Found invalid EKey char")]
    PCv1EKeyInvalid,

    #[error("PCv2/MusicEx: Invalid metadata version {0}")]
    PCv2InvalidVersion(u32),
    #[error("PCv2/MusicEx: Invalid `MusicEx` size: {0}")]
    PCv2MusicExUnsupportedPayloadSize(usize),

    #[error("Android/STag: Invalid ID field: {0}")]
    STagInvalidId(String),
    #[error("Android/STag: Invalid Version: {0}")]
    STagInvalidVersion(String),
    #[error("Android/STag: Invalid CSV metadata: {0}")]
    STagInvalidCSV(String),

    #[error("Android/QTag: Invalid ID field: {0}")]
    QTagInvalidId(String),
    #[error("Android/QTag: Invalid Version: {0}")]
    QTagInvalidVersion(String),
    #[error("Android/QTag: Invalid EKey field: {0}")]
    QTagInvalidEKey(String),

    #[error("Parse: Failed to parse string '{0}' as integer")]
    StringToIntError(String),

    #[error("Failed to parse MusicExV1: {0}")]
    MusicEx1ParseError(std::io::Error),
}

/// Footer type
#[derive(Debug, Clone, PartialEq)]
pub enum Data {
    /// No extra metadata.
    PCv1Legacy(pc_v1_legacy::PcV1Legacy),
    /// "MusicEx" footer.
    PCv2MusicEx(pc_v2_musicex::PcV2MusicEx),

    /// Android "QTag", with ekey.
    AndroidQTag(android_qtag::QTagMetadata),
    /// Android "STag", metadata only.
    AndroidSTag(android_stag::STagMetadata),
}

/// File Footer metadata
#[derive(Debug, Clone, PartialEq)]
pub struct Metadata {
    /// Footer size to trim off.
    pub size: usize,

    /// Embedded key (not decrypted).
    pub ekey: Option<String>,

    /// data/type
    pub data: Data,
}

pub trait MetadataParser {
    fn from_byte_slice(buffer: &[u8]) -> Result<Option<Metadata>, FooterParseError>;
}

pub fn from_byte_slice(buffer: &[u8]) -> Result<Option<Metadata>, FooterParseError> {
    if let Some(metadata) = STagMetadata::from_byte_slice(buffer)? {
        return Ok(Some(metadata));
    }
    if let Some(metadata) = QTagMetadata::from_byte_slice(buffer)? {
        return Ok(Some(metadata));
    }
    if let Some(metadata) = PcV2MusicEx::from_byte_slice(buffer)? {
        return Ok(Some(metadata));
    }
    if let Some(metadata) = PcV1Legacy::from_byte_slice(buffer)? {
        return Ok(Some(metadata));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::footer::android_qtag::QTagMetadata;
    use crate::footer::android_stag::STagMetadata;
    use crate::footer::pc_v1_legacy::PcV1Legacy;

    #[test]
    fn test_qtag() {
        let payload = include_bytes!("fixtures/ekey_android_qtag.bin");
        let payload = from_byte_slice(payload)
            .expect("Should not fail")
            .expect("should parse to qtag");

        assert_eq!(payload.ekey, Some("00112233aBcD+/=".into()));
        assert_eq!(payload.size, 0x23);
        assert_eq!(
            payload.data,
            Data::AndroidQTag(QTagMetadata {
                resource_id: 326454301
            })
        )
    }

    #[test]
    fn test_stag() {
        let payload = include_bytes!("fixtures/ekey_android_stag.bin");
        let payload = from_byte_slice(payload)
            .expect("Should not fail")
            .expect("should parse to stag");

        assert_eq!(payload.ekey, None);
        assert_eq!(payload.size, 0x20);
        assert_eq!(
            payload.data,
            Data::AndroidSTag(STagMetadata {
                media_mid: "001y7CaR29k6YP".into(),
                resource_id: 5177785,
            })
        )
    }

    #[test]
    fn test_pc_enc_v1() {
        let payload = include_bytes!("fixtures/ekey_pc_enc_v1.bin");
        let payload = from_byte_slice(payload)
            .expect("Should not fail")
            .expect("should parse pc v1");

        let ekey = payload.ekey.expect("ekey should be present");
        assert!(ekey.starts_with("NUZ6b0la"));

        assert_eq!(payload.size, 0x2C4);
        assert_eq!(payload.data, Data::PCv1Legacy(PcV1Legacy))
    }

    #[test]
    fn test_pc_enc_v2() {
        let payload = include_bytes!("fixtures/ekey_pc_enc_v2.bin");
        let payload = from_byte_slice(payload)
            .expect("Should not fail")
            .expect("should parse pc v2");

        assert_eq!(payload.ekey, None);
        assert_eq!(payload.size, 0xC0);
        assert_eq!(
            payload.data,
            Data::PCv2MusicEx(PcV2MusicEx {
                mid: "AaBbCcDdEeFfGg".into(),
                media_filename: "F0M000112233445566.mflac".into()
            })
        )
    }
}
