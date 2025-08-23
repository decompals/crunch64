use crate::{Crunch64Error, utils};

fn parse_header(bytes: &[u8]) -> Result<(usize, usize, usize), Crunch64Error> {
    if bytes.len() < 0x10 {
        return Err(Crunch64Error::InvalidYay0Header);
    }

    if &bytes[0..4] != b"Yay0" {
        return Err(Crunch64Error::InvalidYay0Header);
    }

    let decompressed_size = utils::read_u32(bytes, 0x4)? as usize;
    let link_table_offset = utils::read_u32(bytes, 0x8)? as usize;
    let chunk_offset = utils::read_u32(bytes, 0xC)? as usize;

    Ok((decompressed_size, link_table_offset, chunk_offset))
}

fn write_header(
    dst: &mut Vec<u8>,
    uncompressed_size: usize,
    link_table_offset: usize,
    chunk_offset: usize,
) -> Result<(), Crunch64Error> {
    dst.extend(b"Yay0");
    dst.extend((uncompressed_size as u32).to_be_bytes());
    dst.extend((link_table_offset as u32).to_be_bytes());
    dst.extend((chunk_offset as u32).to_be_bytes());

    Ok(())
}

pub fn decompress(bytes: &[u8]) -> Result<Box<[u8]>, Crunch64Error> {
    let (decompressed_size, link_table_offset, chunk_offset) = parse_header(bytes)?;

    let mut link_table_idx = link_table_offset;
    let mut chunk_idx = chunk_offset;
    let mut other_idx = 0x10;

    let mut mask_bit_counter = 0;
    let mut current_mask = 0;

    // Preallocate result and index into it
    let mut idx: usize = 0;
    let mut ret: Vec<u8> = vec![0u8; decompressed_size];

    while idx < decompressed_size {
        // If we're out of bits, get the next mask
        if mask_bit_counter == 0 {
            current_mask = utils::read_u32(bytes, other_idx)?;
            other_idx += 4;
            mask_bit_counter = 32;
        }

        if current_mask & 0x80000000 != 0 {
            ret[idx] = bytes[chunk_idx];
            idx += 1;
            chunk_idx += 1;
        } else {
            let link = utils::read_u16(bytes, link_table_idx)? as usize;
            link_table_idx += 2;

            let offset = idx - (link & 0xFFF);

            let mut count = (link >> 12) as usize;

            if count == 0 {
                let count_modifier = bytes[chunk_idx];
                chunk_idx += 1;
                count = count_modifier as usize + 18;
            } else {
                count += 2;
            }

            for i in 0..count {
                ret[idx] = ret[offset + i - 1];
                idx += 1;
            }
        }

        current_mask <<= 1;
        mask_bit_counter -= 1;
    }

    Ok(ret.into_boxed_slice())
}

fn size_for_compressed_buffer(input_size: usize) -> Result<usize, Crunch64Error> {
    // Taken from Yaz0
    Ok(input_size + input_size.div_ceil(8) + 0x10)
}

pub fn compress(bytes: &[u8]) -> Result<Box<[u8]>, Crunch64Error> {
    let input_size = bytes.len();

    let mut window = utils::Window::new(bytes);

    let mut pp: usize = 0;
    let mut index_cur_layout_byte: usize = 0;

    let mut cmd: Vec<u32> = vec![0; 0x4000];
    let mut pol: Vec<u16> = Vec::with_capacity(2 * 0x1000);
    let mut def: Vec<u8> = Vec::with_capacity(4 * 0x1000);

    let mut input_pos: usize = 0;
    let mut cur_layout_bit: u32 = 0x80000000;

    while input_pos < input_size {
        let (mut group_pos, mut group_size) = window.search(input_pos, 0x111);

        // If the group isn't larger than 2 bytes, copying the input without compression is smaller
        if group_size <= 2 {
            // Set the current layout bit to indicate that this is an uncompressed byte
            cmd[index_cur_layout_byte] |= cur_layout_bit;
            def.push(bytes[input_pos]);
            input_pos += 1;
        } else {
            // Search for a new group after one position after the current one
            let (new_position, new_size) = window.search(input_pos + 1, 0x111);

            // If the new group is better than the current group by at least 2 bytes, use it instead
            if new_size >= group_size + 2 {
                // Mark the current layout bit to skip compressing this byte, as the next input position yielded better compression
                cmd[index_cur_layout_byte] |= cur_layout_bit;
                def.push(bytes[input_pos]);
                input_pos += 1;

                // Advance to the next layout bit
                cur_layout_bit >>= 1;

                if cur_layout_bit == 0 {
                    cur_layout_bit = 0x80000000;
                    index_cur_layout_byte += 1;
                    cmd[index_cur_layout_byte] = 0;
                }

                group_size = new_size;
                group_pos = new_position;
            }

            // Calculate the offset for the current group
            let group_offset = input_pos - group_pos as usize - 1;

            // Determine which encoding to use for the current group
            if group_size >= 0x12 {
                pol.push(group_offset as u16);
                pp += 1;
                def.push((group_size - 0x12) as u8);
            } else {
                pol.push((group_offset | (((group_size as u16 - 2) as usize) << 12)) as u16);
                pp += 1;
            }

            // Move forward in the input by the size of the group
            input_pos += group_size as usize;
        }

        // Advance to the next layout bit
        cur_layout_bit >>= 1;

        if cur_layout_bit == 0 {
            cur_layout_bit = 0x80000000;
            index_cur_layout_byte += 1;
            cmd[index_cur_layout_byte] = 0;
        }
    }

    if cur_layout_bit != 0x80000000 {
        index_cur_layout_byte += 1;
    }

    let link_table_offset: usize = 4 * index_cur_layout_byte + 16;
    let chunk_offset: usize = 2 * pp + link_table_offset;

    let mut output: Vec<u8> = Vec::with_capacity(size_for_compressed_buffer(input_size)?);

    write_header(&mut output, input_size, link_table_offset, chunk_offset)?;

    for &value in &cmd[..index_cur_layout_byte] {
        output.extend(&value.to_be_bytes());
    }

    for &value in &pol[..pp] {
        output.extend(&value.to_be_bytes());
    }

    output.extend(&def);

    Ok(output.into_boxed_slice())
}

#[cfg(feature = "c_bindings")]
mod c_bindings {
    #[unsafe(no_mangle)]
    pub extern "C" fn crunch64_yay0_decompress_bound(
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
            Ok(d) => d,
        };

        match super::parse_header(&bytes) {
            Err(e) => return e,
            Ok((value, _, _)) => unsafe { *dst_size = value },
        }

        super::Crunch64Error::Okay
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn crunch64_yay0_decompress(
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

    #[unsafe(no_mangle)]
    pub extern "C" fn crunch64_yay0_compress_bound(
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

    #[unsafe(no_mangle)]
    pub extern "C" fn crunch64_yay0_compress(
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
    pub(crate) fn decompress_yay0(bytes: Cow<[u8]>) -> Result<Cow<[u8]>, super::Crunch64Error> {
        Ok(Cow::Owned(super::decompress(&bytes)?.into()))
    }

    #[pyfunction]
    pub(crate) fn compress_yay0(bytes: Cow<[u8]>) -> Result<Cow<[u8]>, super::Crunch64Error> {
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
        #[files("../test_data/*.Yay0")] path: PathBuf,
    ) -> Result<(), Crunch64Error> {
        let compressed_file = &read_test_file(path.clone());
        let decompressed_file = &read_test_file(path.with_extension(""));

        let decompressed = super::decompress(compressed_file)?;
        assert_eq!(decompressed_file, decompressed.as_ref());
        Ok(())
    }

    #[rstest]
    fn test_matching_compression(
        #[files("../test_data/*.Yay0")] path: PathBuf,
    ) -> Result<(), Crunch64Error> {
        let compressed_file = &read_test_file(path.clone());
        let decompressed_file = &read_test_file(path.with_extension(""));

        let compressed = super::compress(decompressed_file.as_slice())?;
        assert_eq!(compressed_file, compressed.as_ref());
        Ok(())
    }

    #[rstest]
    fn test_cycle_decompressed(
        #[files("../test_data/*.Yay0")] path: PathBuf,
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
        #[files("../test_data/*.Yay0")] path: PathBuf,
    ) -> Result<(), Crunch64Error> {
        let compressed_file = &read_test_file(path);

        assert_eq!(
            compressed_file,
            super::compress(&super::decompress(compressed_file.as_ref())?)?.as_ref()
        );
        Ok(())
    }
}
