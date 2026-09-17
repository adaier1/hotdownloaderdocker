fn is_base64_chr(chr: u8) -> bool {
    chr.is_ascii_alphanumeric() || (chr == b'+') || (chr == b'/') || (chr == b'=')
}

pub fn is_base64(s: &[u8]) -> bool {
    s.iter().all(|&c| is_base64_chr(c))
}

/// Convert UTF-16 LE string (within ASCII char range) to UTF-8
pub fn from_ascii_utf16(data: &[u8]) -> String {
    let data = data
        .chunks_exact(2)
        .take_while(|chunk| chunk[0] != 0 && chunk[0].is_ascii() && chunk[1] == 0)
        .map(|chunk| chunk[0])
        .collect::<Vec<_>>();
    String::from_utf8_lossy(&data).to_string()
}
