use crate::is_valid_password_chr;
use std::collections::HashMap;

const GUESS_PLAIN_TEXT: [u8; 0x20] = [
    0x00, 0x00, 0x00, 0x00, 0x66, 0x74, 0x79, 0x70, 0x4D, 0x34, 0x41, 0x20, 0x00, 0x00, 0x00, 0x00,
    0x4D, 0x34, 0x41, 0x20, 0x6D, 0x70, 0x34, 0x32, 0x69, 0x73, 0x6F, 0x6D, 0x00, 0x00, 0x00, 0x00,
];

type ByteFreq = HashMap<u8, usize>;
fn get_highest_freq_item(freq: &ByteFreq) -> u8 {
    let mut current_item = 0u8;
    let mut current_count = 0usize;

    for (&item, &count) in freq.iter() {
        if count > current_count {
            current_item = item;
            current_count = count;
        }
    }

    current_item
}

pub fn guess_key(buffer: &[u8]) -> Option<[u8; 0x20]> {
    if buffer.len() < 0x100 {
        // buffer too small
        None?
    }

    let mut key = [0u8; 0x20];
    key.copy_from_slice(&buffer[0..0x20]);

    for (k, plain) in key.iter_mut().zip(GUESS_PLAIN_TEXT) {
        *k = k.wrapping_sub(plain);
    }
    if !&key[0x04..0x1C].iter().all(|&k| is_valid_password_chr(k)) {
        // Includes non-password chr
        None?
    }

    let mut password_0x03_freq = ByteFreq::new();
    let mut password_0x1c_freq = ByteFreq::new();
    let mut password_0x1d_freq = ByteFreq::new();
    let mut password_0x1e_freq = ByteFreq::new();
    let mut password_0x1f_freq = ByteFreq::new();

    let increment_password_freq_count = |freq: &mut ByteFreq, item: u8| {
        if is_valid_password_chr(item) {
            freq.entry(item)
                .and_modify(|counter| *counter += 1)
                .or_insert(1);
        }
    };

    for chunk in buffer[..0x100].chunks(0x20) {
        increment_password_freq_count(&mut password_0x03_freq, chunk[0x03]);
        increment_password_freq_count(&mut password_0x1c_freq, chunk[0x1c]);
        increment_password_freq_count(&mut password_0x1d_freq, chunk[0x1d]);
        increment_password_freq_count(&mut password_0x1e_freq, chunk[0x1e]);
        increment_password_freq_count(&mut password_0x1f_freq, chunk[0x1f]);
    }
    key[0x03] = get_highest_freq_item(&password_0x03_freq);
    key[0x1C] = get_highest_freq_item(&password_0x1c_freq);
    key[0x1D] = get_highest_freq_item(&password_0x1d_freq);
    key[0x1E] = get_highest_freq_item(&password_0x1e_freq);
    key[0x1F] = get_highest_freq_item(&password_0x1f_freq);

    if is_valid_password_chr(key[0x03]) && key[0x1c..].iter().all(|&c| is_valid_password_chr(c)) {
        Some(key)
    } else {
        None
    }
}
