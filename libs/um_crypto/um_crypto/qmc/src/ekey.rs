use itertools::Itertools;
use lazy_static::lazy_static;
use std::ops::Mul;
use tc_tea::TcTeaError;
use thiserror::Error;
use umc_utils::base64;

/// Base64 encoded prefix: "QQMusic EncV2,Key:"
const EKEY_V2_PREFIX: &[u8; 24] = b"UVFNdXNpYyBFbmNWMixLZXk6";
const EKEY_V2_KEY1: [u8; 16] = [
    0x33, 0x38, 0x36, 0x5A, 0x4A, 0x59, 0x21, 0x40, 0x23, 0x2A, 0x24, 0x25, 0x5E, 0x26, 0x29, 0x28,
];
const EKEY_V2_KEY2: [u8; 16] = [
    0x2A, 0x2A, 0x23, 0x21, 0x28, 0x23, 0x24, 0x25, 0x26, 0x5E, 0x61, 0x31, 0x63, 0x5A, 0x2C, 0x54,
];

lazy_static! {
    static ref EKEY_SIMPLE_KEY: [u8; 8] = make_simple_key::<8>();
}

#[derive(Debug, PartialEq, Error)]
pub enum EKeyDecryptError {
    #[error("EKey is too short for decryption")]
    EKeyTooShort,
    #[error("Error when decrypting ekey v1: {0}")]
    FailDecryptV1(TcTeaError),
    #[error("Error when decrypting ekey v2: {0}")]
    FailDecryptV2(TcTeaError),
    #[error("Failed to decode b64 content: {0}")]
    Base64Decode(#[from] base64::DecodeError),
}

fn make_simple_key<const N: usize>() -> [u8; N] {
    let mut result = [0u8; N];

    for (i, v) in result.iter_mut().enumerate() {
        let i = i as f32;
        let value = 106.0 + i * 0.1;
        let value = value.tan().abs().mul(100.0);
        *v = value as u8;
    }

    result
}

pub fn decrypt_v1(ekey: &[u8]) -> Result<Vec<u8>, EKeyDecryptError> {
    if ekey.len() < 12 {
        Err(EKeyDecryptError::EKeyTooShort)?;
    }

    let ekey = base64::decode(ekey)?;
    let (header, cipher) = ekey.split_at(8);

    // tea_key: interleave a byte from each stream
    let tea_key = EKEY_SIMPLE_KEY
        .iter()
        .zip(header)
        .flat_map(|(&simple_key_part, &header_part)| [simple_key_part, header_part])
        .collect_vec();

    let plaintext = tc_tea::decrypt(cipher, tea_key).map_err(EKeyDecryptError::FailDecryptV1)?;
    Ok([header, &plaintext].concat())
}

pub fn decrypt_v2(ekey: &[u8]) -> Result<Vec<u8>, EKeyDecryptError> {
    let ekey = base64::decode(ekey)?;
    let ekey = tc_tea::decrypt(ekey, EKEY_V2_KEY1).map_err(EKeyDecryptError::FailDecryptV2)?;
    let ekey = tc_tea::decrypt(ekey, EKEY_V2_KEY2).map_err(EKeyDecryptError::FailDecryptV2)?;
    let ekey = ekey.iter().take_while(|&&b| b != 0).copied().collect_vec();

    decrypt_v1(&ekey)
}

pub fn decrypt<T: AsRef<[u8]>>(ekey: T) -> Result<Vec<u8>, EKeyDecryptError> {
    let ekey = ekey.as_ref();
    match ekey.strip_prefix(EKEY_V2_PREFIX) {
        Some(v2_ekey) => decrypt_v2(v2_ekey),
        None => decrypt_v1(ekey),
    }
}
