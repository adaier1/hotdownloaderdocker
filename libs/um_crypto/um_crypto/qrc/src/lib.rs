use hex::FromHexError;
use miniz_oxide::inflate::{decompress_to_vec_zlib_with_limit as inflate, DecompressError};
use thiserror::Error;

mod des;
use crate::des::DESMode;
use des::QrcDes;

#[derive(Error, Debug)]
pub enum QrcError {
    #[error("QRCDes: input is not block of 8 bytes")]
    QRCDesInputSizeError,

    #[error("QRC: Failed to inflate: {0}")]
    QRCInflateError(DecompressError),

    #[error("QRC: Failed to decode hex: {0}")]
    QRCHexDecodeError(FromHexError),

    #[error("QRC: Invalid file magic header")]
    QRCInvalidMagicHeader,
}

// Max 4MiB for QRC
const MAX_QRC_SIZE: usize = 4 * 1024 * 1024;

const DES_KEY_1: &[u8; 8] = b"!@#)(NHL";
const DES_KEY_2: &[u8; 8] = b"123ZXC!@";
const DES_KEY_3: &[u8; 8] = b"!@#)(*$%";

pub fn decrypt_qrc(data: &[u8]) -> Result<Vec<u8>, QrcError> {
    let mut temp = data.to_vec();
    QrcDes::new(DES_KEY_1, DESMode::Decrypt).transform_bytes(&mut temp)?;
    QrcDes::new(DES_KEY_2, DESMode::Encrypt).transform_bytes(&mut temp)?;
    QrcDes::new(DES_KEY_3, DESMode::Decrypt).transform_bytes(&mut temp)?;
    let result = inflate(&temp[..], MAX_QRC_SIZE).map_err(QrcError::QRCInflateError)?;
    Ok(result)
}

/// Decrypt QRC data from API response
pub fn decrypt_qrc_network(data: &str) -> Result<Vec<u8>, QrcError> {
    let data = hex::decode(data).map_err(QrcError::QRCHexDecodeError)?;
    decrypt_qrc(&data[..])
}

const QRC_MAGIC: [u8; 11] = [
    0x98, 0x25, 0xB0, 0xAC, 0xE3, 0x02, 0x83, 0x68, 0xE8, 0xFC, 0x6C,
];

/// Decrypt QRC data from cached local file
pub fn decrypt_qrc_file(data: &[u8]) -> Result<Vec<u8>, QrcError> {
    let data = match data.strip_prefix(&QRC_MAGIC) {
        None => Err(QrcError::QRCInvalidMagicHeader)?,
        Some(data) => data,
    };
    let mut temp = data.to_vec();
    umc_qmc::v1::decrypt(&mut temp, QRC_MAGIC.len());
    decrypt_qrc(&temp[..])
}

#[cfg(test)]
mod tests {
    use crate::{decrypt_qrc_file, decrypt_qrc_network};

    #[test]
    fn test_qrc_file() {
        let data = [
            0x98, 0x25, 0xB0, 0xAC, 0xE3, 0x02, 0x83, 0x68, 0xE8, 0xFC, 0x6C, 0xAB, 0x9A, 0x34,
            0xE2, 0x31, 0x26, 0xAF, 0x6E, 0x2A, 0x23, 0xB3, 0x56, 0xC3, 0xBF, 0x8A, 0xA6,
        ];

        let result = decrypt_qrc_file(&data).expect("Decryption failed.");
        assert_eq!(result, b"nothing");
    }

    #[test]
    fn test_qrc_file_2() {
        let data = [
            0x98, 0x25, 0xB0, 0xAC, 0xE3, 0x02, 0x83, 0x68, 0xE8, 0xFC, 0x6C, 0x07, 0xBC, 0x8C,
            0x46, 0x97, 0x36, 0xDF, 0x06, 0x13, 0xE0, 0x31, 0xD1, 0xF8, 0x98, 0xEF, 0xD0, 0x1B,
            0xEA, 0x6B, 0x04, 0x1D, 0xDB, 0xE0, 0x0F, 0x33, 0x2B, 0xBE, 0x95, 0x27, 0xB9, 0xF6,
            0xEE, 0x0C, 0x75, 0x0C, 0x46, 0x4C, 0xA8, 0xE8, 0x37, 0x93, 0x03, 0xC0, 0xA6, 0x98,
            0xD0, 0x4B, 0x6E, 0xBB, 0x2A, 0x8C, 0x3E, 0xE8, 0x7F, 0xC2, 0x0F, 0x6E, 0x2E, 0x3E,
            0xAD, 0x38, 0xCF, 0x74, 0x01, 0x17, 0xDA, 0xE0, 0x62, 0x45, 0x4F, 0xF8, 0x35,
        ];

        let result = decrypt_qrc_file(&data).expect("Decryption failed.");
        assert_eq!(
            String::from_utf8_lossy(&result),
            "[00:00:00]此歌曲为没有填词的纯音乐，请您欣赏"
        );
    }

    #[test]
    fn test_qrc_from_network() {
        // Original QRC
        let input_data = include_str!("__fixture__/qrc_network_1_jp.txt");
        let qrc_original_b = decrypt_qrc_network(input_data).expect("decrypt failed");
        let qrc_original = String::from_utf8(qrc_original_b).expect("decode failed");
        assert!(qrc_original.contains("太(1399,388)陽(1787,486)系(2273,1433)を(3706,404)"));

        // QRC 罗马字
        let input_data = include_str!("__fixture__/qrc_network_1_roma.txt");
        let qrc_romaji_b = decrypt_qrc_network(input_data).expect("decrypt failed");
        let qrc_romaji = String::from_utf8(qrc_romaji_b).expect("decode failed");
        assert!(qrc_romaji.contains("ta (1399,194)i (1593,194)yo (1787,243)u (2029,243)ke"));

        // QRC 翻译 (LRC)
        let input_data = include_str!("__fixture__/qrc_network_1_trans.txt");
        let qrc_translation_b = decrypt_qrc_network(input_data).expect("decrypt failed");
        let qrc_translation = String::from_utf8(qrc_translation_b).expect("decode failed");
        assert!(qrc_translation.contains("[00:01.39]摆脱太阳系"));

        println!("qrc_original: {}", qrc_original);
        println!("qrc_romaji: {}", qrc_romaji);
        println!("qrc_translation: {}", qrc_translation);
    }
}
