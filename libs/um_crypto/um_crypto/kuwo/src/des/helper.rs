pub const fn make_u64(hi32: u32, lo32: u32) -> u64 {
    ((hi32 as u64) << 32) | (lo32 as u64)
}

pub const fn swap_u64_side(value: u64) -> u64 {
    (value.wrapping_shr(32)) | (value.wrapping_shl(32))
}

pub const fn u64_get_lo32(value: u64) -> u32 {
    value as u32
}

pub const fn u64_get_hi32(value: u64) -> u32 {
    value.wrapping_shr(32) as u32
}

pub fn get_u64_by_shift_idx(value: u8) -> u64 {
    if value == 255 {
        return 0;
    }

    if cfg!(target_pointer_width = "64") {
        1u64.wrapping_shl(value as u32)
    } else {
        super::constants::U64_SHIFT_TABLE_CACHE
            .get(value as usize)
            .copied()
            .unwrap_or_default()
    }
}

#[test]
fn test_get_u64_by_shift_idx() {
    assert_eq!(get_u64_by_shift_idx(0), 1);
    assert_eq!(get_u64_by_shift_idx(63), 0x8000000000000000);
}

pub fn map_u64(src_value: u64, table: &[u8]) -> u64 {
    table.iter().enumerate().fold(0u64, |acc, (i, &idx)| {
        match get_u64_by_shift_idx(idx) & src_value {
            0 => acc,
            _ => acc | get_u64_by_shift_idx(i as u8),
        }
    })
}

pub fn map_u32(src_value: u32, table: &[u8]) -> u32 {
    map_u64(src_value as u64, table) as u32
}
