use super::constants;
use super::utils::{hi32, lo32, make_u64, map_u32_bits, map_u64, swap_u64};
use crate::QrcError;
use byteorder::{ByteOrder, LE};
use itertools::Either;

type DesSubKeys = [u64; 16];

pub enum DESMode {
    Encrypt,
    Decrypt,
}

/// QRC's modified DES implementation
#[derive(Debug, Default, Clone, Copy)]
pub struct QrcDes {
    keys: DesSubKeys,
}

impl QrcDes {
    fn ip(data: u64) -> u64 {
        map_u64(data, &constants::IP)
    }

    fn ip_inv(data: u64) -> u64 {
        map_u64(data, &constants::IP_INV)
    }

    const SBOX_SHIFTS: [u8; 8] = [26, 20, 14, 8, 58, 52, 46, 40];
    fn sbox_transform(state: u64) -> u32 {
        let stream = constants::SBOXES.iter().zip(Self::SBOX_SHIFTS);

        stream.fold(0u32, |result, (sbox, large_state_shift)| {
            let sbox_idx = (state >> large_state_shift) & 0b111111;
            (result << 4) | (sbox[sbox_idx as usize] as u32)
        })
    }

    fn des_crypt_proc(state: u64, key: u64) -> u64 {
        let mut state = state;
        let state_hi32 = hi32(state);
        let state_lo32 = lo32(state);

        state = map_u64(make_u64(state_hi32, state_hi32), &constants::KEY_EXPANSION);
        state ^= key;

        let mut next_lo32 = Self::sbox_transform(state);
        next_lo32 = map_u32_bits(next_lo32, &constants::PBOX);
        next_lo32 ^= state_lo32;
        make_u64(next_lo32, state_hi32)
    }

    /// Create a new QrcDes Instance
    pub fn new(key: &[u8; 8], mode: DESMode) -> Self {
        Self {
            keys: Self::derive_subkeys(key, mode),
        }
    }

    fn derive_subkeys(key: &[u8; 8], mode: DESMode) -> DesSubKeys {
        let key = u64::from_le_bytes(*key);

        let param = map_u64(key, &constants::KEY_PERMUTATION_TABLE);
        let mut param_c = lo32(param);
        let mut param_d = hi32(param);

        let update_param = |param: &mut u32, shift_left: u8| {
            let shift_right = 28 - shift_left;
            *param = (*param << shift_left) | ((*param >> shift_right) & 0xFFFFFFF0);
        };

        let mut subkeys = DesSubKeys::default();

        let key_iter = match mode {
            DESMode::Decrypt => Either::Left(subkeys.iter_mut().rev()),
            DESMode::Encrypt => Either::Right(subkeys.iter_mut()),
        };

        for (subkey, shift_left) in key_iter.zip(constants::KEY_RND_SHIFTS) {
            update_param(&mut param_c, shift_left);
            update_param(&mut param_d, shift_left);

            let key = make_u64(param_d, param_c);
            *subkey = map_u64(key, &constants::KEY_COMPRESSION);
        }

        subkeys
    }

    pub fn transform_block(&self, data: u64) -> u64 {
        let mut state = Self::ip(data);

        let keys = self.keys.iter();
        state = keys.fold(state, |state, &key| Self::des_crypt_proc(state, key));

        // Swap data hi32/lo32
        state = swap_u64(state);

        // Final permutation
        state = Self::ip_inv(state);

        state
    }

    pub fn transform_bytes(&self, data: &mut [u8]) -> Result<(), QrcError> {
        if data.len() % 8 != 0 {
            Err(QrcError::QRCDesInputSizeError)?;
        }

        for block in data.chunks_exact_mut(8) {
            let value = LE::read_u64(block);
            let transformed = self.transform_block(value);
            LE::write_u64(block, transformed);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{DESMode, QrcDes};

    #[test]
    fn test_des_decrypt() {
        let mut input = [1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6];
        let expected_data = [
            0xFD, 0x0E, 0x64, 0x06, 0x65, 0xBE, 0x74, 0x13, //
            0x77, 0x63, 0x3B, 0x02, 0x45, 0x4E, 0x70, 0x7A, //
        ];

        let des = QrcDes::new(b"TEST!KEY", DESMode::Decrypt);
        des.transform_bytes(&mut input).unwrap();
        assert_eq!(input, expected_data);
    }

    #[test]
    fn test_des_encrypt() {
        let mut input = [
            0xFD, 0x0E, 0x64, 0x06, 0x65, 0xBE, 0x74, 0x13, //
            0x77, 0x63, 0x3B, 0x02, 0x45, 0x4E, 0x70, 0x7A, //
        ];
        let expected_data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6];

        let des = QrcDes::new(b"TEST!KEY", DESMode::Encrypt);
        des.transform_bytes(&mut input).unwrap();
        assert_eq!(input, expected_data);
    }
}
