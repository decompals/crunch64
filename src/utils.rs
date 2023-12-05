use std::cmp;

pub fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    if offset % 4 != 0 {
        panic!("Unaligned offset");
    }

    u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

pub(crate) fn search(
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
