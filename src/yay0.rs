pub fn decompress_yay0(bytes: &[u8]) -> Box<[u8]> {
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
    let mut ret: Vec<u8> = vec![0u8; decompressed_size as usize];

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
                count += 2;
            }

            for i in 0..count {
                ret[idx] = ret[(offset + i as isize - 1) as usize];
                idx += 1;
            }
        }

        current_mask <<= 1;
        mask_bit_counter -= 1;
    }

    ret.into_boxed_slice()
}

pub fn compress_yay0(_bytes: &[u8]) -> Box<[u8]> {
    panic!("Not implemented")
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_matching() {
        let compressed_file = include_bytes!("../test_data/Yay0/1.Yay0");
        let decompressed_file = include_bytes!("../test_data/Yay0/1.bin");

        let decompressed: Box<[u8]> = super::decompress_yay0(compressed_file);
        assert_eq!(decompressed_file, decompressed.as_ref());

        // let recompressed = super::compress_yay0(decompressed_file.as_slice());
        // assert_eq!(compressed_file, recompressed);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_cycle() {
        let decompressed_file = include_bytes!("../test_data/Yay0/1.bin");

        assert_eq!(
            decompressed_file,
            super::decompress_yay0(&super::compress_yay0(decompressed_file.as_ref())).as_ref()
        );
    }
}
