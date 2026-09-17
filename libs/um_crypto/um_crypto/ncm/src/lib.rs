pub mod content_key;
pub mod header;
pub mod metadata;

use cipher::block_padding::UnpadError;
use thiserror::Error;
use umc_utils::base64;

#[derive(Error, Debug)]
pub enum NetEaseCryptoError {
    #[error("Header need at least {0} more bytes")]
    HeaderTooSmall(usize),

    #[error("File I/O Error: {0}")]
    FileIOError(std::io::Error),
    #[error("File I/O Error: Read 0 bytes")]
    FileIOErrorReadZero,

    #[error("Not a NCM file")]
    NotNCMFile,
    #[error("Invalid NCM checksum. Expected {expected:08x}, actual: {expected:08x}")]
    ChecksumInvalid { expected: u32, actual: u32 },
    #[error("Unsupported cover image version: {0}")]
    UnsupportedCoverImageVersion(u8),
    #[error("Cover image: Frame size is less than image 1. frame_size:{frame_size}, image1_size:{image1_size}")]
    InvalidCoverImage2Size {
        frame_size: usize,
        image1_size: usize,
    },

    #[error("ContentKey: AES PKCS#7 Decode Error")]
    ContentKeyDecryptError(UnpadError),
    #[error("ContentKey: Invalid key prefix: {0}")]
    ContentKeyWrongPrefix(String),

    #[error("Metadata: Invalid prefix while decoding: {0}")]
    MetadataWrongPrefix(String),
    #[error("Metadata: AES PKCS#7 Decode Error")]
    MetadataDecryptError(UnpadError),
    #[error("Metadata: Decode metadata failed: {0}")]
    MetadataDecodeError(base64::DecodeError),
    #[error("Metadata: Invalid prefix on final json: {0}")]
    MetadataInvalidJsonPrefix(String),
}
