// Implements matching zlib/DEFLATE compression for old gzip versions (before
// 2006 or so), used by some N64 and iQue games. The compressed output has a
// gzip footer (with a CRC32 checksum and the uncompressed size) but the gzip
// header is omitted. See https://github.com/Thar0/gzip-1.3.3-ique for the
// original gzip code and https://datatracker.ietf.org/doc/html/rfc1951 for
// details on the DEFLATE compression format.

use crate::{utils, Crunch64Error};

// Bitstream writer for compressed output
struct OutputStream {
    bytes: Vec<u8>,
    bit_buffer: u32,
    bit_count: u8,
}

impl OutputStream {
    fn new(capacity: usize) -> OutputStream {
        OutputStream {
            bytes: Vec::with_capacity(capacity),
            bit_buffer: 0,
            bit_count: 0,
        }
    }

    fn flush_bits(&mut self) {
        if self.bit_count > 0 {
            self.bytes.push((self.bit_buffer & 0xFF) as u8);
            self.bit_buffer = 0;
            self.bit_count = 0;
        }
    }

    fn write_bits(&mut self, value: u16, length: u8) {
        self.bit_buffer |= (value as u32) << self.bit_count;
        self.bit_count += length;
        while self.bit_count >= 8 {
            self.bytes.push((self.bit_buffer & 0xFF) as u8);
            self.bit_buffer >>= 8;
            self.bit_count -= 8;
        }
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.flush_bits();
        self.bytes.extend_from_slice(bytes);
    }

    fn into_boxed_slice(mut self) -> Box<[u8]> {
        self.flush_bits();
        self.bytes.into_boxed_slice()
    }
}

// Standard min-heap implementation as a complete binary tree in a flat array.
// We use this instead of `std::collections::BinaryHeap` to ensure consistent
// behavior when two heap elements have the same priority.
struct Heap<T> {
    data: Vec<T>,
}

impl<T: Copy + Ord> Heap<T> {
    fn new(data: Vec<T>) -> Heap<T> {
        let mut heap = Heap { data };

        let n = heap.data.len() / 2;
        for i in (0..n).rev() {
            heap.sift_down(i);
        }
        heap
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn min(&self) -> T {
        self.data[0]
    }

    fn remove_min(&mut self) -> T {
        let min = self.data[0];
        let last = self.data.pop().unwrap();
        if !self.data.is_empty() {
            self.data[0] = last;
            self.sift_down(0);
        }
        min
    }

    fn replace_min(&mut self, value: T) {
        self.data[0] = value;
        self.sift_down(0);
    }

    fn sift_down(&mut self, mut i: usize) {
        let v = self.data[i];
        let mut child = 2 * i + 1;
        while child < self.len() {
            if child + 1 < self.len() && self.data[child + 1] <= self.data[child] {
                child += 1;
            }

            if v <= self.data[child] {
                break;
            }

            self.data[i] = self.data[child];
            i = child;
            child = 2 * i + 1;
        }
        self.data[i] = v;
    }
}

// Huffman code implementation. A Huffman code maps symbols to variable-length
// code words, such that no code word is a prefix of another. The code words are
// bit-reversed in the output stream and in the lookup table.

#[derive(Clone, Copy)]
struct CodeWord {
    length: u8,
    code: u16,
}

struct HuffmanCode {
    num_symbols: usize,
    table: Vec<CodeWord>,
}

type NodeId = u16;
const NULL: u16 = 0xFFFF;

// Node in a Huffman tree
struct Node {
    symbol: u16,
    parent: NodeId,
}

// Subtree of a Huffman tree under construction
#[derive(Clone, Copy, PartialEq, Eq)]
struct Tree {
    root: NodeId,
    freq: u16,
    depth: u16,
}

impl Ord for Tree {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.freq, self.depth).cmp(&(other.freq, other.depth))
    }
}

impl PartialOrd for Tree {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl HuffmanCode {
    fn new(lengths: &[u8], length_counts: &[u16]) -> HuffmanCode {
        let num_lengths = length_counts.len();
        let empty_word = CodeWord { length: 0, code: 0 };

        // Generate the first code for each length (non-bit-reversed)
        let mut next_code = vec![0; num_lengths + 1];
        for i in 0..num_lengths {
            next_code[i + 1] = (next_code[i] + length_counts[i]) << 1;
        }

        // Generate code words
        let mut table = vec![empty_word; lengths.len()];
        let mut max_symbol = 0;
        for (i, &length) in lengths.iter().enumerate() {
            if length != 0 {
                let code = next_code[length as usize].reverse_bits() >> (16 - length);
                table[i] = CodeWord { length, code };
                next_code[length as usize] += 1;
                max_symbol = i;
            }
        }

        let num_symbols = max_symbol + 1;
        HuffmanCode { num_symbols, table }
    }

    // Construct a dynamic Huffman code from a table of symbol frequencies
    fn create_dynamic(freqs: &[u16], max_length: usize) -> HuffmanCode {
        let num_symbols = freqs.len();
        let max_num_nodes = 2 * num_symbols - 1;

        // Tree nodes
        let mut nodes: Vec<Node> = Vec::with_capacity(max_num_nodes);
        // Initial tree leaves
        let mut trees: Vec<Tree> = Vec::with_capacity(num_symbols);

        // Construct leaf nodes
        let mut max_symbol = 0;
        for (i, &freq) in freqs.iter().enumerate() {
            if freq != 0 {
                let new_node = nodes.len() as NodeId;
                nodes.push(Node {
                    symbol: i as u16,
                    parent: NULL,
                });
                trees.push(Tree {
                    root: new_node,
                    freq,
                    depth: 0,
                });
                max_symbol = i as u16;
            }
        }

        // Ensure at least two symbols
        while nodes.len() < 2 {
            let new_symbol = if max_symbol < 2 {
                max_symbol += 1;
                max_symbol
            } else {
                0
            };
            let new_node = nodes.len() as NodeId;
            nodes.push(Node {
                symbol: new_symbol,
                parent: NULL,
            });
            trees.push(Tree {
                root: new_node,
                freq: 1,
                depth: 0,
            });
        }

        // Construct tree and sort nodes by frequency
        let mut heap = Heap::new(trees);
        let mut sorted: Vec<NodeId> = Vec::with_capacity(max_num_nodes);
        while heap.len() > 1 {
            let node1 = heap.remove_min();
            let node2 = heap.min();

            sorted.push(node1.root);
            sorted.push(node2.root);

            let new_node = nodes.len() as NodeId;
            nodes[node1.root as usize].parent = new_node;
            nodes[node2.root as usize].parent = new_node;
            nodes.push(Node {
                symbol: NULL,
                parent: NULL,
            });

            let new_tree = Tree {
                root: new_node,
                freq: node1.freq + node2.freq,
                depth: 1 + std::cmp::max(node1.depth, node2.depth),
            };
            heap.replace_min(new_tree);
        }
        sorted.push(heap.min().root);

        // Assign lengths to nodes and compute the number of codes for each length
        let mut node_lengths: Vec<u8> = vec![0; max_num_nodes];
        let mut symbol_lengths: Vec<u8> = vec![0; num_symbols];
        let mut length_counts: Vec<u16> = vec![0; max_length + 1];
        let mut overflow: i32 = 0;
        for &i in sorted.iter().rev() {
            let node = &nodes[i as usize];
            let mut length = 0;
            if node.parent != NULL {
                length = 1 + node_lengths[node.parent as usize];
                if length > max_length as u8 {
                    length = max_length as u8;
                    overflow += 1;
                }
            }

            node_lengths[i as usize] = length;
            if node.symbol != NULL {
                symbol_lengths[node.symbol as usize] = length;
                length_counts[length as usize] += 1;
            }
        }

        if overflow > 0 {
            // Adjust bit lengths to avoid overflow
            for _ in 0..((overflow + 1) / 2) {
                for length in (0..max_length).rev() {
                    if length_counts[length] > 0 {
                        length_counts[length] -= 1;
                        length_counts[length + 1] += 2;
                        length_counts[max_length] -= 1;
                        break;
                    }
                }
            }

            // Regenerate lengths for each symbol in order of frequency
            let mut i = 0;
            for length in (0..=max_length).rev() {
                for _ in 0..length_counts[length] {
                    let mut symbol = nodes[sorted[i] as usize].symbol;
                    i += 1;
                    while symbol == NULL {
                        symbol = nodes[sorted[i] as usize].symbol;
                        i += 1;
                    }
                    symbol_lengths[symbol as usize] = length as u8;
                }
            }
        }

        HuffmanCode::new(&symbol_lengths, &length_counts)
    }
}

// Symbol details for the Huffman codes in the compressed data stream. There are
// 3 alphabets: bit length symbols (0-18), literal/length symbols (0-285), and
// distance symbols (0-29). Each symbol can be followed by "extra bits" in the
// compressed data stream depending on the symbol value.

// Bit length alphabet

const B_NUM_SYMBOLS: usize = 19;

#[rustfmt::skip]
const B_EXTRA_BITS: [u8; B_NUM_SYMBOLS] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0-15
    2, 3, 7, // 16-18
];

const B_SYMBOL_ORDER: [u8; B_NUM_SYMBOLS] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

// Literal/length alphabet

const L_NUM_SYMBOLS: usize = 286;

#[rustfmt::skip]
const L_EXTRA_BITS: [u8; L_NUM_SYMBOLS] = [
    // literals
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    // eof
    0,
    // lengths
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1,
    1, 1, 2, 2, 2, 2, 3, 3, 3, 3,
    4, 4, 4, 4, 5, 5, 5, 5, 0,
];

#[rustfmt::skip]
const BASE_LENGTH: [u16; L_NUM_SYMBOLS] = [
    // literals
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    // eof
    0,
    // lengths
    0, 1, 2, 3, 4, 5, 6, 7, 8, 10,
    12, 14, 16, 20, 24, 28, 32, 40, 48, 56,
    64, 80, 96, 112, 128, 160, 192, 224, 255,
];

const fn init_length_symbols() -> [u8; 256] {
    let mut lcodes = [0; 256];
    let mut code = 0;
    while code < 28 {
        let extra_bits = L_EXTRA_BITS[code + 257] as usize;
        let base = BASE_LENGTH[code + 257] as usize;
        let mut n = 0;
        while n < (1 << extra_bits) {
            lcodes[base + n] = code as u8;
            n += 1;
        }
        code += 1;
    }
    lcodes[255] = 28;
    lcodes
}

const LENGTH_SYMBOLS: [u8; 256] = init_length_symbols();

fn length_symbol(length: u8) -> usize {
    LENGTH_SYMBOLS[length as usize] as usize + 257
}

const END: usize = 256;

// Distance alphabet

const D_NUM_SYMBOLS: usize = 30;

#[rustfmt::skip]
const D_EXTRA_BITS: [u8; D_NUM_SYMBOLS] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3,
    4, 4, 5, 5, 6, 6, 7, 7, 8, 8,
    9, 9, 10, 10, 11, 11, 12, 12, 13, 13,
];

#[rustfmt::skip]
const BASE_DISTANCE: [u16; D_NUM_SYMBOLS] = [
    0, 1, 2, 3, 4, 6, 8, 12, 16, 24,
    32, 48, 64, 96, 128, 192, 256, 384, 512, 768,
    1024, 1536, 2048, 3072, 4096, 6144, 8192, 12288, 16384, 24576,
];

const fn init_short_distance_symbols() -> [u8; 256] {
    let mut dcodes = [0; 256];
    let mut code = 0;
    while code < 16 {
        let extra_bits = D_EXTRA_BITS[code] as usize;
        let base = BASE_DISTANCE[code] as usize;
        let mut n = 0;
        while n < (1 << extra_bits) {
            dcodes[base + n] = code as u8;
            n += 1;
        }
        code += 1;
    }
    dcodes
}

const fn init_long_distance_symbols() -> [u8; 256] {
    let mut dcodes = [0; 256];
    let mut code = 16;
    while code < 30 {
        let extra_bits = D_EXTRA_BITS[code] as usize;
        let base = (BASE_DISTANCE[code] as usize) / 128;
        let mut n = 0;
        while n < (1 << (extra_bits - 7)) {
            dcodes[base + n] = code as u8;
            n += 1;
        }
        code += 1;
    }
    dcodes
}

const SHORT_DISTANCE_SYMBOLS: [u8; 256] = init_short_distance_symbols();
const LONG_DISTANCE_SYMBOLS: [u8; 256] = init_long_distance_symbols();

fn distance_symbol(distance: u16) -> usize {
    if distance < 256 {
        SHORT_DISTANCE_SYMBOLS[distance as usize] as usize
    } else {
        LONG_DISTANCE_SYMBOLS[(distance as usize) / 128] as usize
    }
}

// Elements in the compressed data stream

#[derive(Clone, Copy)]
enum CodeElement {
    Length { length: u8 },
    RepeatPrev { repeat: u8 },
    RepeatZero3Bits { repeat: u8 },
    RepeatZero7Bits { repeat: u8 },
}

#[derive(Clone, Copy)]
enum DataElement {
    Literal { value: u8 },
    End,
    Match { length: u8, distance: u16 },
}

fn write_symbol(output: &mut OutputStream, code: &HuffmanCode, symbol: usize) {
    let huffman_symbol = code.table[symbol];
    output.write_bits(huffman_symbol.code, huffman_symbol.length);
}

fn write_bit_lengths(output: &mut OutputStream, elems: &[CodeElement], bcode: &HuffmanCode) {
    for elem in elems {
        match elem {
            CodeElement::Length { length } => {
                write_symbol(output, bcode, *length as usize);
            }
            CodeElement::RepeatPrev { repeat } => {
                write_symbol(output, bcode, 16);
                output.write_bits((*repeat - 3) as u16, 2);
            }
            CodeElement::RepeatZero3Bits { repeat } => {
                write_symbol(output, bcode, 17);
                output.write_bits((*repeat - 3) as u16, 3);
            }
            CodeElement::RepeatZero7Bits { repeat } => {
                write_symbol(output, bcode, 18);
                output.write_bits((*repeat - 11) as u16, 7);
            }
        }
    }
}

fn write_compressed_data(
    output: &mut OutputStream,
    elems: &[DataElement],
    lcode: &HuffmanCode,
    dcode: &HuffmanCode,
) {
    for elem in elems {
        match elem {
            DataElement::Literal { value } => {
                write_symbol(output, lcode, *value as usize);
            }
            DataElement::End => {
                write_symbol(output, lcode, END);
            }
            DataElement::Match { length, distance } => {
                let lsymbol = length_symbol(*length);
                write_symbol(output, lcode, lsymbol);
                let extra_lbits = L_EXTRA_BITS[lsymbol];
                if extra_lbits > 0 {
                    output.write_bits(*length as u16 - BASE_LENGTH[lsymbol], extra_lbits);
                }

                let dsymbol = distance_symbol(*distance);
                write_symbol(output, dcode, dsymbol);
                let extra_dbits = D_EXTRA_BITS[dsymbol];
                if extra_dbits > 0 {
                    output.write_bits(*distance - BASE_DISTANCE[dsymbol], extra_dbits);
                }
            }
        }
    }
}

fn compressed_size_bits(freqs: &[u16], code: &HuffmanCode, extra_bits: &[u8]) -> usize {
    let mut size_bits = 0;
    for i in 0..freqs.len() {
        size_bits += freqs[i] as usize * (code.table[i].length as usize + extra_bits[i] as usize);
    }
    size_bits
}

// Tables for "fixed" Huffman codes. Literal/length symbols 286-287 are never
// used and distance symbols 30-31 are never used but are still included when
// constructing the code.
const fn init_fixed_lcode_lengths() -> [u8; 288] {
    let mut fixed_lcode_lengths = [0; 288];
    let mut i = 0;
    while i < 144 {
        fixed_lcode_lengths[i] = 8;
        i += 1;
    }
    while i < 256 {
        fixed_lcode_lengths[i] = 9;
        i += 1;
    }
    while i < 280 {
        fixed_lcode_lengths[i] = 7;
        i += 1;
    }
    while i < 288 {
        fixed_lcode_lengths[i] = 8;
        i += 1;
    }
    fixed_lcode_lengths
}

const FIXED_LCODE_LENGTHS: [u8; 288] = init_fixed_lcode_lengths();
const FIXED_LCODE_LENGTH_COUNTS: [u16; 10] = [0, 0, 0, 0, 0, 0, 0, 24, 152, 112];

const FIXED_DCODE_LENGTHS: [u8; 32] = [5; 32];
const FIXED_DCODE_LENGTH_COUNTS: [u16; 5] = [0, 0, 0, 0, 32];

// Buffers compressed data into blocks
struct BlockWriter {
    // Maximum number of elements in a block
    buffer_size: usize,
    // Elements for the current block
    code_elements: Vec<CodeElement>,
    data_elements: Vec<DataElement>,
    // Number of matches in the current block
    num_matches: usize,
    // Symbol frequencies
    bfreqs: [u16; B_NUM_SYMBOLS],
    lfreqs: [u16; L_NUM_SYMBOLS],
    dfreqs: [u16; D_NUM_SYMBOLS],
    // Fixed codes
    fixed_lcode: HuffmanCode,
    fixed_dcode: HuffmanCode,
}

impl BlockWriter {
    fn new(buffer_size: usize) -> BlockWriter {
        BlockWriter {
            buffer_size,
            code_elements: Vec::with_capacity(L_NUM_SYMBOLS + D_NUM_SYMBOLS),
            data_elements: Vec::with_capacity(buffer_size),
            num_matches: 0,
            bfreqs: [0; B_NUM_SYMBOLS],
            lfreqs: [0; L_NUM_SYMBOLS],
            dfreqs: [0; D_NUM_SYMBOLS],
            fixed_lcode: HuffmanCode::new(&FIXED_LCODE_LENGTHS, &FIXED_LCODE_LENGTH_COUNTS),
            fixed_dcode: HuffmanCode::new(&FIXED_DCODE_LENGTHS, &FIXED_DCODE_LENGTH_COUNTS),
        }
    }

    fn add_literal(&mut self, value: u8) {
        self.data_elements.push(DataElement::Literal { value });
        self.lfreqs[value as usize] += 1;
    }

    fn add_match(&mut self, match_length: usize, match_distance: usize) {
        let length = (match_length - 3) as u8;
        let distance = (match_distance - 1) as u16;

        self.data_elements
            .push(DataElement::Match { length, distance });
        self.num_matches += 1;
        self.lfreqs[length_symbol(length)] += 1;
        self.dfreqs[distance_symbol(distance)] += 1;
    }

    fn add_code_elements(&mut self, length: u8, repeat: u8) {
        if repeat >= 3 {
            let (elem, symbol) = if length != 0 {
                (CodeElement::RepeatPrev { repeat }, 16)
            } else if repeat <= 10 {
                (CodeElement::RepeatZero3Bits { repeat }, 17)
            } else {
                (CodeElement::RepeatZero7Bits { repeat }, 18)
            };
            self.code_elements.push(elem);
            self.bfreqs[symbol] += 1;
        } else if repeat > 0 {
            let elem = CodeElement::Length { length };
            self.code_elements
                .resize(self.code_elements.len() + repeat as usize, elem);
            self.bfreqs[length as usize] += repeat as u16;
        }
    }

    fn generate_code_elements(&mut self, code: &HuffmanCode) {
        let mut prev = 0;
        let mut length = code.table[0].length;
        let mut repeat = 0;
        let mut max_repeat = if length == 0 { 138 } else { 6 };

        for i in 0..code.num_symbols {
            let next = if i + 1 < code.num_symbols {
                code.table[i + 1].length
            } else {
                0
            };
            repeat += 1;
            if i + 1 == code.num_symbols
                || (length != prev && length != 0)
                || length != next
                || repeat == max_repeat
            {
                self.add_code_elements(length, repeat);
                repeat = 0;
                max_repeat = if next == 0 { 138 } else { 6 };
            }
            prev = length;
            length = next;
        }
    }

    fn should_flush_block(&self, block_size: usize) -> bool {
        let num_elements = self.data_elements.len();
        let num_matches = self.num_matches;
        if num_elements == self.buffer_size - 1 {
            return true;
        }
        if num_elements % 0x1000 == 0 {
            let mut estimated_output_size_bits = num_elements * 8;
            for (i, &freq) in self.dfreqs.iter().enumerate() {
                estimated_output_size_bits += (5 + D_EXTRA_BITS[i] as usize) * freq as usize;
            }

            let estimated_output_size = estimated_output_size_bits / 8;
            if num_matches < num_elements / 2 && estimated_output_size < block_size / 2 {
                return true;
            }
        }
        false
    }

    fn flush_block(&mut self, output: &mut OutputStream, input_bytes: Option<&[u8]>, eof: bool) {
        self.data_elements.push(DataElement::End);
        self.lfreqs[END] += 1;

        // Generate Huffman codes
        let lcode = HuffmanCode::create_dynamic(&self.lfreqs, 15);
        let dcode = HuffmanCode::create_dynamic(&self.dfreqs, 15);

        self.generate_code_elements(&lcode);
        self.generate_code_elements(&dcode);
        let bcode = HuffmanCode::create_dynamic(&self.bfreqs, 7);

        let mut num_bl_indices = 4;
        for i in (4..B_NUM_SYMBOLS).rev() {
            if bcode.table[B_SYMBOL_ORDER[i] as usize].length != 0 {
                num_bl_indices = i + 1;
                break;
            }
        }

        // Compute lengths to determine best compression method
        let fixed_size_bits = 3  // block header
            + compressed_size_bits(&self.lfreqs, &self.fixed_lcode, &L_EXTRA_BITS)
            + compressed_size_bits(&self.dfreqs, &self.fixed_dcode, &D_EXTRA_BITS);
        let dynamic_size_bits = 3  // block header
            + 5 + 5 + 4  // hlit, hdist, hclen
            + 3 * num_bl_indices  // bit lengths
            + compressed_size_bits(&self.bfreqs, &bcode, &B_EXTRA_BITS)
            + compressed_size_bits(&self.lfreqs, &lcode, &L_EXTRA_BITS)
            + compressed_size_bits(&self.dfreqs, &dcode, &D_EXTRA_BITS);

        let fixed_size_bytes = fixed_size_bits.div_ceil(8);
        let dynamic_size_bytes = dynamic_size_bits.div_ceil(8);

        let uncompressed_size_bytes = if let Some(bytes) = input_bytes {
            4 + bytes.len()
        } else {
            usize::MAX
        };
        let compressed_size_bytes = std::cmp::min(fixed_size_bytes, dynamic_size_bytes);

        // Write block
        output.write_bits(eof as u16, 1);
        if uncompressed_size_bytes <= compressed_size_bytes {
            let data: &[u8] = input_bytes.unwrap();
            let len: u16 = data.len() as u16;
            let nlen: u16 = !len;
            output.write_bits(0b00, 2);
            output.write_bytes(&len.to_le_bytes());
            output.write_bytes(&nlen.to_le_bytes());
            output.write_bytes(data);
        } else if fixed_size_bytes <= dynamic_size_bytes {
            output.write_bits(0b01, 2);
            write_compressed_data(
                output,
                &self.data_elements,
                &self.fixed_lcode,
                &self.fixed_dcode,
            );
        } else {
            output.write_bits(0b10, 2);
            output.write_bits(lcode.num_symbols as u16 - 257, 5);
            output.write_bits(dcode.num_symbols as u16 - 1, 5);
            output.write_bits(num_bl_indices as u16 - 4, 4);
            for &bsymbol in B_SYMBOL_ORDER.iter().take(num_bl_indices) {
                output.write_bits(bcode.table[bsymbol as usize].length as u16, 3);
            }
            write_bit_lengths(output, &self.code_elements, &bcode);
            write_compressed_data(output, &self.data_elements, &lcode, &dcode);
        }

        // Reset for next block
        self.code_elements.clear();
        self.data_elements.clear();
        self.num_matches = 0;
        self.bfreqs.fill(0);
        self.lfreqs.fill(0);
        self.dfreqs.fill(0);
    }
}

// Compression settings

struct Config {
    good_match: usize,
    max_lazy_match: usize,
    nice_match: usize,
    max_chain_length: usize,
}

const COMPRESSION_LEVELS: [Config; 10] = [
    Config {
        good_match: 0,
        max_lazy_match: 0,
        nice_match: 0,
        max_chain_length: 0,
    },
    Config {
        good_match: 4,
        max_lazy_match: 4,
        nice_match: 8,
        max_chain_length: 4,
    },
    Config {
        good_match: 4,
        max_lazy_match: 5,
        nice_match: 16,
        max_chain_length: 8,
    },
    Config {
        good_match: 4,
        max_lazy_match: 6,
        nice_match: 32,
        max_chain_length: 32,
    },
    Config {
        good_match: 4,
        max_lazy_match: 4,
        nice_match: 16,
        max_chain_length: 16,
    },
    Config {
        good_match: 8,
        max_lazy_match: 16,
        nice_match: 32,
        max_chain_length: 32,
    },
    Config {
        good_match: 8,
        max_lazy_match: 16,
        nice_match: 128,
        max_chain_length: 128,
    },
    Config {
        good_match: 8,
        max_lazy_match: 32,
        nice_match: 128,
        max_chain_length: 256,
    },
    Config {
        good_match: 32,
        max_lazy_match: 128,
        nice_match: 258,
        max_chain_length: 1024,
    },
    Config {
        good_match: 32,
        max_lazy_match: 258,
        nice_match: 258,
        max_chain_length: 4096,
    },
];

const WINDOW_SIZE: usize = 0x8000;
const WINDOW_MASK: usize = WINDOW_SIZE - 1;

const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 258;

const MIN_LOOKAHEAD: usize = MAX_MATCH + MIN_MATCH + 1;
const MAX_DIST: usize = WINDOW_SIZE - MIN_LOOKAHEAD;
const TOO_FAR: usize = 4096;

// Sentinel value for hash chains. Using 0 means we can't have any matches at
// the start of the window.
const NIL: usize = 0;

fn update_hash(hash: usize, c: u8, mask: usize) -> usize {
    ((hash << 5) ^ c as usize) & mask
}

fn size_for_compressed_buffer(input_size: usize) -> Result<usize, Crunch64Error> {
    // Upper bound based on fixed-Huffman-code blocks consisting of only 9-byte
    // literals (stored blocks might be shorter but might fall out of the window
    // before we can emit them). The minimum block size is 0x1000 bytes (if
    // `should_flush_block` decides to end a block early) and each block
    // requires a 3-bit header.
    let upper_bound_bits = 3 * input_size.div_ceil(0x1000)// block headers
        + 9 * input_size // literals
        + 8 * 8; // footer
    Ok(upper_bound_bits.div_ceil(8))
}

pub fn compress(bytes: &[u8], level: usize, small_mem: bool) -> Result<Box<[u8]>, Crunch64Error> {
    let input_size = bytes.len();

    // Levels 0-3 use a slightly different compression algorithm which is not
    // implemented here
    if !(4..=9).contains(&level) {
        return Err(Crunch64Error::InvalidCompressionLevel);
    }

    let config = &COMPRESSION_LEVELS[level];
    let buffer_size = if small_mem { 0x2000 } else { 0x8000 };
    let hash_bits = if small_mem { 13 } else { 15 };
    let hash_size = 1 << hash_bits;
    let hash_mask = hash_size - 1;

    let mut output = OutputStream::new(size_for_compressed_buffer(bytes.len())?);
    let mut writer = BlockWriter::new(buffer_size);
    let mut hasher = crc32fast::Hasher::new();

    // Old gzip versions can read past the window into memory used for other
    // global variables, which can affect compression output. We allocate a
    // little extra space and reproduce the original memory layout to match.
    let mut window: Vec<u8> = vec![0; 2 * WINDOW_SIZE + MAX_MATCH];

    const ORIGINAL_GZIP_GARBAGE: &[u8] = &[
        0x00, 0x00, 0x00, 0x00, // inptr (0)
        0x03, 0x00, 0x00, 0x00, // ifd (3)
        0xB5, 0x2F, 0x05, 0x08, // z_suffix (0x08052FB5)
        0x00, 0x00, 0x00, 0x00, // bk (0)
        0x00, 0x00, 0x00, 0x00, // bb (0)
        0x52, 0xD0, 0xFF, 0xFF, // file_type (0xFFFFD052)
        0xD0, 0x4A, 0x05, 0x08, // file_method (0x08054AD0)
        0x00, 0x00, 0x00, 0x00, // decrypt (0)
        0x00, 0x00, 0x00, 0x00, // key (0)
        0x0A, 0x00, 0x00, 0x00, // header_bytes (10)
              // Remaining bytes are 0
    ];
    window[2 * WINDOW_SIZE..2 * WINDOW_SIZE + ORIGINAL_GZIP_GARBAGE.len()]
        .copy_from_slice(ORIGINAL_GZIP_GARBAGE);

    // Position in input buffer
    let mut input_pos = std::cmp::min(2 * WINDOW_SIZE, input_size);
    // True if we have reached the end of input
    let mut eof: bool = input_size < 2 * WINDOW_SIZE;
    // Length of current block
    let mut block_length: usize = 0;

    // Copy start of input into window
    window[0..input_pos].copy_from_slice(&bytes[0..input_pos]);
    hasher.update(&bytes[0..input_pos]);

    // Current position in window
    let mut pos: usize = 0;
    // Number of bytes left in window
    let mut lookahead = input_pos;

    // Heads of hash chains
    let mut head: Vec<usize> = vec![NIL; hash_size];
    // Next pointers in hash chains
    let mut next: Vec<usize> = vec![NIL; WINDOW_SIZE];

    // Current hash value
    let mut hash: usize = 0;
    hash = update_hash(hash, window[0], hash_mask);
    hash = update_hash(hash, window[1], hash_mask);

    // True if we haven't emitted the previous character yet (either as a
    // literal or a match)
    let mut has_prev_char: bool = false;
    // Best match length for previous character
    let mut prev_match_len: usize = MIN_MATCH - 1;
    // Best match distance for previous character
    let mut prev_match_dist: usize = 0;

    while lookahead > 0 {
        // Insert new string into the hash table
        hash = update_hash(hash, window[pos + MIN_MATCH - 1], hash_mask);
        next[pos & WINDOW_MASK] = head[hash];
        head[hash] = pos;

        // Find the longest match
        let mut match_pos = next[pos & WINDOW_MASK];
        let mut best_pos = 0;
        let mut best_len = prev_match_len;
        if match_pos != NIL && prev_match_len < config.max_lazy_match && pos - match_pos <= MAX_DIST
        {
            // Bound for number of potential matches to check. If the previous
            // match is "good" we don't check as many.
            let mut chain_length = if prev_match_len >= config.good_match {
                config.max_chain_length / 4
            } else {
                config.max_chain_length
            };

            // Earliest position to check for matches
            let limit = if pos > MAX_DIST { pos - MAX_DIST } else { NIL };

            loop {
                if window[match_pos] == window[pos]
                    && window[match_pos + 1] == window[pos + 1]
                    && window[match_pos + best_len] == window[pos + best_len]
                {
                    // The hash function guarantees that if the hashes are equal and
                    // the first two bytes match, the third byte will too
                    let candidate_length = 3 + utils::longest_common_prefix(
                        &window[match_pos + 3..match_pos + MAX_MATCH],
                        &window[pos + 3..pos + MAX_MATCH],
                    );
                    if candidate_length > best_len {
                        best_pos = match_pos;
                        best_len = candidate_length;
                        if best_len >= config.nice_match {
                            break;
                        }
                    }
                }

                match_pos = next[match_pos & WINDOW_MASK];
                chain_length -= 1;
                if match_pos <= limit || chain_length == 0 {
                    break;
                }
            }

            best_len = std::cmp::min(best_len, lookahead);
            if best_len == MIN_MATCH && pos - best_pos > TOO_FAR {
                best_len = MIN_MATCH - 1;
            }
        }

        let mut should_flush = false;
        if prev_match_len >= MIN_MATCH && prev_match_len >= best_len {
            // Emit previous match
            writer.add_match(prev_match_len, prev_match_dist);
            should_flush = writer.should_flush_block(block_length);

            // Insert new strings in the hash table
            for i in 1..prev_match_len - 1 {
                hash = update_hash(hash, window[pos + i + MIN_MATCH - 1], hash_mask);
                next[(pos + i) & WINDOW_MASK] = head[hash];
                head[hash] = pos + i;
            }

            block_length += prev_match_len - 1;
            pos += prev_match_len - 1;
            lookahead -= prev_match_len - 1;

            if should_flush {
                if pos >= block_length {
                    writer.flush_block(&mut output, Some(&window[pos - block_length..pos]), false);
                } else {
                    writer.flush_block(&mut output, None, false);
                }
                block_length = 0;
            }

            has_prev_char = false;
            prev_match_len = MIN_MATCH - 1;
            prev_match_dist = 0;
        } else {
            // Emit previous character as literal (if it exists) and remember current match
            if has_prev_char {
                writer.add_literal(window[pos - 1]);
                should_flush = writer.should_flush_block(block_length);
            }

            if should_flush {
                if pos >= block_length {
                    writer.flush_block(&mut output, Some(&window[pos - block_length..pos]), false);
                } else {
                    writer.flush_block(&mut output, None, false);
                }
                block_length = 0;
            }

            block_length += 1;
            pos += 1;
            lookahead -= 1;

            has_prev_char = true;
            prev_match_len = best_len;
            prev_match_dist = pos - 1 - best_pos;
        }

        // Refill window
        if lookahead < MIN_LOOKAHEAD && !eof && pos >= WINDOW_SIZE + MAX_DIST {
            window.copy_within(WINDOW_SIZE..2 * WINDOW_SIZE, 0);

            pos -= WINDOW_SIZE;
            for i in &mut head {
                *i = if *i >= WINDOW_SIZE {
                    *i - WINDOW_SIZE
                } else {
                    NIL
                };
            }
            for i in &mut next {
                *i = if *i >= WINDOW_SIZE {
                    *i - WINDOW_SIZE
                } else {
                    NIL
                };
            }

            let refill_start = input_pos;
            let refill_end = std::cmp::min(refill_start + WINDOW_SIZE, input_size);
            let refill_size = refill_end - refill_start;
            window[WINDOW_SIZE..WINDOW_SIZE + refill_size]
                .copy_from_slice(&bytes[refill_start..refill_end]);
            hasher.update(&bytes[refill_start..refill_end]);

            input_pos = refill_end;
            lookahead += refill_size;
            if refill_size == 0 {
                eof = true;
            }
        }
    }

    if has_prev_char {
        writer.add_literal(window[pos - 1]);
    }

    if pos >= block_length {
        writer.flush_block(&mut output, Some(&window[pos - block_length..pos]), true);
    } else {
        writer.flush_block(&mut output, None, true);
    }

    output.write_bytes(&hasher.finalize().to_le_bytes());
    output.write_bytes(&(input_size as u32).to_le_bytes());

    Ok(output.into_boxed_slice())
}

#[cfg(feature = "c_bindings")]
mod c_bindings {
    use std::ffi::c_int;

    #[no_mangle]
    pub extern "C" fn crunch64_gzip_compress_bound(
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
    pub extern "C" fn crunch64_gzip_compress(
        dst_len: *mut usize,
        dst: *mut u8,
        src_len: usize,
        src: *const u8,
        level: c_int,
        small_mem: bool,
    ) -> super::Crunch64Error {
        if dst_len.is_null() || dst.is_null() || src.is_null() {
            return super::Crunch64Error::NullPointer;
        }

        let bytes = match super::utils::u8_vec_from_pointer_array(src_len, src) {
            Err(e) => return e,
            Ok(d) => d,
        };

        let data = match super::compress(&bytes, level as usize, small_mem) {
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
    #[pyo3(signature = (bytes, level=9, small_mem=false))]
    pub(crate) fn compress_gzip(
        bytes: Cow<[u8]>,
        level: usize,
        small_mem: bool,
    ) -> Result<Cow<[u8]>, super::Crunch64Error> {
        Ok(Cow::Owned(
            super::compress(&bytes, level, small_mem)?.into(),
        ))
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
    fn test_matching_compression_level_9(
        #[files("../test_data/*.gzip-9")] path: PathBuf,
    ) -> Result<(), Crunch64Error> {
        let compressed_file = &read_test_file(path.clone());
        let decompressed_file = &read_test_file(path.with_extension(""));

        let compressed = super::compress(decompressed_file.as_slice(), 9, false)?;
        assert_eq!(compressed_file, compressed.as_ref());
        Ok(())
    }

    #[rstest]
    fn test_matching_compression_level_9_small_mem(
        #[files("../test_data/*.gzip-9-small-mem")] path: PathBuf,
    ) -> Result<(), Crunch64Error> {
        let compressed_file = &read_test_file(path.clone());
        let decompressed_file = &read_test_file(path.with_extension(""));

        let compressed = super::compress(decompressed_file.as_slice(), 9, true)?;
        assert_eq!(compressed_file, compressed.as_ref());
        Ok(())
    }

    #[rstest]
    fn test_matching_compression_level_6_small_mem(
        #[files("../test_data/*.gzip-6-small-mem")] path: PathBuf,
    ) -> Result<(), Crunch64Error> {
        let compressed_file = &read_test_file(path.clone());
        let decompressed_file = &read_test_file(path.with_extension(""));

        let compressed = super::compress(decompressed_file.as_slice(), 6, true)?;
        assert_eq!(compressed_file, compressed.as_ref());
        Ok(())
    }
}
