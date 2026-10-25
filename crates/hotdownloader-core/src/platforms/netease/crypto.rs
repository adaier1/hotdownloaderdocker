//! 网易云 WEAPI/EAPI 请求加密和 EAPI 响应解密。
use aes::{
    cipher::{generic_array::GenericArray, BlockDecrypt, BlockEncrypt, KeyInit},
    Aes128,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use md5::{Digest, Md5};
use num_bigint::BigUint;
use rand::Rng;
use serde_json::Value;

const EAPI_KEY: &[u8; 16] = b"e82ckenh8dichen8";

fn encrypt(input: &[u8], key: &[u8; 16], iv: Option<[u8; 16]>) -> Vec<u8> {
    let cipher = Aes128::new(GenericArray::from_slice(key));
    let mut bytes = input.to_vec();
    let padding = 16 - bytes.len() % 16;
    bytes.extend(vec![padding as u8; padding]);
    let mut previous = iv;
    for chunk in bytes.chunks_exact_mut(16) {
        if let Some(previous) = previous {
            for (byte, last) in chunk.iter_mut().zip(previous) {
                *byte ^= last;
            }
        }
        cipher.encrypt_block(GenericArray::from_mut_slice(chunk));
        if previous.is_some() {
            previous = Some(chunk.try_into().unwrap());
        }
    }
    bytes
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

pub(super) fn eapi(path: &str, data: &Value) -> String {
    let text = data.to_string();
    let digest = format!(
        "{:x}",
        Md5::digest(format!("nobody{path}use{text}md5forencrypt"))
    );
    hex(&encrypt(
        format!("{path}-36cd479b6b5-{text}-36cd479b6b5-{digest}").as_bytes(),
        EAPI_KEY,
        None,
    ))
}

pub(super) fn weapi(data: &Value) -> Vec<(&'static str, String)> {
    let alphabet = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let secret: [u8; 16] =
        std::array::from_fn(|_| alphabet[rand::rng().random_range(0..alphabet.len())]);
    let first = STANDARD.encode(encrypt(
        data.to_string().as_bytes(),
        b"0CoJUm6Qyw8W8jud",
        Some(*b"0102030405060708"),
    ));
    let params = STANDARD.encode(encrypt(
        first.as_bytes(),
        &secret,
        Some(*b"0102030405060708"),
    ));
    let modulus = BigUint::parse_bytes(b"e0b509f6259df8642dbc35662901477df22677ec152b5ff68ace615bb7b725152b3ab17a876aea8a5aa76d2e417629ec4ee341f56135fccf695280104e0312ecbda92557c93870114af6c9d05c4f7f0c3685b7a46bee255932575cce10b424d813cfe4875d3e82047b97ddef52741d546b8e289dc6935b3ece0462db0a22b8e7", 16).unwrap();
    let reversed: Vec<_> = secret.iter().rev().copied().collect();
    let encrypted = BigUint::from_bytes_be(&reversed).modpow(&BigUint::from(65537u32), &modulus);
    vec![
        ("params", params),
        ("encSecKey", format!("{encrypted:0256x}")),
    ]
}

pub(super) fn decode(bytes: &[u8]) -> Result<Value, String> {
    if let Ok(value) = serde_json::from_slice(bytes) {
        return Ok(value);
    }
    // Some EAPI endpoints return AES-ECB ciphertext, others return plain JSON.
    if bytes.is_empty() || bytes.len() % 16 != 0 {
        return Err("网易云响应格式无效".into());
    }
    let mut plain = bytes.to_vec();
    let cipher = Aes128::new(GenericArray::from_slice(EAPI_KEY));
    for chunk in plain.chunks_exact_mut(16) {
        cipher.decrypt_block(GenericArray::from_mut_slice(chunk));
    }
    let padding = *plain.last().unwrap() as usize;
    if padding == 0
        || padding > 16
        || !plain[plain.len() - padding..]
            .iter()
            .all(|b| *b as usize == padding)
    {
        return Err("网易云响应解密失败".into());
    }
    plain.truncate(plain.len() - padding);
    serde_json::from_slice(&plain).map_err(|_| "网易云响应不是有效 JSON".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn aes_matches_nist_and_decodes_both_response_formats() {
        let key = [0u8; 16];
        assert_eq!(
            &hex(&encrypt(&[0u8; 16], &key, None))[..32],
            "66E94BD4EF8A2C3B884CFA59CA342B2E"
        );
        let value = json!({
            "code": 200,
            "text": "网易云"
        });
        assert_eq!(
            decode(&encrypt(value.to_string().as_bytes(), EAPI_KEY, None)).unwrap(),
            value
        );
        assert_eq!(decode(value.to_string().as_bytes()).unwrap(), value);
        assert!(decode(b"invalid").is_err());
    }
}
