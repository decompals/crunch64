pub fn decompress_yay0(bytes: Vec<u8>) -> Vec<u8> {
    let decompressed_size = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
    let link_table_offset = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
    let chunk_offset = u32::from_be_bytes(bytes[12..16].try_into().unwrap());

    let mut link_table_idx = link_table_offset as usize;
    let mut chunk_idx = chunk_offset as usize;
    let mut other_idx = 16;

    let mut mask_bit_counter = 0;
    let mut current_mask = 0;

    // Preallocate result and index into it
    let mut idx: usize = 0;
    let mut ret = vec![0u8; decompressed_size as usize];

    while idx < decompressed_size as usize {
        // If we're out of bits, get the next mask
        if mask_bit_counter == 0 {
            current_mask = u32::from_be_bytes(bytes[other_idx..other_idx + 4].try_into().unwrap());
            other_idx += 4;
            mask_bit_counter = 32;
        }

        if current_mask & 0x80000000 != 0 {
            ret[idx] = bytes[chunk_idx];
            idx += 1;
            chunk_idx += 1;
        } else {
            let link = u16::from_be_bytes(
                bytes[link_table_idx..link_table_idx + 2]
                    .try_into()
                    .unwrap(),
            );
            link_table_idx += 2;

            let offset = idx as isize - (link as isize & 0xFFF);

            let mut count = (link >> 12) as usize;

            if count == 0 {
                let count_modifier = bytes[chunk_idx];
                chunk_idx += 1;
                count = count_modifier as usize + 18;
            } else {
                count = count + 2;
            }

            for i in 0..count {
                ret[idx] = ret[(offset + i as isize - 1) as usize];
                idx += 1;
            }
        }

        current_mask <<= 1;
        mask_bit_counter -= 1;
    }

    ret
}

pub fn compress_yay0(_bytes: Vec<u8>) -> Vec<u8> {
    panic!("Not implemented")
}
