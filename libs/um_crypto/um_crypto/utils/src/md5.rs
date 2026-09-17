use md5::{Digest, Md5};

/// Calculate the MD5 hash (non-modified) of a buffer.
pub fn md5(buffer: impl AsRef<[u8]>) -> [u8; 0x10] {
    Md5::digest(buffer).into()
}

pub fn md5_2<T1: AsRef<[u8]>, T2: AsRef<[u8]>>(buffer1: T1, buffer2: T2) -> [u8; 16] {
    let mut md5_digest = Md5::default();
    md5_digest.update(buffer1);
    md5_digest.update(buffer2);
    md5_digest.finalize().into()
}
