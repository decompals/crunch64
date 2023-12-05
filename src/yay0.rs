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

fn search(a1: usize, insize: usize, a3: &mut i32, a4: &mut u32, bz: &[u8]) {
    let mut patternlen: usize = 3;
    let mut v5: usize = 0;
    let mut v8: isize = 0;

    if a1 > 0x1000 {
        v5 = a1 - 0x1000;
    }

    let mut v9 = 273;

    if insize - a1 <= 273 {
        v9 = insize - a1;
    }

    if v9 > 2 {
        while v5 < a1 {
            let v7 = mischarsearch(&bz[a1..], patternlen, &bz[v5..], patternlen + a1 - v5);

            if v7 >= a1 - v5 {
                break;
            }

            while patternlen < v9 {
                if bz[patternlen + v5 + v7] != bz[patternlen + a1] {
                    break;
                }
                patternlen += 1;
            }

            if v9 == patternlen {
                *a3 = (v7 + v5) as i32;
                *a4 = patternlen as u32;
                return;
            }

            v8 = (v5 + v7) as isize;
            patternlen += 1;
            v5 += v7 + 1;
        }

        *a3 = v8 as i32;
        if patternlen > 3 {
            patternlen -= 1;
            *a4 = patternlen as u32;
            return;
        }
    } else {
        *a3 = 0;
    }
    *a4 = 0;
}

fn mischarsearch(pattern: &[u8], pattern_len: usize, data: &[u8], data_len: usize) -> usize {
    let mut skip_table = [0u16; 256];
    let mut i: isize;
    let mut k: usize;

    let mut v6: isize;
    let mut j: isize;

    if pattern_len <= data_len {
        // initskip
        k = 0;
        while k < skip_table.len() {
            skip_table[k] = pattern_len as u16;
            k += 1;
        }

        k = 0;
        while k < pattern_len {
            skip_table[pattern[k] as usize] = (pattern_len - k - 1) as u16;
            k += 1;
        }

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

                if skip_table[data[i as usize] as usize] <= (pattern_len as isize - j) as u16 {
                    v6 = pattern_len as isize - j;
                    i += v6;
                    continue;
                }
            }
            v6 = skip_table[data[i as usize] as usize] as isize;
            i += v6;
        }
    }
    data_len
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_matching_decompression() {
        let compressed_file = include_bytes!("../test_data/Yay0/1.Yay0");
        let decompressed_file = include_bytes!("../test_data/Yay0/1.bin");

        let decompressed: Box<[u8]> = super::decompress_yay0(compressed_file);
        assert_eq!(decompressed_file, decompressed.as_ref());
    }

    #[test]
    fn test_matching_compression() {
        let compressed_file = include_bytes!("../test_data/Yay0/1.Yay0");
        let decompressed_file = include_bytes!("../test_data/Yay0/1.bin");

        let compressed = super::compress_yay0(decompressed_file.as_slice());
        assert_eq!(compressed_file, compressed.as_ref());
    }

    #[test]
    fn test_cycle() {
        let decompressed_file = include_bytes!("../test_data/Yay0/1.bin");

        assert_eq!(
            decompressed_file,
            super::decompress_yay0(&super::compress_yay0(decompressed_file.as_ref())).as_ref()
        );
    }
}
