const V1_OFFSET_BOUNDARY: usize = 0x7FFF;

pub const V1_KEY_SIZE: usize = 128;

#[inline]
pub fn qmc1_transform(key: &[u8; V1_KEY_SIZE], value: u8, offset: usize) -> u8 {
    let offset = match offset {
        0..=V1_OFFSET_BOUNDARY => offset,
        offset => offset % V1_OFFSET_BOUNDARY,
    };

    value ^ key[offset % V1_KEY_SIZE]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::generate_key_128;

    #[test]
    fn test_decode_start() {
        let test_key = generate_key_128();
        let mut data = *b"igohj&pg{fo";
        for (i, datum) in data.iter_mut().enumerate() {
            *datum = qmc1_transform(&test_key, *datum, i);
        }
        assert_eq!(data, *b"hello world");
    }
    #[test]
    fn test_decode_boundary() {
        let test_key = generate_key_128();
        let mut data = [
            0x13, 0x19, 0x11, 0x12, 0x10, 0xa0, 0x75, 0x6c, 0x76, 0x69, 0x62,
        ];
        for (i, datum) in data.iter_mut().enumerate() {
            *datum = qmc1_transform(&test_key, *datum, 0x7FFA + i);
        }
        assert_eq!(data, *b"hello world");
    }
}
