use crate::utils;

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

pub fn compress_yay0(bytes: &[u8]) -> Box<[u8]> {
    let mut pp: usize = 0;
    let mut cp: usize = 0;

    let mut cmd: Vec<u32> = vec![0; 0x4000];
    let mut pol: Vec<u16> = Vec::with_capacity(2 * 0x1000);
    let mut def: Vec<u8> = Vec::with_capacity(4 * 0x1000);

    let mut v0: usize = 0;
    let mut v1: u32 = 0x80000000;
    let mut v6: u32 = 1024;
    let mut v7: u32 = 0;
    let mut v8: i32 = 0;

    let mut a3: i32 = 0;
    let mut a4: u32 = 0;

    let insize = bytes.len();

    let mut ret: Vec<u8> = vec![];

    while v0 < insize {
        if v6 < v0 as u32 {
            v6 += 1024;
        }
        search(v0, insize, &mut a3, &mut a4, bytes);

        if a4 <= 2 {
            cmd[cp] |= v1;
            def.push(bytes[v0]);
            v0 += 1;
        } else {
            search(v0 + 1, insize, &mut v8, &mut v7, bytes);
            if v7 > a4 + 1 {
                cmd[cp] |= v1;
                def.push(bytes[v0]);
                v0 += 1;

                v1 >>= 1;
                if v1 == 0 {
                    v1 = 0x80000000;
                    cp += 1;
                    cmd[cp] = 0;
                }

                a4 = v7;
                a3 = v8;
            }

            let v3 = v0 - a3 as usize - 1;
            a3 = (v0 - a3 as usize - 1) as i32;

            if a4 > 0x11 {
                pol.push(v3 as u16);
                pp += 1;
                def.push((a4 - 18) as u8);
            } else {
                pol.push((v3 | (((a4 as u16 - 2) as usize) << 12)) as u16);
                pp += 1;
            }

            v0 += a4 as usize;
        }

        v1 >>= 1;
        if v1 == 0 {
            v1 = 0x80000000;
            cp += 1;
            cmd[cp] = 0;
        }
    }

    if v1 != 0x80000000 {
        cp += 1;
    }

    let offset: u32 = 4 * cp as u32 + 16;
    let offset2: u32 = 2 * pp as u32 + offset;

    let offset_bytes: [u8; 4] = offset.to_be_bytes();
    let offset2_bytes: [u8; 4] = offset2.to_be_bytes();

    ret.extend(b"Yay0");

    ret.extend(&(insize as u32).to_be_bytes());
    ret.extend(offset_bytes);
    ret.extend(offset2_bytes);

    for &value in &cmd[..cp] {
        ret.extend(&value.to_be_bytes());
    }

    for &value in &pol[..pp] {
        ret.extend(&value.to_be_bytes());
    }

    ret.extend(&def);

    ret.into_boxed_slice()
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
    let mut search_pos: usize = 0;

    if input_pos > 0x1000 {
        search_pos = input_pos - 0x1000;
    }

    let mut search_size = 273;

    if input_size - input_pos <= 273 {
        search_size = input_size - input_pos;
    }

    if search_size >= 3 {
        while search_pos < input_pos {
            let found_offset = utils::mischarsearch(
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

#[cfg(test)]
mod tests {
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
    fn test_matching_decompression(#[files("test_data/*.Yay0")] path: PathBuf) {
        let compressed_file = &read_test_file(path.clone());
        let decompressed_file = &read_test_file(path.with_extension(""));

        let decompressed: Box<[u8]> = super::decompress_yay0(compressed_file);
        assert_eq!(decompressed_file, decompressed.as_ref());
    }

    #[rstest]
    fn test_matching_compression(#[files("test_data/*.Yay0")] path: PathBuf) {
        let compressed_file = &read_test_file(path.clone());
        let decompressed_file = &read_test_file(path.with_extension(""));

        let compressed = super::compress_yay0(decompressed_file.as_slice());
        assert_eq!(compressed_file, compressed.as_ref());
    }

    #[rstest]
    fn test_cycle_decompressed(#[files("test_data/*.Yay0")] path: PathBuf) {
        let decompressed_file = &read_test_file(path.with_extension(""));

        assert_eq!(
            decompressed_file,
            super::decompress_yay0(&super::compress_yay0(decompressed_file.as_ref())).as_ref()
        );
    }

    #[rstest]
    fn test_cycle_compressed(#[files("test_data/*.Yay0")] path: PathBuf) {
        let compressed_file = &read_test_file(path);

        assert_eq!(
            compressed_file,
            super::compress_yay0(&super::decompress_yay0(compressed_file.as_ref())).as_ref()
        );
    }
}
