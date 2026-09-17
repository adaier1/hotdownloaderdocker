use crate::AudioError;
use byteorder::{ByteOrder, BE, LE};

fn parse_id3_sync_safe_int(buffer: &[u8]) -> i32 {
    const UNSAFE_INT_MASK_32: u32 = 0x80808080;
    const U32_BYTE_MASK_1: u32 = 0xFF000000;
    const U32_BYTE_MASK_2: u32 = 0x00FF0000;
    const U32_BYTE_MASK_3: u32 = 0x0000FF00;
    const U32_BYTE_MASK_4: u32 = 0x000000FF;

    let value = BE::read_u32(buffer);

    // Sync safe int should use only lower 7-bits of each byte.
    if (value & UNSAFE_INT_MASK_32) != 0 {
        return 0;
    }

    let value = (value & U32_BYTE_MASK_1) >> 3
        | (value & U32_BYTE_MASK_2) >> 2
        | (value & U32_BYTE_MASK_3) >> 1
        | (value & U32_BYTE_MASK_4);

    value as i32
}

const MIN_ID3_HEADER_LEN: usize = 10;

fn get_id3_header_size(buffer: &[u8], offset: usize) -> Result<usize, AudioError> {
    if buffer.len() < offset + MIN_ID3_HEADER_LEN {
        Err(AudioError::NeedMoreHeader(offset + MIN_ID3_HEADER_LEN))?;
    }

    let buffer = &buffer[offset..];

    // TAG: ID3v1, 128 bytes
    if buffer.starts_with(b"TAG") {
        return Ok(128);
    }

    // ID3: ID3v2
    if buffer.starts_with(b"ID3") {
        // offset    value
        //      0    header('ID3')
        //      3    uint8_t(ver_major) uint8_t(ver_minor)
        //      5    uint8_t(flags)
        //      6    uint32_t(inner_tag_size)
        //     10    byte[inner_tag_size] id3v2 data
        //     ??    byte[*] original_file_content
        let inner_size = parse_id3_sync_safe_int(&buffer[6..10]) as usize;
        return Ok(10 + inner_size);
    }

    Ok(0)
}

const MIN_APE_V2_HEADER_LEN: usize = 32;
fn get_ape_v2_size(buffer: &[u8], offset: usize) -> Result<usize, AudioError> {
    if buffer.len() < MIN_APE_V2_HEADER_LEN {
        Err(AudioError::NeedMoreHeader(offset + MIN_APE_V2_HEADER_LEN))?;
    }

    if buffer.starts_with(b"APETAGEX") {
        let extra_size = LE::read_u32(&buffer[0x0c..0x10]) as usize;
        return Ok(MIN_APE_V2_HEADER_LEN + extra_size);
    }

    Ok(0)
}

pub fn get_header_metadata_size(buffer: &[u8], offset: usize) -> Result<usize, AudioError> {
    let mut offset = offset;

    // Workaround: Some files have multiple ID3v2 tags, max 5 times.
    for _ in 0..5 {
        let len = match get_id3_header_size(buffer, offset)? {
            0 => get_ape_v2_size(buffer, offset)?,
            len => len,
        };

        if len == 0 {
            break;
        }
        offset += len;
    }

    Ok(offset)
}
