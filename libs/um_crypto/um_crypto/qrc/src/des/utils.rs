use crate::des::constants::U64_SHIFT_TABLE_CACHE;

pub const fn make_u64(hi32: u32, lo32: u32) -> u64 {
    ((hi32 as u64) << 32) | (lo32 as u64)
}

pub const fn swap_u64(value: u64) -> u64 {
    (value.wrapping_shr(32)) | (value.wrapping_shl(32))
}

pub const fn lo32(value: u64) -> u32 {
    value as u32
}

pub const fn hi32(value: u64) -> u32 {
    value.wrapping_shr(32) as u32
}

pub const fn get_u64_by_shift_idx(value: u8) -> u64 {
    // 1u64.wrapping_shl(31u32.wrapping_sub(value as u32))
    // This is not portable, so let's use a pre-computed table...

    U64_SHIFT_TABLE_CACHE[value as usize]
}

pub fn map_bit(result: u64, src: u64, check: u8, set: u8) -> u64 {
    match get_u64_by_shift_idx(check) & src {
        0 => result,
        _ => result | get_u64_by_shift_idx(set),
    }
}

pub fn map_u32_bits(src_value: u32, table: &[u8]) -> u32 {
    let stream = table.iter().enumerate();

    stream.fold(0u64, |result, (i, &check_idx)| {
        map_bit(result, src_value as u64, check_idx, i as u8)
    }) as u32
}

pub fn map_u64(src_value: u64, table: &[u8]) -> u64 {
    assert_eq!(table.len() % 2, 0, "table.len() should be even");

    let (table_lo32, table_hi32) = table.split_at(table.len() / 2);

    let mut lo32 = 0u64;
    let mut hi32 = 0u64;

    for (i, (&idx_lo32, &idx_hi32)) in table_lo32.iter().zip(table_hi32).enumerate() {
        lo32 = map_bit(lo32, src_value, idx_lo32, i as u8);
        hi32 = map_bit(hi32, src_value, idx_hi32, i as u8);
    }

    make_u64(hi32 as u32, lo32 as u32)
}
