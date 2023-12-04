// Based on https://gist.github.com/Mr-Wiseguy/6cca110d74b32b5bb19b76cfa2d7ab4f

use crate::utils;

pub fn decompress_yaz0(bytes: &[u8]) -> Box<[u8]> {
    if &bytes[0..4] != b"Yaz0" {
        panic!("not Yaz0 data");
    }

    // Skip the header
    let mut index_src = 0x10;
    let mut index_dst = 0;

    let uncompressed_size = utils::read_u32(bytes, 4) as usize;
    let mut ret = vec![0u8; uncompressed_size as usize];

    while index_src < bytes.len() {
        let mut layout_bit_index = 0;
        let mut layout_bits = bytes[index_src];
        index_src += 1;

        while (layout_bit_index < 8) && (index_src < bytes.len()) && (index_dst < uncompressed_size)
        {
            if (layout_bits & 0x80) != 0 {
                ret[index_dst] = bytes[index_src];
                index_src += 1;
                index_dst += 1;
            } else {
                let first_byte = bytes[index_src];
                index_src += 1;
                let second_byte = bytes[index_src];
                index_src += 1;
                let byte_pair = ((first_byte as u16) << 8) | (second_byte as u16);
                let offset = (byte_pair & 0x0FFF) + 1;
                let mut length: usize;

                // Check how the group length is encoded
                if (first_byte & 0xF0) == 0 {
                    // 3 byte encoding, 0RRRNN
                    let third_byte = bytes[index_src];
                    index_src += 1;
                    length = (third_byte as usize) + 0x12;
                } else {
                    // 2 byte encoding, NRRR
                    length = (((byte_pair & 0xF000) >> 12) + 2) as usize;
                }

                while length > 0 {
                    ret[index_dst] = ret[index_dst - offset as usize];
                    index_dst += 1;
                    length -= 1;
                }
            }

            layout_bit_index += 1;
            layout_bits <<= 1;
        }
    }

    ret.into_boxed_slice()
}

pub fn compress_yaz0(_bytes: &[u8]) -> Box<[u8]> {
    panic!("Not implemented")
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_matching_decompression() {
        let compressed_file = include_bytes!("../test_data/Yaz0/1.Yaz0");
        let decompressed_file = include_bytes!("../test_data/Yaz0/1.bin");

        let decompressed: Box<[u8]> = super::decompress_yaz0(compressed_file);
        assert_eq!(decompressed_file, decompressed.as_ref());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_matching_compression() {
        let compressed_file = include_bytes!("../test_data/Yaz0/1.Yaz0");
        let decompressed_file = include_bytes!("../test_data/Yaz0/1.bin");

        let compressed = super::compress_yaz0(decompressed_file.as_slice());
        assert_eq!(compressed_file, compressed.as_ref());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_cycle() {
        let decompressed_file = include_bytes!("../test_data/Yaz0/1.bin");

        assert_eq!(
            decompressed_file,
            super::decompress_yaz0(&super::compress_yaz0(decompressed_file.as_ref())).as_ref()
        );
    }
}
