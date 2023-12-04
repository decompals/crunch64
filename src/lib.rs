pub mod yay0;

use thiserror::Error;
use yay0::{compress_yay0, decompress_yay0};

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
}

impl CompressionType {
    pub fn decompress(self: CompressionType, bytes: &[u8]) -> Box<[u8]> {
        match self {
            CompressionType::Yay0 => decompress_yay0(bytes),
            _ => panic!("Unsupported compression type: {:?}", self),
        }
    }

    pub fn compress(self: CompressionType, bytes: &[u8]) -> Box<[u8]> {
        match self {
            CompressionType::Yay0 => compress_yay0(bytes),
            _ => panic!("Unsupported compression type: {:?}", self),
        }
    }
}
