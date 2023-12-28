use std::cmp;

use crate::Crunch64Error;

pub fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, Crunch64Error> {
    if offset % 2 != 0 {
        return Err(Crunch64Error::UnalignedRead);
    }

    if offset + 2 >= bytes.len() {
        return Err(Crunch64Error::OutOfBounds);
    }

    match bytes[offset..offset + 2].try_into() {
        Ok(bytes) => Ok(u16::from_be_bytes(bytes)),
        Err(_error) => Err(Crunch64Error::ByteConversion),
    }
}

pub fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, Crunch64Error> {
    if offset % 4 != 0 {
        return Err(Crunch64Error::UnalignedRead);
    }

    if offset + 4 > bytes.len() {
        return Err(Crunch64Error::OutOfBounds);
    }

    match bytes[offset..offset + 4].try_into() {
        Ok(bytes) => Ok(u32::from_be_bytes(bytes)),
        Err(_error) => Err(Crunch64Error::ByteConversion),
    }
}

#[cfg(feature = "c_bindings")]
pub(crate) fn u8_vec_from_pointer_array(
    src_len: usize,
    src: *const u8,
) -> Result<Vec<u8>, Crunch64Error> {
    if src.is_null() {
        return Err(Crunch64Error::NullPointer);
    }

    let mut bytes = Vec::with_capacity(src_len);

    for i in 0..src_len {
        bytes.push(unsafe { *src.add(i) });
    }

    Ok(bytes)
}

#[cfg(feature = "c_bindings")]
pub(crate) fn set_pointer_array_from_u8_array(
    dst_len: *mut usize,
    dst: *mut u8,
    src: &[u8],
) -> Result<(), Crunch64Error> {
    if dst_len.is_null() || dst.is_null() {
        return Err(Crunch64Error::NullPointer);
    }

    // `dst_len` is expected to point to the size of the `dst` pointer,
    // we use this to check if the data will fit in `dst`
    if src.len() > unsafe { *dst_len } {
        return Err(Crunch64Error::OutOfBounds);
    }

    for (i, b) in src.iter().enumerate() {
        unsafe {
            *dst.add(i) = *b;
        }
    }
    unsafe {
        *dst_len = src.len();
    }

    Ok(())
}

pub(crate) fn search(input_pos: usize, data_in: &[u8], max_match_length: usize) -> (i32, u32) {
    let mut cur_size = 3;
    let mut found_pos = 0;
    let mut search_pos = cmp::max(input_pos as isize - 0x1000, 0) as usize;
    let search_size = cmp::min(data_in.len() - input_pos, max_match_length);

    if search_size < 3 {
        return (0, 0);
    }

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
            return ((found_offset + search_pos) as i32, cur_size as u32);
        }

        found_pos = (search_pos + found_offset) as isize;
        search_pos = (found_pos + 1) as usize;
        cur_size += 1;
    }

    (found_pos as i32, cmp::max(cur_size as isize - 1, 0) as u32)
}

fn mischarsearch(pattern: &[u8], pattern_len: usize, data: &[u8], data_len: usize) -> usize {
    let mut skip_table = [0u16; 256];
    let mut i: isize;

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
