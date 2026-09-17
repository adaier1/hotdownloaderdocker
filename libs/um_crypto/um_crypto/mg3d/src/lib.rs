use thiserror::Error;
use umc_utils::md5_2;

mod guess_m4a;
mod guess_wav;

pub use guess_m4a::guess_key as guess_m4a_key;
pub use guess_wav::guess_key as guess_wav_key;

pub fn guess_key(buffer: &[u8]) -> Option<[u8; 0x20]> {
    guess_wav_key(buffer).or_else(|| guess_m4a_key(buffer))
}

fn raw_decrypt<T: AsMut<[u8]> + ?Sized>(buffer: &mut T, key: &[u8; 0x20], offset: usize) {
    for (b, i) in buffer.as_mut().iter_mut().zip(offset..) {
        *b = (*b).wrapping_sub(key[i % key.len()]);
    }
}

#[derive(Error, Debug)]
pub enum Migu3dError {
    #[error("Invalid FileKey")]
    InvalidFileKey,

    #[error("Convert hash to key error")]
    ConvertKeyError,
}

fn is_valid_password_chr(chr: u8) -> bool {
    matches!(chr, b'0'..=b'9' | b'A'..=b'F')
}

pub struct Decipher {
    key: [u8; 0x20],
}

impl Decipher {
    /// Init decipher from "file_key" (androidFileKey or iosFileKey)
    pub fn new_from_file_key(file_key: &str) -> Result<Self, Migu3dError> {
        let hash = md5_2(b"AC89EC47A70B76F307CB39A0D74BCCB0", file_key.as_bytes());
        let key = hex::encode_upper(hash);
        let key = key
            .as_bytes()
            .try_into()
            .map_err(|_| Migu3dError::ConvertKeyError)?;
        Ok(Self { key })
    }

    /// Init decipher from "key" (the final hash)
    pub fn new_from_final_key(key: &[u8; 0x20]) -> Result<Self, Migu3dError> {
        Ok(Self { key: *key })
    }

    pub fn decrypt<T: AsMut<[u8]> + ?Sized>(&self, buffer: &mut T, offset: usize) {
        raw_decrypt(buffer, &self.key, offset)
    }

    pub fn get_key(&self) -> [u8; 0x20] {
        self.key
    }
}
