use crate::{is_valid_password_chr, raw_decrypt};

pub fn guess_key(buffer: &[u8]) -> Option<[u8; 0x20]> {
    if buffer.len() < 0x100 {
        // buffer too small
        None?
    }

    let mut key = [0u8; 0x20];
    key.copy_from_slice(&buffer[0x40..0x60]);
    if !key.iter().all(|&k| is_valid_password_chr(k)) {
        // Not valid password
        None?
    }

    let mut test_riff = [0u8; 4];
    test_riff.copy_from_slice(&buffer[0..4]);
    raw_decrypt(&mut test_riff, &key, 0x00);

    let mut test_data = [0u8; 4];
    test_data.copy_from_slice(&buffer[0x60..0x64]);
    raw_decrypt(&mut test_data, &key, 0x60);

    match (&test_riff, &test_data) {
        (b"RIFF", b"data") => Some(key),
        (_, _) => None,
    }
}
