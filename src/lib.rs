pub mod yay0;
pub mod yaz0;

mod utils;

use thiserror::Error;
use yay0::{compress_yay0, decompress_yay0};
use yaz0::{compress_yaz0, decompress_yaz0};

#[derive(Copy, Clone, Debug, Error, PartialEq, Eq, Hash)]
pub enum Crunch64Error {
    #[error("Failed to open file")]
    OpenFile,
    #[error("Failed to read file")]
    ReadFile,
    #[error("Failed to write file")]
    WriteFile,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CompressionType {
    Yay0,
    Yaz0,
    Mio0,
}

impl CompressionType {
    pub fn decompress(self: CompressionType, bytes: &[u8]) -> Box<[u8]> {
        match self {
            CompressionType::Yay0 => decompress_yay0(bytes),
            CompressionType::Yaz0 => decompress_yaz0(bytes),
            _ => panic!("Unsupported compression type: {:?}", self),
        }
    }

    pub fn compress(self: CompressionType, bytes: &[u8]) -> Box<[u8]> {
        match self {
            CompressionType::Yay0 => compress_yay0(bytes),
            CompressionType::Yaz0 => compress_yaz0(bytes),
            _ => panic!("Unsupported compression type: {:?}", self),
        }
    }
}
