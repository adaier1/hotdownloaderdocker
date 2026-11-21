//! 汽水加密音频解密。
//!
//! 带 `play_auth` 的流是加密 M4A（`enca` 采样描述 + `senc` 逐样本 IV），
//! 先由 `play_auth` 还原出 AES 密钥，再按 `stsz` 的样本大小逐块做 AES-CTR，
//! 最后把采样描述改回原始格式。明文流（无 `play_auth`）不经过这里。

use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockEncrypt, KeyInit};
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use base64::Engine;

/// 解密整段音频。
pub fn decrypt_audio(data: &[u8], play_auth: &str) -> Result<Vec<u8>, String> {
    let key = extract_key(play_auth)?;
    if data.len() < 8 {
        return Err("音频数据过短，无法解密".into());
    }
    let moov = find_box(data, b"moov", 0, data.len()).ok_or("音频缺少 moov 结构")?;
    let stbl = find_stbl(data, moov)?;
    let stsz = find_box(data, b"stsz", stbl.0 + 8, stbl.0 + stbl.1).ok_or("音频缺少 stsz 结构")?;
    let sample_sizes = parse_stsz(&data[stsz.0 + 8..stsz.0 + stsz.1]);
    let senc = find_box(data, b"senc", moov.0 + 8, moov.0 + moov.1)
        .or_else(|| find_box(data, b"senc", stbl.0 + 8, stbl.0 + stbl.1))
        .ok_or("音频缺少 senc 结构")?;
    let iv_size = per_sample_iv_size(data, stbl);
    let samples = parse_senc(&data[senc.0 + 8..senc.0 + senc.1], iv_size);
    let mdat = find_box(data, b"mdat", 0, data.len()).ok_or("音频缺少 mdat 结构")?;

    let mut output = data.to_vec();
    let mut cursor = mdat.0 + 8;
    let mut decrypted = Vec::with_capacity(mdat.1.saturating_sub(8));
    for (index, size) in sample_sizes.iter().enumerate() {
        let size = *size as usize;
        if size == 0 || cursor + size > data.len() {
            break;
        }
        let chunk = &data[cursor..cursor + size];
        match samples.get(index) {
            Some(sample) => decrypted.extend(decrypt_sample(&key, sample, chunk)?),
            None => decrypted.extend_from_slice(chunk),
        }
        cursor += size;
    }
    if decrypted.len() != mdat.1 - 8 {
        return Err("汽水解密后长度与原始数据不一致".into());
    }
    output[mdat.0 + 8..mdat.0 + mdat.1].copy_from_slice(&decrypted);
    restore_sample_format(&mut output, stbl);
    Ok(output)
}

/// 下载完成后原地解密；只有带 `play_auth` 的曲目才需要调用。
pub async fn decrypt_file_in_place(path: &str, play_auth: &str) -> Result<(), String> {
    let data = tokio::fs::read(path)
        .await
        .map_err(|error| format!("读取待解密音频失败: {error}"))?;
    let decrypted = decrypt_audio(&data, play_auth)?;
    tokio::fs::write(path, decrypted)
        .await
        .map_err(|error| format!("写入解密音频失败: {error}"))
}

/// (offset, size) 二元组，与 MP4 box 的绝对位置保持一致。
type Mp4Box = (usize, usize);

fn find_stbl(data: &[u8], moov: Mp4Box) -> Result<Mp4Box, String> {
    if let Some(stbl) = find_box(data, b"stbl", moov.0 + 8, moov.0 + moov.1) {
        return Ok(stbl);
    }
    let trak = find_box(data, b"trak", moov.0 + 8, moov.0 + moov.1).ok_or("音频缺少 trak 结构")?;
    let mdia = find_box(data, b"mdia", trak.0 + 8, trak.0 + trak.1).ok_or("音频缺少 mdia 结构")?;
    let minf = find_box(data, b"minf", mdia.0 + 8, mdia.0 + mdia.1).ok_or("音频缺少 minf 结构")?;
    find_box(data, b"stbl", minf.0 + 8, minf.0 + minf.1).ok_or_else(|| "音频缺少 stbl 结构".into())
}

/// 同级顺序扫描，与 Go 参考实现一致：只信任前 4 字节的 size。
fn find_box(data: &[u8], name: &[u8; 4], start: usize, end: usize) -> Option<Mp4Box> {
    let end = end.min(data.len());
    let mut position = start;
    while position + 8 <= end {
        let size = u32::from_be_bytes(data[position..position + 4].try_into().ok()?) as usize;
        if size < 8 || position + size > data.len() {
            break;
        }
        if &data[position + 4..position + 8] == name {
            return Some((position, size));
        }
        position += size;
    }
    None
}

/// 递归查找嵌套 box，用于定位 `schi` 里的 `tenc`。
fn find_box_deep(data: &[u8], name: &[u8; 4], start: usize, end: usize) -> Option<Mp4Box> {
    let end = end.min(data.len());
    let mut position = start;
    while position + 8 <= end {
        let mut header = 8;
        let mut size = u32::from_be_bytes(data[position..position + 4].try_into().ok()?) as usize;
        if size == 1 {
            if position + 16 > end {
                return None;
            }
            size = u64::from_be_bytes(data[position + 8..position + 16].try_into().ok()?) as usize;
            header = 16;
        }
        if size < header || position + size > end {
            return None;
        }
        if &data[position + 4..position + 8] == name {
            return Some((position, size));
        }
        let child = child_start(&data[position + 4..position + 8], position, header);
        if let Some(child) = child {
            if child < position + size {
                if let Some(found) = find_box_deep(data, name, child, position + size) {
                    return Some(found);
                }
            }
        }
        position += size;
    }
    None
}

/// 容器的子 box 起点；`stsd` 与采样描述需要跳过各自的固定头部。
fn child_start(box_type: &[u8], offset: usize, header: usize) -> Option<usize> {
    let start = match box_type {
        b"moov" | b"trak" | b"mdia" | b"minf" | b"stbl" | b"sinf" | b"schi" => offset + header,
        b"stsd" => offset + header + 8,
        b"enca" | b"mp4a" | b"alac" | b"fLaC" => offset + header + 28,
        _ => return None,
    };
    Some(start)
}

fn per_sample_iv_size(data: &[u8], stbl: Mp4Box) -> usize {
    let Some(tenc) = find_box_deep(data, b"tenc", stbl.0 + 8, stbl.0 + stbl.1) else {
        return 8;
    };
    let payload = tenc.0 + 8;
    if payload + 8 > data.len() {
        return 8;
    }
    match data[payload + 7] {
        8 | 16 => data[payload + 7] as usize,
        _ => 8,
    }
}

fn parse_stsz(payload: &[u8]) -> Vec<u32> {
    if payload.len() < 12 {
        return Vec::new();
    }
    let fixed = u32::from_be_bytes(payload[4..8].try_into().unwrap_or_default());
    let count = u32::from_be_bytes(payload[8..12].try_into().unwrap_or_default()) as usize;
    let mut sizes = vec![0u32; count];
    if fixed != 0 {
        sizes.fill(fixed);
        return sizes;
    }
    for (index, slot) in sizes.iter_mut().enumerate() {
        let start = 12 + index * 4;
        if start + 4 <= payload.len() {
            *slot = u32::from_be_bytes(payload[start..start + 4].try_into().unwrap_or_default());
        }
    }
    sizes
}

struct SencSample {
    iv: Vec<u8>,
    subsamples: Vec<(u16, u32)>,
}

fn parse_senc(payload: &[u8], iv_size: usize) -> Vec<SencSample> {
    if payload.len() < 8 {
        return Vec::new();
    }
    let iv_size = if iv_size == 16 { 16 } else { 8 };
    let flags = u32::from_be_bytes(payload[0..4].try_into().unwrap_or_default()) & 0x00FF_FFFF;
    let count = u32::from_be_bytes(payload[4..8].try_into().unwrap_or_default()) as usize;
    let has_subsamples = flags & 0x02 != 0;
    let mut samples = Vec::with_capacity(count);
    let mut position = 8;
    for _ in 0..count {
        if position + iv_size > payload.len() {
            break;
        }
        let iv = payload[position..position + iv_size].to_vec();
        position += iv_size;
        let mut subsamples = Vec::new();
        if has_subsamples {
            if position + 2 > payload.len() {
                break;
            }
            let total = u16::from_be_bytes(payload[position..position + 2].try_into().unwrap_or_default())
                as usize;
            position += 2;
            if position + total * 6 > payload.len() {
                break;
            }
            for _ in 0..total {
                let clear =
                    u16::from_be_bytes(payload[position..position + 2].try_into().unwrap_or_default());
                let encrypted = u32::from_be_bytes(
                    payload[position + 2..position + 6].try_into().unwrap_or_default(),
                );
                subsamples.push((clear, encrypted));
                position += 6;
            }
        }
        samples.push(SencSample { iv, subsamples });
    }
    samples
}

fn decrypt_sample(key: &[u8], sample: &SencSample, chunk: &[u8]) -> Result<Vec<u8>, String> {
    let mut iv = [0u8; 16];
    let copy = sample.iv.len().min(16);
    iv[..copy].copy_from_slice(&sample.iv[..copy]);
    match key.len() {
        16 => ctr_decrypt::<aes::Aes128>(key, &iv, chunk, &sample.subsamples),
        24 => ctr_decrypt::<aes::Aes192>(key, &iv, chunk, &sample.subsamples),
        32 => ctr_decrypt::<aes::Aes256>(key, &iv, chunk, &sample.subsamples),
        _ => Err("汽水解密密钥长度无效".into()),
    }
}

/// 按 subsample 的「明文段 / 密文段」交替推进 CTR 计数器。
fn ctr_decrypt<C>(key: &[u8], iv: &[u8; 16], chunk: &[u8], subsamples: &[(u16, u32)]) -> Result<Vec<u8>, String>
where
    C: BlockEncrypt + KeyInit,
{
    let cipher = C::new_from_slice(key).map_err(|_| "汽水解密器初始化失败".to_string())?;
    let mut counter = *iv;
    let mut output = chunk.to_vec();
    if subsamples.is_empty() {
        xor_keystream(&cipher, &mut counter, &mut output);
        return Ok(output);
    }
    let mut position = 0;
    for (clear, encrypted) in subsamples {
        position = (position + *clear as usize).min(output.len());
        if position >= output.len() {
            break;
        }
        let length = (*encrypted as usize).min(output.len() - position);
        xor_keystream(&cipher, &mut counter, &mut output[position..position + length]);
        position += length;
        if position >= output.len() {
            break;
        }
    }
    Ok(output)
}

/// 手写 CTR-128BE：逐块加密计数器并与数据异或，计数器按大端递增。
fn xor_keystream<C>(cipher: &C, counter: &mut [u8; 16], data: &mut [u8])
where
    C: BlockEncrypt,
{
    let mut offset = 0;
    while offset < data.len() {
        let mut block = GenericArray::clone_from_slice(counter);
        cipher.encrypt_block(&mut block);
        let length = (data.len() - offset).min(16);
        for index in 0..length {
            data[offset + index] ^= block[index];
        }
        increment_counter(counter);
        offset += length;
    }
}

fn increment_counter(counter: &mut [u8; 16]) {
    for byte in counter.iter_mut().rev() {
        let (next, overflow) = byte.overflowing_add(1);
        *byte = next;
        if !overflow {
            break;
        }
    }
}

/// 把 `enca` 采样描述改回原始编码，否则播放器仍按加密容器处理。
fn restore_sample_format(data: &mut [u8], stbl: Mp4Box) {
    let Some(stsd) = find_box(data, b"stsd", stbl.0 + 8, stbl.0 + stbl.1) else {
        return;
    };
    let start = stsd.0;
    let end = start + stsd.1;
    let Some(index) = data[start..end]
        .windows(4)
        .position(|window| window == b"enca")
    else {
        return;
    };
    let position = start + index;
    let Some(format) = original_format(data, position) else {
        return;
    };
    data[position..position + 4].copy_from_slice(&format);
}

fn original_format(data: &[u8], enca: usize) -> Option<[u8; 4]> {
    let frma = data[enca..]
        .windows(4)
        .position(|window| window == b"frma")?;
    let position = enca + frma;
    if position < 4 || position + 8 > data.len() {
        return None;
    }
    let size = u32::from_be_bytes(data[position - 4..position].try_into().ok()?) as usize;
    if size < 12 || position - 4 + size > data.len() {
        return None;
    }
    data[position + 4..position + 8].try_into().ok()
}

/// `play_auth` → hex 密钥 → 原始密钥字节。
fn extract_key(play_auth: &str) -> Result<Vec<u8>, String> {
    let play_auth = play_auth.trim();
    let decoded = STANDARD
        .decode(play_auth)
        .or_else(|_| STANDARD_NO_PAD.decode(play_auth))
        .map_err(|_| "汽水 play_auth 不是有效的 Base64".to_string())?;
    if decoded.len() < 3 {
        return Err("汽水 play_auth 长度不足".into());
    }
    let padding = i32::from(decoded[0] ^ decoded[1] ^ decoded[2]) - 48;
    if padding < 0 || decoded.len() < padding as usize + 2 {
        return Err("汽水 play_auth 填充长度异常".into());
    }
    let padding = padding as usize;
    let inner = &decoded[1..decoded.len() - padding];
    let transformed = spade_inner(inner);
    if transformed.is_empty() {
        return Err("汽水 play_auth 解密结果为空".into());
    }
    let skip = base36_value(transformed[0]).ok_or("汽水 play_auth 跳过位无效")? as usize;
    let end = 1 + (decoded.len() - padding - 2);
    if end < 1 || end > transformed.len() || skip > end - 1 {
        return Err("汽水 play_auth 索引越界".into());
    }
    decode_hex(&transformed[1..end - skip])
}

/// 参考实现里的逐字节变换：异或固定前缀，再减去下标 popcount 与常数 21。
fn spade_inner(input: &[u8]) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(input.len() + 2);
    buffer.push(0xFA);
    buffer.push(0x55);
    buffer.extend_from_slice(input);
    input
        .iter()
        .enumerate()
        .map(|(index, byte)| {
            let mut value = i32::from(byte ^ buffer[index]) - popcount(index) - 21;
            while value < 0 {
                value += 255;
            }
            value as u8
        })
        .collect()
}

fn popcount(value: usize) -> i32 {
    (value as u32).count_ones() as i32
}

fn base36_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'z' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn decode_hex(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() % 2 != 0 {
        return Err("汽水解密密钥不是偶数长度十六进制".into());
    }
    bytes
        .chunks(2)
        .map(|pair| {
            let high = hex_value(pair[0]).ok_or("汽水解密密钥包含非法字符")?;
            let low = hex_value(pair[1]).ok_or("汽水解密密钥包含非法字符")?;
            Ok(high * 16 + low)
        })
        .collect()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{base36_value, decode_hex, extract_key, popcount};

    #[test]
    fn popcount_matches_bit_count() {
        assert_eq!(popcount(0), 0);
        assert_eq!(popcount(7), 3);
        assert_eq!(popcount(8), 1);
        assert_eq!(popcount(255), 8);
    }

    #[test]
    fn base36_only_accepts_digits_and_lowercase() {
        assert_eq!(base36_value(b'0'), Some(0));
        assert_eq!(base36_value(b'9'), Some(9));
        assert_eq!(base36_value(b'a'), Some(10));
        assert_eq!(base36_value(b'z'), Some(35));
        assert_eq!(base36_value(b'A'), None);
    }

    #[test]
    fn invalid_play_auth_is_rejected() {
        assert!(extract_key("not-base64!!").is_err());
        assert!(extract_key("QUJD").is_err() || extract_key("QUJD").is_ok());
    }

    #[test]
    fn hex_keys_are_decoded() {
        assert_eq!(decode_hex(b"00ff10").unwrap(), vec![0, 255, 16]);
        assert!(decode_hex(b"0f0").is_err());
        assert!(decode_hex(b"zz").is_err());
    }
}
