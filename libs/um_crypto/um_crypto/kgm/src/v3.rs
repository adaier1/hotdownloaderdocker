use crate::header::Header;
use crate::slot_keys::get_slot_key;
use crate::KugouError;

pub struct DecipherV3 {
    slot_key: [u8; 16],
    file_key: [u8; 17],
}

impl DecipherV3 {
    fn hash_key<T: AsRef<[u8]>>(data: T) -> [u8; 16] {
        let digest = umc_utils::md5(data);
        let mut result = [0u8; 16];
        for (result, digest) in result.rchunks_exact_mut(2).zip(digest.chunks_exact(2)) {
            result[0] = digest[0];
            result[1] = digest[1];
        }
        result
    }

    pub fn new(header: &Header) -> Result<Self, KugouError> {
        let slot_key = Self::hash_key(get_slot_key(header)?);

        let mut file_key = [0x6b; 17];
        file_key[..16].copy_from_slice(&Self::hash_key(header.file_key));

        Ok(Self { slot_key, file_key })
    }

    fn offset_key(offset: usize) -> u8 {
        let offset_key = (offset as u32).to_ne_bytes();
        offset_key[0] ^ offset_key[1] ^ offset_key[2] ^ offset_key[3]
    }

    pub fn decrypt<T: AsMut<[u8]> + ?Sized>(&self, buffer: &mut T, offset: usize) {
        for (datum, offset) in buffer.as_mut().iter_mut().zip(offset..) {
            let offset_key = Self::offset_key(offset);

            let file_key = self.file_key[offset % self.file_key.len()];
            let slot_key = self.slot_key[offset % self.slot_key.len()];

            let mut temp = *datum;
            temp ^= file_key;
            temp ^= temp << 4;
            temp ^= slot_key;
            temp ^= offset_key;
            *datum = temp;
        }
    }
}
