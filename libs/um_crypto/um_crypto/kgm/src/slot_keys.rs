use crate::header::Header;
use crate::KugouError;

pub fn get_slot_key(header: &Header) -> Result<&'static [u8], KugouError> {
    match header.key_slot {
        1 => Ok(b"l,/'"),
        slot => Err(KugouError::UnsupportedKeySlot(slot)),
    }
}
