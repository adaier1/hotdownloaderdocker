use cipher::block_padding::UnpadError;
use cipher::inout::NotEqualError;
use thiserror::Error;

pub mod decrypt;
pub mod header;

#[derive(Error, Debug, Clone)]
pub enum JooxError {
    #[error("Header too small, require at least {0} bytes.")]
    HeaderTooSmall(usize),

    #[error("Output buffer require at least {0} bytes.")]
    OutputBufferTooSmall(usize),

    #[error("Input buffer require at least {0} bytes.")]
    InputBufferTooSmall(usize),

    #[error("Not Joox encrypted header: {0:2x?}")]
    NotJooxHeader([u8; 4]),

    #[error("Unsupported Joox version: {0:2x}")]
    UnsupportedJooxVersion(u8),
    #[error("AES Decryption Unpad Error: {0}")]
    AesUnpadError(UnpadError),
    #[error("AES Buffer setup error: {0}")]
    AesBufferSetupError(NotEqualError),
}
