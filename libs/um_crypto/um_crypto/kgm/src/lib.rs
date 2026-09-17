pub mod header;
mod pc_db_decrypt;
mod slot_keys;
pub mod v2;
pub mod v3;
mod v5;

pub use pc_db_decrypt::decrypt_db;

use crate::header::Header;
use crate::v2::DecipherV2;
use crate::v3::DecipherV3;
use thiserror::Error;

use crate::v5::DecipherV5;
use block_padding::UnpadError;

#[derive(Debug, Error)]
pub enum KugouError {
    #[error("Header too small, need at least {0} bytes.")]
    HeaderTooSmall(usize),

    #[error("Unsupported key slot: {0}")]
    UnsupportedKeySlot(i32),

    #[error("Unsupported cipher version: {0}")]
    UnsupportedCipherVersion(u32),

    #[error("V5 requires ekey.")]
    V5EKeyRequired,

    #[error("Not KGM File (magic mismatch)")]
    NotKGMFile,

    #[error("Unsupported cipher (self-test failed)")]
    SelfTestFailed,

    #[error("Failed decrypt kugou db data: {0}")]
    DecryptKugouDbError(UnpadError),

    #[error("Invalid database size: {0}")]
    InvalidDatabaseSize(usize),

    #[error("Failed to decrypt page 1 (invalid header)")]
    DecryptPage1Failed,

    #[error("Database does not seem valid")]
    InvalidPage1Header,

    #[error("QMC2EKeyError: {0}")]
    QMC2EKeyError(String),

    #[error("Parse KGM header with i/o error: {0}")]
    HeaderParseIOError(std::io::Error),

    #[error("Invalid audio hash size: {0}")]
    HeaderInvalidAudioHash(usize),
}

pub enum Decipher {
    V2(DecipherV2),
    V3(DecipherV3),
    V5(DecipherV5),
}

impl Decipher {
    pub fn new(header: &Header) -> Result<Self, KugouError> {
        Self::new_v5(header, None)
    }

    pub fn new_v5(header: &Header, ekey: Option<String>) -> Result<Self, KugouError> {
        let decipher = match header.crypto_version {
            2 => Decipher::V2(DecipherV2::new(header)?),
            3 => Decipher::V3(DecipherV3::new(header)?),
            5 => match ekey {
                Some(ekey) => Decipher::V5(DecipherV5::new(&ekey)?),
                _ => Err(KugouError::V5EKeyRequired)?,
            },
            version => Err(KugouError::UnsupportedCipherVersion(version))?,
        };

        let mut test_data = header.decrypt_test_data;
        decipher.decrypt(&mut test_data, 0);
        if test_data != header.get_challenge_data() {
            Err(KugouError::SelfTestFailed)?;
        }

        Ok(decipher)
    }

    pub fn decrypt<T: AsMut<[u8]> + ?Sized>(&self, buffer: &mut T, offset: usize) {
        match self {
            Decipher::V2(decipher) => decipher.decrypt(buffer, offset),
            Decipher::V3(decipher) => decipher.decrypt(buffer, offset),
            Decipher::V5(decipher) => decipher.decrypt(buffer, offset),
        }
    }
}
