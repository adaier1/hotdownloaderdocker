use aes::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};
use thiserror::Error;
use umc_utils::base64::DecodeError;

pub mod nonce;
pub mod secret;

#[derive(Error, Debug, Clone)]
pub enum QingTingFMError {
    #[error("Failed to decode file name.")]
    DecodeFileNameFailed(DecodeError),

    #[error("File name does not start with known prefix.")]
    MissingPrefix,
}

type Aes128Ctr64BE = ctr::Ctr64BE<aes::Aes128>;
pub struct Decipher(Aes128Ctr64BE);

impl Decipher {
    pub fn new(device_key: &[u8; 0x10], iv: &[u8; 0x10]) -> Self {
        Decipher(Aes128Ctr64BE::new(device_key.into(), iv.into()))
    }

    pub fn decrypt(&self, buffer: &mut [u8], offset: usize) {
        let mut aes_engine = self.0.clone();
        aes_engine.seek(offset);
        aes_engine.apply_keystream(&mut buffer[..]);
    }
}
