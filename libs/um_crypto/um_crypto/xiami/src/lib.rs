use thiserror::Error;

#[derive(Error, Debug)]
pub enum XiamiError {
    #[error("header too small, require at least {0} bytes")]
    HeaderTooSmall(usize),

    #[error("not a xiami file")]
    NotXiamiFile,
}

pub struct XiamiFile {
    pub copy_len: usize,
    pub format: [u8; 4],
    key: u8,
}

impl XiamiFile {
    pub fn from_header(buffer: &[u8]) -> Result<Self, XiamiError> {
        if buffer.len() < 0x10 {
            Err(XiamiError::HeaderTooSmall(0x10))?;
        }

        let (format, copy_len, key) = match buffer[..0x10] {
            [b'i', b'f', b'm', b't', f1, f2, f3, f4, 0xfe, 0xfe, 0xfe, 0xfe, a, b, c, key] => {
                let copy_len = (a as usize) | ((b as usize) << 8) | ((c as usize) << 16);
                let format = [f1, f2, f3, f4];
                (format, copy_len, key.wrapping_sub(1))
            }
            _ => Err(XiamiError::NotXiamiFile)?,
        };

        Ok(Self {
            copy_len,
            format,
            key,
        })
    }

    pub fn decrypt(&self, buffer: &mut [u8]) {
        for b in buffer.iter_mut() {
            *b = self.key.wrapping_sub(*b);
        }
    }
}
