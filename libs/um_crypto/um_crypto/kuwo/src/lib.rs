use crate::kwm_v1::CipherV1;
use byteorder::{ReadBytesExt, LE};
use std::fmt;
use std::io::{Cursor, Read};
use thiserror::Error;
use umc_utils::base64::DecodeError;

pub mod des;

pub mod kwm_v1;
pub use umc_qmc::{QMCv2Cipher as CipherV2, QmcCryptoError};

/// Commonly used secret key for Kuwo services.
pub const SECRET_KEY: [u8; 8] = *b"ylzsxkwm";

#[derive(Debug)]
pub struct HeaderMagicBytes(pub [u8; 16]);

#[derive(Error, Debug)]
pub enum KuwoCryptoError {
    #[error("Invalid DES data size (expected: {0} mod 8 == 0)")]
    InvalidDesDataSize(usize),

    #[error("Invalid KWM header magic bytes: {0}")]
    InvalidHeaderMagic(HeaderMagicBytes),

    #[error("KWMv2: EKey required")]
    V2EKeyRequired,

    #[error("KWM: Unsupported version {0}")]
    UnsupportedVersion(usize),

    #[error("Failed to decode base64: {0}")]
    Base64DecodeError(#[from] DecodeError),

    #[error("{0}")]
    QMCCipherError(#[from] QmcCryptoError),

    #[error("Failed to read data: {0}")]
    IoError(#[from] std::io::Error),
}

impl fmt::Display for HeaderMagicBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0.iter() {
            write!(f, "{:02x} ", byte)?;
        }
        Ok(())
    }
}

pub const DATA_START_OFFSET: usize = 0x400;

pub enum Decipher {
    V1(CipherV1),
    V2(CipherV2),
}

impl Decipher {
    pub fn new<T: AsRef<[u8]>>(
        header: &Header,
        ekey: Option<T>,
    ) -> Result<Decipher, KuwoCryptoError> {
        let cipher = match header.version {
            1 => Decipher::V1(CipherV1::new(header.resource_id)),
            2 => match ekey {
                Some(ekey) => Decipher::V2(CipherV2::new_from_ekey(ekey)?),
                None => Err(KuwoCryptoError::V2EKeyRequired)?,
            },
            version => Err(KuwoCryptoError::UnsupportedVersion(version as usize))?,
        };

        Ok(cipher)
    }

    pub fn decrypt<T>(&self, data: &mut T, offset: usize)
    where
        T: AsMut<[u8]> + ?Sized,
    {
        match self {
            Decipher::V1(cipher) => cipher.decrypt(data, offset),
            Decipher::V2(cipher) => cipher.decrypt(data, offset),
        }
    }
}

pub struct Header {
    pub magic: [u8; 0x10],

    /// 1: LegacyKWM
    /// 2: TME/QMCv2
    pub version: u32,
    pub unknown_1: u32,
    pub resource_id: u32,
    pub unknown_2: [u8; 0x14],
    pub format_name: [u8; 0x0C],
}

impl Header {
    const MAGIC_1: [u8; 16] = *b"yeelion-kuwo-tme";
    const MAGIC_2: [u8; 16] = *b"yeelion-kuwo\0\0\0\0";

    pub fn from_bytes<T>(bytes: T) -> Result<Self, KuwoCryptoError>
    where
        T: AsRef<[u8]>,
    {
        let mut cursor = Cursor::new(bytes);
        let mut magic = [0u8; 0x10];
        cursor.read_exact(&mut magic)?;
        let version = cursor.read_u32::<LE>()?;
        let unknown_1 = cursor.read_u32::<LE>()?;
        let resource_id = cursor.read_u32::<LE>()?;
        let mut unknown_2 = [0u8; 0x14];
        cursor.read_exact(&mut unknown_2)?;
        let mut format_name = [0u8; 0x0C];
        cursor.read_exact(&mut format_name)?;

        if magic != Self::MAGIC_1 && magic != Self::MAGIC_2 {
            Err(KuwoCryptoError::InvalidHeaderMagic(HeaderMagicBytes(magic)))?;
        }

        Ok(Self {
            magic,
            version,
            unknown_1,
            resource_id,
            unknown_2,
            format_name,
        })
    }

    /// Get the quality id
    /// Used for matching Android MMKV id.
    pub fn get_quality_id(&self) -> u32 {
        self.format_name
            .iter()
            .take_while(|&&c| c != 0 && c.is_ascii_digit())
            .fold(0, |sum, &value| sum * 10 + u32::from(value - b'0'))
    }
}

pub struct CipherBoDian(CipherV2);

impl CipherBoDian {
    pub fn new<T: AsRef<[u8]>>(ekey: T) -> Result<Self, KuwoCryptoError> {
        let ekey = des::decode_ekey(ekey, &SECRET_KEY)?;
        let cipher = CipherV2::new_from_ekey(ekey.as_str())?;
        Ok(Self(cipher))
    }

    #[inline]
    pub fn decrypt<T>(&self, data: &mut T, offset: usize)
    where
        T: AsMut<[u8]> + ?Sized,
    {
        self.0.decrypt(data.as_mut(), offset)
    }
}
