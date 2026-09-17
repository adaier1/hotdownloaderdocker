use crate::header::Header;
use crate::slot_keys::get_slot_key;
use crate::KugouError;

pub struct DecipherV2 {
    key: [u8; 4],
}

impl DecipherV2 {
    pub fn new(header: &Header) -> Result<Self, KugouError> {
        let mut key = [0u8; 4];
        key.copy_from_slice(get_slot_key(header)?);
        Ok(Self { key })
    }

    pub fn decrypt<T: AsMut<[u8]> + ?Sized>(&self, buffer: &mut T, offset: usize) {
        for (datum, offset) in buffer.as_mut().iter_mut().zip(offset..) {
            *datum ^= self.key[offset % self.key.len()];
        }
    }
}

#[test]
fn test_v2_init() -> Result<(), KugouError> {
    let hdr = Header::from_buffer(include_bytes!("__fixtures__/kgm_v2_hdr.bin"))?;
    DecipherV2::new(&hdr)?;

    Ok(())
}
