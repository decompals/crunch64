pub fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    if offset % 4 != 0 {
        panic!("Unaligned offset");
    }

    u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
