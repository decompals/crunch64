pub fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    if offset % 4 != 0 {
        panic!("Unaligned offset");
    }

    u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn initskip(pattern: &[u8], len: i32, skip: &mut [u16; 256]) {
    skip.fill(len as u16);

    for i in 0..len {
        skip[pattern[i as usize] as usize] = (len - i - 1) as u16;
    }
}

pub(crate) fn mischarsearch(
    pattern: &[u8],
    pattern_len: usize,
    data: &[u8],
    data_len: usize,
) -> usize {
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
