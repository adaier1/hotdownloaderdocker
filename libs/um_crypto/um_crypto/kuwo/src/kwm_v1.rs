#[derive(Debug, PartialEq, Clone)]
pub struct CipherV1 {
    key: [u8; 0x20],
}

const KEY: [u8; 0x20] = [
    0x4D, 0x6F, 0x4F, 0x74, 0x4F, 0x69, 0x54, 0x76, 0x49, 0x4E, 0x47, 0x77, 0x64, 0x32, 0x45, 0x36,
    0x6E, 0x30, 0x45, 0x31, 0x69, 0x37, 0x4C, 0x35, 0x74, 0x32, 0x49, 0x6F, 0x4F, 0x6F, 0x4E, 0x6B,
];

impl CipherV1 {
    pub fn new(resource_id: u32) -> Self {
        let mut key = KEY;
        let resource_id = resource_id.to_string();
        for (k, &r) in key.iter_mut().zip(resource_id.as_bytes().iter().cycle()) {
            *k ^= r;
        }

        Self { key }
    }

    pub fn decrypt<T>(&self, data: &mut T, offset: usize)
    where
        T: AsMut<[u8]> + ?Sized,
    {
        let data = data.as_mut();

        for (datum, offset) in data.iter_mut().zip(offset..) {
            *datum ^= self.key[offset % self.key.len()];
        }
    }
}
