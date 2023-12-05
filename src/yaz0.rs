// Based on https://gist.github.com/Mr-Wiseguy/6cca110d74b32b5bb19b76cfa2d7ab4f

use std::cmp;

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

fn divide_round_up(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}

pub fn compress_yaz0(bytes: &[u8]) -> Box<[u8]> {
    let input_size = bytes.len();
    // Worst-case size for output is zero compression on the input, meaning the input size plus the number of layout bytes plus the Yaz0 header.
    // There would be one layout byte for every 8 input bytes, so the worst-case size is:
    //   input_size + ROUND_UP_DIVIDE(input_size, 8) + 0x10
    let mut output: Vec<u8> =
        Vec::with_capacity(input_size + divide_round_up(input_size, 8) + 0x10);

    output.extend(b"Yaz0");
    output.extend((input_size as u32).to_be_bytes());
    // padding
    output.extend(&[0, 0, 0, 0, 0, 0, 0, 0]);

    output.push(0);
    let mut index_cur_layout_byte: usize = 0x10;
    let mut index_out_ptr: usize = index_cur_layout_byte + 1;
    let mut input_pos: usize = 0;
    let mut cur_layout_bit: u8 = 0x80;

    while input_pos < input_size {
        let mut group_pos: i32 = 0;
        let mut group_size: u32 = 0;

        search(
            input_pos,
            input_size,
            &mut group_pos,
            &mut group_size,
            bytes,
        );

        // If the group isn't larger than 2 bytes, copying the input without compression is smaller
        if group_size <= 2 {
            // Set the current layout bit to indicate that this is an uncompressed byte
            output[index_cur_layout_byte] |= cur_layout_bit;
            output.push(bytes[input_pos]);
            input_pos += 1;
            index_out_ptr += 1;
        } else {
            let mut new_size: u32 = 0;
            let mut new_position: i32 = 0;

            // Search for a new group after one position after the current one
            search(
                input_pos + 1,
                input_size,
                &mut new_position,
                &mut new_size,
                bytes,
            );

            // If the new group is better than the current group by at least 2 bytes, use it one instead
            if new_size >= group_size + 2 {
                // Mark the current layout bit to skip compressing this byte, as the next input position yielded better compression
                output[index_cur_layout_byte] |= cur_layout_bit;
                // Copy the input byte to the output
                output.push(bytes[input_pos]);
                input_pos += 1;
                index_out_ptr += 1;

                // Advance to the next layout bit
                cur_layout_bit >>= 1;

                if cur_layout_bit == 0 {
                    cur_layout_bit = 0x80;
                    index_cur_layout_byte = index_out_ptr;
                    index_out_ptr += 1;
                    output[index_cur_layout_byte] = 0;
                }

                group_size = new_size;
                group_pos = new_position;
            }

            // Calculate the offset for the current group
            let group_offset: u32 = (input_pos as i32 - group_pos - 1) as u32;

            // Determine which encoding to use for the current group
            if group_size >= 0x12 {
                // Three bytes, 0RRRNN
                output.push((group_offset >> 8) as u8);
                index_out_ptr += 1;
                output.push((group_offset & 0xFF) as u8);
                index_out_ptr += 1;
                output.push((group_size - 0x12) as u8);
                index_out_ptr += 1;
            } else {
                // Two bytes, NRRR
                output.push((group_offset >> 8) as u8 | ((group_size - 2) << 4) as u8);
                index_out_ptr += 1;
                output.push((group_offset & 0xFF) as u8);
                index_out_ptr += 1;
            }

            // Move forward in the input by the size of the group
            input_pos += group_size as usize;
        }

        // Advance to the next layout bit
        cur_layout_bit >>= 1;

        if cur_layout_bit == 0 {
            cur_layout_bit = 0x80;
            index_cur_layout_byte = index_out_ptr;
            output.push(0);
            index_out_ptr += 1;
        }
    }

    output.shrink_to_fit();
    output.into_boxed_slice()
}

fn search(
    input_pos: usize,
    input_size: usize,
    pos_out: &mut i32,
    size_out: &mut u32,
    data_in: &[u8],
) {
    let mut cur_size: usize = 3;
    let mut found_pos: isize = 0;
    let mut search_pos: usize = cmp::max(input_pos as isize - 0x1000, 0) as usize;
    let search_size = cmp::min(input_size - input_pos, 0x111);

    if search_size >= 3 {
        while search_pos < input_pos {
            let found_offset = mischarsearch(
                &data_in[input_pos..],
                cur_size,
                &data_in[search_pos..],
                cur_size + input_pos - search_pos,
            );

            if found_offset >= input_pos - search_pos {
                break;
            }

            while cur_size < search_size {
                if data_in[cur_size + search_pos + found_offset] != data_in[cur_size + input_pos] {
                    break;
                }
                cur_size += 1;
            }

            if search_size == cur_size {
                *pos_out = (found_offset + search_pos) as i32;
                *size_out = cur_size as u32;
                return;
            }

            found_pos = (search_pos + found_offset) as isize;
            search_pos = (found_pos + 1) as usize;
            cur_size += 1;
        }

        *pos_out = found_pos as i32;
        if cur_size > 3 {
            cur_size -= 1;
            *size_out = cur_size as u32;
            return;
        }
    } else {
        *pos_out = 0;
    }
    *size_out = 0;
}

fn mischarsearch(pattern: &[u8], pattern_len: usize, data: &[u8], data_len: usize) -> usize {
    let mut skip_table = [0u16; 256];
    let mut i: isize;
    //let mut k: usize;

    let mut v6: isize;
    let mut j: isize;

    if pattern_len <= data_len {
        initskip(pattern, pattern_len as i32, &mut skip_table);

        i = pattern_len as isize - 1;
        loop {
            if pattern[pattern_len - 1] == data[i as usize] {
                i -= 1;
                j = pattern_len as isize - 2;
                if j < 0 {
                    return (i + 1) as usize;
                }

                while pattern[j as usize] == data[i as usize] {
                    i -= 1;
                    j -= 1;
                    if j < 0 {
                        return (i + 1) as usize;
                    }
                }

                v6 = pattern_len as isize - j;

                if skip_table[data[i as usize] as usize] as isize > v6 {
                    v6 = skip_table[data[i as usize] as usize] as isize;
                }
            } else {
                v6 = skip_table[data[i as usize] as usize] as isize;
            }
            i += v6;
        }
    }
    data_len
}

fn initskip(pattern: &[u8], len: i32, skip: &mut [u16; 256]) {
    skip.fill(len as u16);

    for i in 0..len {
        skip[pattern[i as usize] as usize] = (len - i - 1) as u16;
    }
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
    fn test_matching_compression() {
        let compressed_file = include_bytes!("../test_data/Yaz0/1.Yaz0");
        let decompressed_file = include_bytes!("../test_data/Yaz0/1.bin");

        let compressed = super::compress_yaz0(decompressed_file.as_slice());
        assert_eq!(compressed_file, compressed.as_ref());
    }

    #[test]
    fn test_cycle_decompressed() {
        let decompressed_file = include_bytes!("../test_data/Yaz0/1.bin");

        assert_eq!(
            decompressed_file,
            super::decompress_yaz0(&super::compress_yaz0(decompressed_file.as_ref())).as_ref()
        );
    }

    #[test]
    fn test_cycle_compressed() {
        let compressed_file = include_bytes!("../test_data/Yaz0/1.Yaz0");

        assert_eq!(
            compressed_file,
            super::compress_yaz0(&super::decompress_yaz0(compressed_file.as_ref())).as_ref()
        );
    }
}
