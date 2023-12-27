use crate::{utils, Crunch64Error};

fn parse_header(bytes: &[u8]) -> Result<(usize, usize, usize), Crunch64Error> {
    if bytes.len() < 0x10 {
        return Err(Crunch64Error::InvalidMio0Header);
    }

    if &bytes[0..4] != b"MIO0" {
        return Err(Crunch64Error::InvalidMio0Header);
    }

    let decompressed_size = utils::read_u32(bytes, 0x4)? as usize;
    let link_table_offset = utils::read_u32(bytes, 0x8)? as usize;
    let chunk_offset = utils::read_u32(bytes, 0xC)? as usize;

    Ok((decompressed_size, link_table_offset, chunk_offset))
}

/*
fn write_header(
    dst: &mut Vec<u8>,
    uncompressed_size: usize,
    link_table_offset: usize,
    chunk_offset: usize,
) -> Result<(), Crunch64Error> {
    dst.extend(b"MIO0");
    dst.extend((uncompressed_size as u32).to_be_bytes());
    dst.extend((link_table_offset as u32).to_be_bytes());
    dst.extend((chunk_offset as u32).to_be_bytes());

    Ok(())
}
*/

pub fn decompress(bytes: &[u8]) -> Result<Box<[u8]>, Crunch64Error> {
    let (decompressed_size,
        comp_offset,
        uncomp_offset) = parse_header(bytes)?;

    let mut layout_data_index = 0x10;
    let mut uncompressed_data_index = uncomp_offset;
    let mut compressed_data_index = comp_offset;

    let mut mask_bit_counter = 0;
    let mut current_mask = 0;

    let mut idx: usize = 0;
    let mut ret: Vec<u8> = vec![0u8; decompressed_size];

    while idx < decompressed_size {
        if mask_bit_counter == 0 {
            current_mask = utils::read_u32(bytes, layout_data_index)?;
            layout_data_index += 4;
            mask_bit_counter = 32;
        }

        if current_mask & 0x80000000 != 0 {
            ret[idx] = bytes[uncompressed_data_index];
            uncompressed_data_index += 1;
            idx += 1;
        } else {
            let length_offset = utils::read_u16(bytes, compressed_data_index)?;
            compressed_data_index += 2;

            let length = ((length_offset >> 12) + 3) as usize;
            let index =  ((length_offset & 0xFFF) + 1) as usize;
            let offset = idx - index;

            if ! (3 <= length && length <= 18) {
                return Err(Crunch64Error::CorruptData);
            }

            if !(1 <= index && index <= 4096) {
                return Err(Crunch64Error::CorruptData);
            }

            for i in 0..length {
                ret[idx] = ret[offset + i];
                idx += 1;
            }
        }

        current_mask <<= 1;
        mask_bit_counter -= 1;
    }

    Ok(ret.into_boxed_slice())
}

#[cfg(feature = "c_bindings")]
mod c_bindings {
    #[no_mangle]
    pub extern "C" fn crunch64_mio0_decompress_bound(
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

    #[no_mangle]
    pub extern "C" fn crunch64_mio0_decompress(
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

    /*
    #[no_mangle]
    pub extern "C" fn crunch64_mio0_compress_bound(
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
    pub extern "C" fn crunch64_mio0_compress(
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
    */
}

#[cfg(feature = "python_bindings")]
pub(crate) mod python_bindings {
    use pyo3::prelude::*;
    use std::borrow::Cow;

    #[pyfunction]
    pub(crate) fn decompress_mio0(bytes: &[u8]) -> Result<Cow<[u8]>, super::Crunch64Error> {
        Ok(Cow::Owned(super::decompress(bytes)?.into()))
    }

    //#[pyfunction]
    //pub(crate) fn compress_mio0(bytes: &[u8]) -> Result<Cow<[u8]>, super::Crunch64Error> {
    //    Ok(Cow::Owned(super::compress(bytes)?.into()))
    //}
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
        #[files("../test_data/*.MIO0")] path: PathBuf,
    ) -> Result<(), Crunch64Error> {
        let compressed_file = &read_test_file(path.clone());
        let decompressed_file = &read_test_file(path.with_extension(""));

        let decompressed = super::decompress(compressed_file)?;
        assert_eq!(decompressed_file, decompressed.as_ref());
        Ok(())
    }

    //#[rstest]
    //fn test_matching_compression(
    //    #[files("../test_data/*.MIO0")] path: PathBuf,
    //) -> Result<(), Crunch64Error> {
    //    let compressed_file = &read_test_file(path.clone());
    //    let decompressed_file = &read_test_file(path.with_extension(""));
//
    //    let compressed = super::compress(decompressed_file.as_slice())?;
    //    assert_eq!(compressed_file, compressed.as_ref());
    //    Ok(())
    //}

    //#[rstest]
    //fn test_cycle_decompressed(
    //    #[files("../test_data/*.MIO0")] path: PathBuf,
    //) -> Result<(), Crunch64Error> {
    //    let decompressed_file = &read_test_file(path.with_extension(""));
//
    //    assert_eq!(
    //        decompressed_file,
    //        super::decompress(&super::compress(decompressed_file.as_ref())?)?.as_ref()
    //    );
    //    Ok(())
    //}

    //#[rstest]
    //fn test_cycle_compressed(
    //    #[files("../test_data/*.MIO0")] path: PathBuf,
    //) -> Result<(), Crunch64Error> {
    //    let compressed_file = &read_test_file(path);
//
    //    assert_eq!(
    //        compressed_file,
    //        super::compress(&super::decompress(compressed_file.as_ref())?)?.as_ref()
    //    );
    //    Ok(())
    //}
}
