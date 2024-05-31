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

const HASH_SIZE: usize = 1 << 15;
const HASH_MASK: usize = HASH_SIZE - 1;

const WINDOW_SIZE: usize = 0x1000;
const WINDOW_MASK: usize = WINDOW_SIZE - 1;

const MIN_MATCH: usize = 3;
const NULL: u16 = 0xFFFF;

fn update_hash(hash: usize, byte: u8) -> usize {
    ((hash << 5) ^ (byte as usize)) & HASH_MASK
}

fn longest_common_prefix(a: &[u8], b: &[u8], max_len: usize) -> usize {
    for i in 0..max_len {
        if a[i] != b[i] {
            return i;
        }
    }
    max_len
}

// Finds the longest match in a 0x1000-byte sliding window, searching
// front-to-back with a minimum match size of 3 bytes. The algorithm is similar
// to the one described in section 4 of RFC 1951
// (https://www.rfc-editor.org/rfc/rfc1951.html#section-4), using a chained hash
// table of 3-byte sequences to find matches. Each character in the window is
// identified by its position & 0xFFF (like in a circular buffer).
pub(crate) struct Window<'a> {
    // Compression input
    input: &'a [u8],
    // Current position in the input
    input_pos: usize,
    // Hash value at the window start
    hash_start: usize,
    // Hash value at the current input position
    hash_end: usize,
    // Head of hash chain for each hash value, or NULL
    head: [u16; HASH_SIZE],
    // Tail of hash chain for each hash value, or NULL
    tail: [u16; HASH_SIZE],
    // Next index in the hash chain, or NULL
    next: [u16; WINDOW_SIZE],
}

impl Window<'_> {
    pub(crate) fn new(input: &[u8]) -> Window {
        let mut hash = 0;
        for &b in input.iter().take(MIN_MATCH - 1) {
            hash = update_hash(hash, b);
        }

        Window {
            input,
            input_pos: 0,
            hash_start: hash,
            hash_end: hash,
            head: [NULL; HASH_SIZE],
            tail: [NULL; HASH_SIZE],
            next: [NULL; WINDOW_SIZE],
        }
    }

    // Advances the window by one byte, updating the hash chains.
    pub(crate) fn advance(&mut self) {
        if self.input_pos >= self.input.len() {
            return;
        }

        // Remove the oldest byte from the hash chain
        if self.input_pos >= WINDOW_SIZE {
            self.hash_start = update_hash(
                self.hash_start,
                self.input[self.input_pos - WINDOW_SIZE + MIN_MATCH - 1],
            );

            let head = self.head[self.hash_start];
            let next = self.next[head as usize];

            self.head[self.hash_start] = next;
            if next == NULL {
                self.tail[self.hash_start] = NULL;
            }
        }

        // Add the current byte to the hash chain
        if self.input_pos + MIN_MATCH < self.input.len() {
            self.hash_end = update_hash(self.hash_end, self.input[self.input_pos + MIN_MATCH - 1]);
            let tail = self.tail[self.hash_end];
            let pos = (self.input_pos & WINDOW_MASK) as u16;

            self.next[pos as usize] = NULL;
            self.tail[self.hash_end] = pos;
            if tail == NULL {
                self.head[self.hash_end] = pos;
            } else {
                self.next[tail as usize] = pos;
            }
        }

        self.input_pos += 1;
    }

    // Move the window forward the input position, and seach the window back-to-front for a match
    // at most `max_match_length` bytes long, returning the offset and length of the longest match found.
    // Successive searches can only be performed at increasing input positions.
    pub(crate) fn search(&mut self, input_pos: usize, max_match_length: usize) -> (u32, u32) {
        if input_pos < self.input_pos {
            panic!("window moved backwards");
        } else if input_pos >= self.input.len() {
            return (0, 0);
        }

        let max_match_length = cmp::min(max_match_length, self.input.len() - input_pos);
        if max_match_length < MIN_MATCH {
            return (0, 0);
        }

        while self.input_pos < input_pos {
            self.advance();
        }

        let hash = update_hash(self.hash_end, self.input[self.input_pos + MIN_MATCH - 1]);
        let mut pos = self.head[hash];
        let mut best_len = MIN_MATCH - 1;
        let mut best_offset = 0;

        while pos != NULL {
            // Figure out the current match offset from `pos` (which is equal to `match_offset & WINDOW_MASK`)
            // using the fact that `1 <= input_pos - match_offset <= WINDOW_SIZE`
            let match_offset =
                input_pos - 1 - (input_pos.wrapping_sub(pos as usize + 1) & WINDOW_MASK);

            if self.input[input_pos] == self.input[match_offset]
                && self.input[input_pos + 1] == self.input[match_offset + 1]
                && self.input[match_offset + best_len] == self.input[input_pos + best_len]
            {
                // The hash function guarantees that if the first two bytes match, the third byte will too
                let candidate_len = 3 + longest_common_prefix(
                    &self.input[input_pos + 3..],
                    &self.input[match_offset + 3..],
                    max_match_length - 3,
                );
                if candidate_len > best_len {
                    best_len = candidate_len;
                    best_offset = match_offset;
                    if best_len == max_match_length {
                        break;
                    }
                }
            }

            pos = self.next[pos as usize];
        }
        (best_offset as u32, best_len as u32)
    }
}
