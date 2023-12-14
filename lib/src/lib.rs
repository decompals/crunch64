pub mod yay0;
pub mod yaz0;

mod utils;

use thiserror::Error;
use yay0::{compress_yay0, decompress_yay0};
use yaz0::{compress_yaz0, decompress_yaz0};

#[cfg(feature = "python_bindings")]
use pyo3::exceptions::PyRuntimeError;
#[cfg(feature = "python_bindings")]
use pyo3::prelude::*;

/* This needs to be in sync with the C equivalent at `crunch64_error.h` */
#[cfg_attr(feature = "c_bindings", repr(u32))]
#[derive(Copy, Clone, Debug, Error, PartialEq, Eq, Hash)]
pub enum Crunch64Error {
    #[error("Not an error")]
    Okay,
    #[error("File does not begin with Yay0 header")]
    InvalidYay0Header,
    #[error("File does not begin with Yaz0 header")]
    InvalidYaz0Header,
    #[error("File does not begin with Mio0 header")]
    InvalidMio0Header,
    #[error("Unsupported compression type")]
    UnsupportedCompressionType,
    #[error("Unaligned read")]
    UnalignedRead,
    #[error("Failed to convert bytes")]
    ByteConversion,
    #[error("Tried to access data out of bounds")]
    OutOfBounds,
    #[error("Pointer is null")]
    NullPointer,
}

#[cfg(feature = "python_bindings")]
impl std::convert::From<Crunch64Error> for PyErr {
    fn from(err: Crunch64Error) -> PyErr {
        PyRuntimeError::new_err(err.to_string())
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CompressionType {
    Yay0,
    Yaz0,
    Mio0,
}

impl CompressionType {
    pub fn decompress(self: CompressionType, bytes: &[u8]) -> Result<Box<[u8]>, Crunch64Error> {
        match self {
            CompressionType::Yay0 => decompress_yay0(bytes),
            CompressionType::Yaz0 => decompress_yaz0(bytes),
            _ => Err(Crunch64Error::UnsupportedCompressionType),
        }
    }

    pub fn compress(self: CompressionType, bytes: &[u8]) -> Result<Box<[u8]>, Crunch64Error> {
        match self {
            CompressionType::Yay0 => compress_yay0(bytes),
            CompressionType::Yaz0 => compress_yaz0(bytes),
            _ => Err(Crunch64Error::UnsupportedCompressionType),
        }
    }
}

#[cfg(feature = "python_bindings")]
/// A Python module implemented in Rust. The name of this function must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
fn crunch64(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(yaz0::python_bindings::decompress_yaz0, m)?)?;
    m.add_function(wrap_pyfunction!(yaz0::python_bindings::compress_yaz0, m)?)?;
    m.add_function(wrap_pyfunction!(yay0::python_bindings::decompress_yay0, m)?)?;
    m.add_function(wrap_pyfunction!(yay0::python_bindings::compress_yay0, m)?)?;
    Ok(())
}
