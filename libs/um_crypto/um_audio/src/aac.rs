pub const SYNC_FRAME_TEST_SIZE: usize = 4096;

pub fn is_aac(magic: u32) -> bool {
    // Frame sync should have the first 12 bits set to 1.
    const AAC_AND_MASK: u32 = 0b1111_1111_1111_0110u32 << 16;
    const AAC_EXPECTED: u32 = 0b1111_1111_1111_0000u32 << 16;

    (magic & AAC_AND_MASK) == AAC_EXPECTED
}
