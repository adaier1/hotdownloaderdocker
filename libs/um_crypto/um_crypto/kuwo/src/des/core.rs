use super::{constants, helper};
use crate::KuwoCryptoError;
use itertools::Either;

/// Encrypt or Decrypt?
pub enum Mode {
    Encrypt,
    Decrypt,
}

#[derive(Debug, Clone, Copy)]
pub struct KuwoDes {
    subkeys: [u64; 16],
}

fn des_subkey_expansion(key: &[u8; 8], mode: Mode) -> [u64; 16] {
    let key = u64::from_le_bytes(*key);

    let mut param = helper::map_u64(key, &constants::KEY_PERMUTATION_TABLE);

    let mut subkeys = [0u64; 16];
    let subkey_iter = match mode {
        Mode::Decrypt => Either::Left(subkeys.iter_mut().rev()),
        Mode::Encrypt => Either::Right(subkeys.iter_mut()),
    };

    for ((subkey, shl), shl_mask) in subkey_iter
        .zip(constants::KEY_RND_SHIFTS)
        .zip(constants::KEY_SHIFT_LEFT_MASKS)
    {
        param = ((param & shl_mask) << (28 - shl)) | ((param & !shl_mask) >> (shl));
        *subkey = helper::map_u64(param, &constants::KEY_COMPRESSION);
    }

    subkeys
}

fn des_subkey_round(state: u64, subkey: u64) -> u64 {
    let old_left = helper::u64_get_hi32(state);
    let old_right = helper::u64_get_lo32(state);

    // Key expansion
    let state = helper::map_u64(old_left as u64, &constants::KEY_EXPANSION);
    let state = state ^ subkey;

    // SBox transformation
    let right = constants::SBOXES
        .iter()
        .zip(state.to_be_bytes())
        .fold(0u32, |next, (sbox, b)| {
            (next << 4) | (sbox[b as usize] as u32)
        });

    let right = helper::map_u32(right, &constants::PBOX);
    let right = right ^ old_right;

    helper::make_u64(right, old_left)
}

impl KuwoDes {
    pub fn new(key: &[u8; 8], mode: Mode) -> Self {
        Self {
            subkeys: des_subkey_expansion(key, mode),
        }
    }

    pub fn transform_block(&self, data: u64) -> u64 {
        let mut state = helper::map_u64(data, &constants::IP);

        state = self
            .subkeys
            .iter()
            .fold(state, |state, &subkey| des_subkey_round(state, subkey));

        // Swap data hi32/lo32
        state = helper::swap_u64_side(state);

        // Final permutation
        state = helper::map_u64(state, &constants::IP_INV);

        state
    }

    pub fn transform(&self, data: &mut [u8]) -> Result<(), KuwoCryptoError> {
        if data.len() % 8 != 0 {
            Err(KuwoCryptoError::InvalidDesDataSize(data.len()))?
        }

        for block in data.chunks_exact_mut(8) {
            let value = u64::from_le_bytes(block.try_into().unwrap_or_default());
            let value = self.transform_block(value);
            block.copy_from_slice(&value.to_le_bytes());
        }
        Ok(())
    }
}

#[test]
fn test_des_decrypt() {
    let mut input = [
        0x36, 0x3C, 0x3E, 0x0D, 0x30, 0x31, 0xA4, 0x6C, 0xA0, 0xF0, 0x3A, 0xEC, 0x7F, 0x26, 0xF6,
        0xF4,
    ];

    let des = KuwoDes::new(b"ylzsxkwm", Mode::Decrypt);
    des.transform(&mut input).unwrap();
    assert_eq!(&input, b"12345678ABCDEFGH");
}
