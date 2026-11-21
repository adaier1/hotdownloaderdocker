//! 二维码 PNG 编码：把登录链接渲染成可直接转发到聊天窗口的图片。
//!
//! 浏览器端用的是 `data:image/svg+xml`，聊天与图片预览对 SVG 支持参差，
//! 因此 MCP 侧统一转成 PNG。这里只依赖 `qrcode` 的矩阵输出，自带最小
//! PNG 写出（8 位灰度 + 存储型 deflate），避免为一个二维码引入 image 这类重依赖。

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use qrcode::QrCode;

/// 静区：扫码器需要四周留白，否则贴边容易识别失败。
const QUIET_ZONE: u32 = 4;
/// 每个模块放大到多少像素，8px 在手机屏幕上足够清晰。
const MODULE_SCALE: u32 = 8;

/// 渲染二维码并返回 PNG 字节。
pub fn png_for_text(text: &str) -> Result<Vec<u8>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("二维码内容为空".into());
    }
    let code = QrCode::new(text.as_bytes()).map_err(|error| format!("生成二维码失败: {error}"))?;
    let modules = code.width() as u32;
    let colors = code.to_colors();
    let side = modules + QUIET_ZONE * 2;
    let size = side * MODULE_SCALE;

    // 每行前置一个 filter 字节（0 = None），随后是灰度像素；
    // 一个模块行重复 MODULE_SCALE 次，才是 SCALE 働高的像素行。
    let mut raw = Vec::with_capacity((size as usize + 1) * size as usize);
    for y in 0..side {
        let mut row = Vec::with_capacity(size as usize + 1);
        row.push(0);
        for x in 0..side {
            let dark = match (x.checked_sub(QUIET_ZONE), y.checked_sub(QUIET_ZONE)) {
                (Some(x), Some(y)) if x < modules && y < modules => {
                    colors[(y * modules + x) as usize] == qrcode::Color::Dark
                }
                _ => false,
            };
            let value = if dark { 0 } else { 255 };
            for _ in 0..MODULE_SCALE {
                row.push(value);
            }
        }
        for _ in 0..MODULE_SCALE {
            raw.extend_from_slice(&row);
        }
    }
    Ok(encode_png(size, size, &raw))
}

/// 生成 `data:image/png;base64,...`，供不支持图片内容块的客户端兜底使用。
pub fn png_data_uri(text: &str) -> Result<String, String> {
    let bytes = png_for_text(text)?;
    Ok(format!("data:image/png;base64,{}", STANDARD.encode(bytes)))
}

fn encode_png(width: u32, height: u32, raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len() + 128);
    out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);

    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    // 位深 8、颜色类型 0（灰度）、压缩/过滤/隔行均为 0。
    header.extend_from_slice(&[8, 0, 0, 0, 0]);
    write_chunk(&mut out, b"IHDR", &header);
    write_chunk(&mut out, b"IDAT", &zlib_stored(raw));
    write_chunk(&mut out, b"IEND", &[]);
    out
}

fn write_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

/// zlib 容器 + 存储型 deflate 块：无需压缩库，图片体积仍只有几十 KB。
fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len() + 32);
    // CMF/FLG：deflate、32K 窗口、最快压缩标志。
    out.extend_from_slice(&[0x78, 0x01]);
    let mut offset = 0;
    while offset < raw.len() {
        let len = (raw.len() - offset).min(0xFFFF);
        let last = offset + len == raw.len();
        out.push(u8::from(last));
        out.extend_from_slice(&(len as u16).to_le_bytes());
        out.extend_from_slice(&(!(len as u16)).to_le_bytes());
        out.extend_from_slice(&raw[offset..offset + len]);
        offset += len;
    }
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let (mut low, mut high) = (1u32, 0u32);
    for byte in data {
        low = (low + u32::from(*byte)) % 65521;
        high = (high + low) % 65521;
    }
    (high << 16) | low
}

#[cfg(test)]
mod tests {
    use super::{adler32, crc32, png_for_text};

    fn chunk_types(png: &[u8]) -> Vec<String> {
        let mut types = Vec::new();
        let mut offset = 8;
        while offset + 8 <= png.len() {
            let len = u32::from_be_bytes(png[offset..offset + 4].try_into().unwrap()) as usize;
            let kind = String::from_utf8_lossy(&png[offset + 4..offset + 8]).to_string();
            types.push(kind);
            offset += 12 + len;
        }
        types
    }

    #[test]
    fn png_has_signature_and_expected_chunks() {
        let png = png_for_text("https://music.163.com/login?codekey=abc").unwrap();
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
        assert_eq!(chunk_types(&png), vec!["IHDR", "IDAT", "IEND"]);
        // 宽高写在 IHDR 前 8 字节，边长必须是模块数的整数倍且含静区。
        let width = u32::from_be_bytes(png[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(png[20..24].try_into().unwrap());
        assert_eq!(width, height);
        assert!(width >= 25 * 8 && width % 8 == 0);

        // 存储型 deflate 是可精确预测的：IDAT 长度必须等于
        // zlib 头(2) + 分块开销(每 65535 字节 5 字节) + 原始数据 + adler32(4)。
        // 早期版本把模块行当像素行写少数据，靠这个不变量兜住。
        let raw_len = (width as usize + 1) * height as usize;
        let expected = raw_len + 6 + raw_len.div_ceil(0xFFFF) * 5;
        let idat = png
            .windows(4)
            .position(|window| window == b"IDAT".as_slice())
            .expect("IDAT 位置");
        let idat_len = u32::from_be_bytes(png[idat - 4..idat].try_into().unwrap()) as usize;
        assert_eq!(idat_len, expected, "IDAT 长度与像素数据量不符");
    }

    #[test]
    fn empty_text_is_rejected() {
        assert!(png_for_text("   ").is_err());
    }

    #[test]
    fn checksums_match_known_values() {
        // 标准测试向量，避免自写校验和出现静默偏差。
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }
}
