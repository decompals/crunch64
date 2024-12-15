// Based on https://gist.github.com/Mr-Wiseguy/6cca110d74b32b5bb19b76cfa2d7ab4f

use crate::{utils, Crunch64Error};

fn parse_header(bytes: &[u8]) -> Result<usize, Crunch64Error> {
    if bytes.len() < 0x10 {
        return Err(Crunch64Error::InvalidYaz0Header);
    }

    if &bytes[0..4] != b"Yaz0" {
        return Err(Crunch64Error::InvalidYaz0Header);
    }

    if bytes[8..0x10] != [0u8; 8] {
        return Err(Crunch64Error::InvalidYaz0Header);
    }

    Ok(utils::read_u32(bytes, 4)? as usize)
}

fn write_header(dst: &mut Vec<u8>, uncompressed_size: usize) -> Result<(), Crunch64Error> {
    dst.extend(b"Yaz0");
    dst.extend((uncompressed_size as u32).to_be_bytes());
    // padding
    dst.extend(&[0u8; 8]);

    Ok(())
}

pub fn decompress(bytes: &[u8]) -> Result<Box<[u8]>, Crunch64Error> {
    let uncompressed_size = parse_header(bytes)?;

    // Skip the header
    let mut index_src = 0x10;
    let mut index_dst = 0;

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

fn size_for_compressed_buffer(input_size: usize) -> Result<usize, Crunch64Error> {
    // Worst-case size for output is zero compression on the input, meaning the input size plus the number of layout bytes plus the Yaz0 header.
    // There would be one layout byte for every 8 input bytes, so the worst-case size is:
    //   input_size + ROUND_UP_DIVIDE(input_size, 8) + 0x10
    Ok(input_size + input_size.div_ceil(8) + 0x10)
}

pub fn compress(bytes: &[u8]) -> Result<Box<[u8]>, Crunch64Error> {
    let input_size = bytes.len();

    let mut output: Vec<u8> = Vec::with_capacity(size_for_compressed_buffer(input_size)?);
    let mut window = utils::Window::new(bytes);

    write_header(&mut output, input_size)?;

    let mut index_cur_layout_byte: usize = 0x10;
    let mut index_out_ptr: usize = index_cur_layout_byte;
    let mut input_pos: usize = 0;
    let mut cur_layout_bit: u8 = 1;

    while input_pos < input_size {
        // Advance to the next layout bit
        cur_layout_bit >>= 1;

        if cur_layout_bit == 0 {
            cur_layout_bit = 0x80;
            index_cur_layout_byte = index_out_ptr;
            output.push(0);
            index_out_ptr += 1;
        }

        let (mut group_pos, mut group_size) = window.search(input_pos, 0x111);

        // If the group isn't larger than 2 bytes, copying the input without compression is smaller
        if group_size <= 2 {
            // Set the current layout bit to indicate that this is an uncompressed byte
            output[index_cur_layout_byte] |= cur_layout_bit;
            output.push(bytes[input_pos]);
            input_pos += 1;
            index_out_ptr += 1;
        } else {
            // Search for a new group after one position after the current one
            let (new_position, new_size) = window.search(input_pos + 1, 0x111);

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
            let group_offset = input_pos as u32 - group_pos - 1;

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
    }

    Ok(output.into_boxed_slice())
}

#[cfg(feature = "c_bindings")]
mod c_bindings {
    #[no_mangle]
    pub extern "C" fn crunch64_yaz0_decompress_bound(
        dst_size: *mut usize,
        src_len: usize,
        src: *const u8,
    ) -> super::Crunch64Error {
        if src_len < 0x10 {
            return super::Crunch64Error::OutOfBounds;
        }

        if dst_size.is_null() || src.is_null() {
            return super::Crunch64Error::NullPointer;
        }

        let bytes = match super::utils::u8_vec_from_pointer_array(0x10, src) {
            Err(e) => return e,
            Ok(data) => data,
        };

        match super::parse_header(&bytes) {
            Err(e) => return e,
            Ok(value) => unsafe { *dst_size = value },
        }

        super::Crunch64Error::Okay
    }

    #[no_mangle]
    pub extern "C" fn crunch64_yaz0_decompress(
        dst_len: *mut usize,
        dst: *mut u8,
        src_len: usize,
        src: *const u8,
    ) -> super::Crunch64Error {
        if dst_len.is_null() || dst.is_null() || src.is_null() {
            return super::Crunch64Error::NullPointer;
        }

        let bytes = match super::utils::u8_vec_from_pointer_array(src_len, src) {
            Err(e) => return e,
            Ok(d) => d,
        };

        let data = match super::decompress(&bytes) {
            Err(e) => return e,
            Ok(d) => d,
        };

        if let Err(e) = super::utils::set_pointer_array_from_u8_array(dst_len, dst, &data) {
            return e;
        }

        super::Crunch64Error::Okay
    }

    #[no_mangle]
    pub extern "C" fn crunch64_yaz0_compress_bound(
        dst_size: *mut usize,
        src_len: usize,
        src: *const u8,
    ) -> super::Crunch64Error {
        if dst_size.is_null() || src.is_null() {
            return super::Crunch64Error::NullPointer;
        }

        match super::size_for_compressed_buffer(src_len) {
            Err(e) => return e,
            Ok(uncompressed_size) => unsafe { *dst_size = uncompressed_size },
        }

        super::Crunch64Error::Okay
    }

    #[no_mangle]
    pub extern "C" fn crunch64_yaz0_compress(
        dst_len: *mut usize,
        dst: *mut u8,
        src_len: usize,
        src: *const u8,
    ) -> super::Crunch64Error {
        if dst_len.is_null() || dst.is_null() || src.is_null() {
            return super::Crunch64Error::NullPointer;
        }

        let bytes = match super::utils::u8_vec_from_pointer_array(src_len, src) {
            Err(e) => return e,
            Ok(d) => d,
        };

        let data = match super::compress(&bytes) {
            Err(e) => return e,
            Ok(d) => d,
        };

        if let Err(e) = super::utils::set_pointer_array_from_u8_array(dst_len, dst, &data) {
            return e;
        }

        super::Crunch64Error::Okay
    }
}

#[cfg(feature = "python_bindings")]
pub(crate) mod python_bindings {
    use pyo3::prelude::*;
    use std::borrow::Cow;

    /**
     * We use a `Cow` instead of a plain &[u8] because the latter only allows Python's
     * `bytes` objects, while `Cow`` allows for both `bytes` and `bytearray`.
     * This is important because an argument typed as `bytes` allows to pass a
     * `bytearray` object too.
     */

    #[pyfunction]
    pub(crate) fn decompress_yaz0(bytes: Cow<[u8]>) -> Result<Cow<[u8]>, super::Crunch64Error> {
        Ok(Cow::Owned(super::decompress(&bytes)?.into()))
    }

    #[pyfunction]
    pub(crate) fn compress_yaz0(bytes: Cow<[u8]>) -> Result<Cow<[u8]>, super::Crunch64Error> {
        Ok(Cow::Owned(super::compress(&bytes)?.into()))
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

        let decompressed: Box<[u8]> = super::decompress(compressed_file)?;
        assert_eq!(decompressed_file, decompressed.as_ref());
        Ok(())
    }

    #[rstest]
    fn test_matching_compression(
        #[files("../test_data/*.Yaz0")] path: PathBuf,
    ) -> Result<(), Crunch64Error> {
        let compressed_file = &read_test_file(path.clone());
        let decompressed_file = &read_test_file(path.with_extension(""));

        let compressed = super::compress(decompressed_file.as_slice())?;
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
            super::decompress(&super::compress(decompressed_file.as_ref())?)?.as_ref()
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
            super::compress(&super::decompress(compressed_file.as_ref())?)?.as_ref()
        );
        Ok(())
    }
}
