use cipher::block_padding::UnpadError;
use cipher::InvalidLength;
use thiserror::Error;
use umc_utils::base64::DecodeError;

pub mod android;
pub mod pc;

#[derive(Error, Debug)]
pub enum XmlyError {
    #[error("Expected ID3 metadata")]
    MetadataMissing,

    #[error("ID3 Metadata too small (require {0} bytes)")]
    MetadataTooSmall(usize),

    #[error("Failed to extract encrypted audio segment size")]
    ParseHeaderSizeError,

    #[error("Failed to extract Stage 1 IV data")]
    ParseStage1IVError,

    #[error("Failed to extract Stage 2 decryption key")]
    ParseStage2KeyError,

    #[error("Failed to extract audio header")]
    ParseAudioHeaderError,

    #[error("Decryption stage 1 failed (padding)")]
    DecryptStage1Error(UnpadError),

    #[error("Decryption stage 1 failed (b64 decode)")]
    DecryptStage1B64Error(DecodeError),

    #[error("Decryption stage 2 failed (init)")]
    InitStage2Error(InvalidLength),

    #[error("Decryption stage 2 failed (padding)")]
    DecryptStage2Error(UnpadError),

    #[error("Decryption stage 2 failed (b64 decode)")]
    DecryptStage2B64Error(DecodeError),
}
