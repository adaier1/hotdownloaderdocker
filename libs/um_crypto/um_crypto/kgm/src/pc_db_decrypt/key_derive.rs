use byteorder::{ByteOrder, LE};
use umc_utils::md5;

use aes::cipher::{
    block_padding::NoPadding, generic_array::GenericArray, BlockDecryptMut, KeyIvInit,
};

use crate::KugouError;

type Aes128CbcDec = cbc::Decryptor<aes::Aes128Dec>;

const DEFAULT_MASTER_KEY: [u8; 0x18] = [
    // master key (0x10 bytes)
    0x1D, 0x61, 0x31, 0x45, 0xB2, 0x47, 0xBF, 0x7F, 0x3D, 0x18, 0x96, 0x72, 0x14, 0x4F, 0xE4, 0xBF,
    0x00, 0x00, 0x00, 0x00, // page number (le)
    0x73, 0x41, 0x6C, 0x54, // fixed value
];

fn next_page_iv(seed: u32) -> u32 {
    let left = seed.wrapping_mul(0x9EF4);
    let right = seed.wrapping_div(0xce26).wrapping_mul(0x7FFFFF07);
    let value = left.wrapping_sub(right);
    match value & 0x8000_0000 {
        0 => value,
        _ => value.wrapping_add(0x7FFF_FF07),
    }
}

fn derive_page_aes_key(seed: u32) -> [u8; 0x10] {
    let mut master_key = DEFAULT_MASTER_KEY;
    LE::write_u32(&mut master_key[0x10..0x14], seed);
    md5(master_key)
}

fn derive_page_aes_iv(seed: u32) -> [u8; 0x10] {
    let mut buffer = [0u8; 0x10];
    let mut iv = seed + 1;
    for i in (0..0x10).step_by(4) {
        iv = next_page_iv(iv);
        LE::write_u32(&mut buffer[i..i + 4], iv);
    }
    md5(buffer)
}

/// Page number starts from 1.
/// Buffer should have size of ().
pub fn decrypt_db_page(buffer: &mut [u8], page_number: u32) -> Result<(), KugouError> {
    let key = derive_page_aes_key(page_number);
    let iv = derive_page_aes_iv(page_number);

    let key = GenericArray::from(key);
    let iv = GenericArray::from(iv);
    let dec = Aes128CbcDec::new(&key, &iv);
    dec.decrypt_padded_mut::<NoPadding>(buffer)
        .map_err(KugouError::DecryptKugouDbError)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_derive(page_no: u32, expected_key: [u8; 0x10], expected_iv: [u8; 0x10]) {
        let aes_key = derive_page_aes_key(page_no);
        assert_eq!(aes_key, expected_key, "key mismatch for page {}", page_no);

        let aes_iv = derive_page_aes_iv(page_no);
        assert_eq!(aes_iv, expected_iv, "iv mismatch for page {}", page_no);
    }

    #[test]
    fn test_derive_page_0_iv() {
        test_derive(
            0,
            [
                0x19, 0x62, 0xc0, 0x5f, 0xa2, 0xeb, 0xbe, 0x24, 0x28, 0xff, 0x52, 0x2b, 0x9e, 0x03,
                0xea, 0xd4,
            ],
            [
                0x05, 0x5a, 0x67, 0x35, 0x93, 0x89, 0x2d, 0xdf, 0x3a, 0xb3, 0xb3, 0xc6, 0x21, 0xc3,
                0x48, 0x02,
            ],
        );
    }

    #[test]
    fn test_derive_page_12345_iv() {
        test_derive(
            12345,
            [
                0xc1, 0x70, 0x06, 0x4e, 0xf8, 0x1e, 0x15, 0x35, 0xc2, 0x9a, 0x65, 0xe4, 0xb6, 0xf5,
                0x78, 0xe9,
            ],
            [
                0xd0, 0xcd, 0x91, 0xd0, 0x23, 0xc5, 0x1e, 0x21, 0xbc, 0x01, 0xaa, 0xd2, 0x81, 0x4c,
                0x9b, 0xb8,
            ],
        );
    }
    #[test]
    fn test_derive_page_498651347_iv() {
        test_derive(
            498651347,
            [
                0x5a, 0x69, 0xb3, 0xdc, 0x58, 0xca, 0x16, 0x2e, 0xb4, 0xa7, 0x71, 0x4e, 0xf2, 0x73,
                0x6b, 0xf7,
            ],
            [
                0x62, 0xa7, 0x22, 0x26, 0x64, 0x08, 0x89, 0xb8, 0xff, 0x5d, 0xdc, 0x31, 0x7e, 0x7c,
                0x7e, 0xcc,
            ],
        );
    }
}
