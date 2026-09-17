mod key;

use crate::v1::cipher::{qmc1_transform, V1_KEY_SIZE};
use crate::v2_map::key::key_compress;
use crate::QmcCryptoError;

#[derive(Debug, PartialEq, Clone)]
pub struct QMC2Map {
    key: [u8; V1_KEY_SIZE],
}

impl QMC2Map {
    pub fn new<T: AsRef<[u8]>>(key: T) -> Result<Self, QmcCryptoError> {
        let key = key_compress(key)?;
        Ok(Self { key })
    }

    pub fn decrypt<T>(&self, data: &mut T, offset: usize)
    where
        T: AsMut<[u8]> + ?Sized,
    {
        for (i, datum) in data.as_mut().iter_mut().enumerate() {
            *datum = qmc1_transform(&self.key, *datum, offset + i);
        }
    }
}

#[test]
fn test_decrypt() {
    let key = (b'a'..=b'z')
        .chain(b'A'..=b'Z')
        .chain(b'0'..=b'9')
        .cycle()
        .take(325)
        .collect::<Vec<u8>>();

    let cipher = QMC2Map::new(key).expect("should not fail");
    let mut actual = [
        0x00u8, 0x9e, 0x41, 0xc1, 0x71, 0x36, 0x00, 0x80, 0xf4, 0x00, 0x75, 0x9e, 0x36, 0x00, 0x14,
        0x8a,
    ];
    cipher.decrypt(&mut actual, 32760);
    assert_eq!(actual, [0u8; 0x10]);
}
