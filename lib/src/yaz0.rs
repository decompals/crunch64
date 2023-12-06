// Based on https://gist.github.com/Mr-Wiseguy/6cca110d74b32b5bb19b76cfa2d7ab4f

use crate::{
    utils::{self},
    Crunch64Error,
};

pub fn decompress_yaz0(bytes: &[u8]) -> Result<Box<[u8]>, Crunch64Error> {
    if &bytes[0..4] != b"Yaz0" {
        return Err(Crunch64Error::InvalidYaz0Header);
    }

    // Skip the header
    let mut index_src = 0x10;
    let mut index_dst = 0;

    let uncompressed_size = utils::read_u32(bytes, 4)? as usize;
    let mut ret = vec![0u8; uncompressed_size];

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

    Ok(ret.into_boxed_slice())
}

fn divide_round_up(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}

fn size_for_compressed_buffer(input_size: usize) -> Option<usize> {
    // Worst-case size for output is zero compression on the input, meaning the input size plus the number of layout bytes plus the Yaz0 header.
    // There would be one layout byte for every 8 input bytes, so the worst-case size is:
    //   input_size + ROUND_UP_DIVIDE(input_size, 8) + 0x10
    Some(input_size + divide_round_up(input_size, 8) + 0x10)
}

pub fn compress_yaz0(bytes: &[u8]) -> Result<Box<[u8]>, Crunch64Error> {
    let input_size = bytes.len();

    let comp_buffer_size = size_for_compressed_buffer(input_size);
    // if comp_buffer_size.is_none() {}
    let mut output: Vec<u8> = Vec::with_capacity(comp_buffer_size.unwrap());

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

        utils::search(
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
            utils::search(
                input_pos + 1,
                input_size,
                &mut new_position,
                &mut new_size,
                bytes,
            );

            // If the new group is better than the current group by at least 2 bytes, use it instead
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
                    output.push(0);
                    index_out_ptr += 1;
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

    Ok(output.into_boxed_slice())
}

mod c_bindings {
    // TODO: better name
    #[no_mangle]
    pub extern "C" fn crunch64_decompress_yaz0_get_dst_buffer_size(
        dst_size: *mut usize,
        src_len: usize,
        src: *const u8,
    ) -> bool {
        if src_len < 0x10 {
            return false;
        }

        if dst_size.is_null() || src.is_null() {
            return false;
        }

        let mut bytes = Vec::with_capacity(0x10);
        for i in 0..0x10 {
            bytes.push(unsafe { *src.offset(i as isize) });
        }

        if &bytes[0..4] != b"Yaz0" {
            return false;
        }

        match super::utils::read_u32(&bytes, 4) {
            Err(_) => {
                return false;
            }
            Ok(value) => {
                unsafe { *dst_size = value as usize };
            }
        }

        true
    }

    #[no_mangle]
    pub extern "C" fn crunch64_decompress_yaz0(
        dst_len: *mut usize,
        dst: *mut u8,
        src_len: usize,
        src: *const u8,
    ) -> bool {
        if dst_len.is_null() || dst.is_null() || src.is_null() {
            return false;
        }

        let mut bytes = Vec::with_capacity(src_len);

        for i in 0..src_len {
            bytes.push(unsafe { *src.offset(i as isize) });
        }

        if &bytes[0..4] != b"Yaz0" {
            return false;
        }

        match super::decompress_yaz0(&bytes) {
            Err(_) => {
                return false;
            }
            Ok(dec) => {
                // `dst_len` is expected to point to the size of the `dst` pointer,
                // we use this to check if the decompressed data will fit in `dst`
                if dec.len() > unsafe { *dst_len } {
                    return false;
                }

                for (i, b) in dec.iter().enumerate() {
                    unsafe {
                        *dst.offset(i as isize) = *b;
                    }
                }
                unsafe {
                    *dst_len = dec.len();
                }
            }
        }

        true
    }

    // TODO: better name
    #[no_mangle]
    pub extern "C" fn crunch64_compress_yaz0_get_dst_buffer_size(
        dst_size: *mut usize,
        src_len: usize,
        src: *const u8,
    ) -> bool {
        if dst_size.is_null() || src.is_null() {
            return false;
        }

        let _ = src;
        let uncompressed_size = super::size_for_compressed_buffer(src_len);

        if uncompressed_size.is_none() {
            return false;
        }

        unsafe { *dst_size = uncompressed_size.unwrap() };

        true
    }

    #[no_mangle]
    pub extern "C" fn crunch64_compress_yaz0(
        dst_len: *mut usize,
        dst: *mut u8,
        src_len: usize,
        src: *const u8,
    ) -> bool {
        if dst_len.is_null() || dst.is_null() || src.is_null() {
            return false;
        }

        let mut bytes = Vec::with_capacity(src_len);

        for i in 0..src_len {
            bytes.push(unsafe { *src.offset(i as isize) });
        }

        match super::compress_yaz0(&bytes) {
            Err(_) => {
                return false;
            }
            Ok(data) => {
                // `dst_len` is expected to point to the size of the `dst` pointer,
                // we use this to check if the decompressed data will fit in `dst`
                if data.len() > unsafe { *dst_len } {
                    return false;
                }

                for (i, b) in data.iter().enumerate() {
                    unsafe {
                        *dst.offset(i as isize) = *b;
                    }
                }
                unsafe {
                    *dst_len = data.len();
                }
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use crate::Crunch64Error;
    use core::panic;
    use rstest::rstest;
    use std::{
        fs::File,
        io::{BufReader, Read},
        path::PathBuf,
    };

    pub fn read_test_file(path: PathBuf) -> Vec<u8> {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(_error) => {
                panic!("Failed to open file");
            }
        };

        let mut buf_reader = BufReader::new(file);
        let mut buffer = Vec::new();

        let _ = buf_reader.read_to_end(&mut buffer);

        buffer
    }

    #[rstest]
    fn test_matching_decompression(
        #[files("../test_data/*.Yaz0")] path: PathBuf,
    ) -> Result<(), Crunch64Error> {
        let compressed_file = &read_test_file(path.clone());
        let decompressed_file = &read_test_file(path.with_extension(""));

        let decompressed: Box<[u8]> = super::decompress_yaz0(compressed_file)?;
        assert_eq!(decompressed_file, decompressed.as_ref());
        Ok(())
    }

    #[rstest]
    fn test_matching_compression(
        #[files("../test_data/*.Yaz0")] path: PathBuf,
    ) -> Result<(), Crunch64Error> {
        let compressed_file = &read_test_file(path.clone());
        let decompressed_file = &read_test_file(path.with_extension(""));

        let compressed = super::compress_yaz0(decompressed_file.as_slice())?;
        assert_eq!(compressed_file, compressed.as_ref());
        Ok(())
    }

    #[rstest]
    fn test_cycle_decompressed(
        #[files("../test_data/*.Yaz0")] path: PathBuf,
    ) -> Result<(), Crunch64Error> {
        let decompressed_file = &read_test_file(path.with_extension(""));

        assert_eq!(
            decompressed_file,
            super::decompress_yaz0(&super::compress_yaz0(decompressed_file.as_ref())?)?.as_ref()
        );
        Ok(())
    }

    #[rstest]
    fn test_cycle_compressed(
        #[files("../test_data/*.Yaz0")] path: PathBuf,
    ) -> Result<(), Crunch64Error> {
        let compressed_file = &read_test_file(path);

        assert_eq!(
            compressed_file,
            super::compress_yaz0(&super::decompress_yaz0(compressed_file.as_ref())?)?.as_ref()
        );
        Ok(())
    }
}
