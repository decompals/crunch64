#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate alloc;

pub mod gzip;
pub mod mio0;
pub mod yay0;
pub mod yaz0;

mod utils;

use thiserror::Error;

#[cfg(feature = "python_bindings")]
use pyo3::exceptions::PyRuntimeError;
#[cfg(feature = "python_bindings")]
use pyo3::prelude::*;

/* This needs to be in sync with the C equivalent at `crunch64/error.h` */
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
    #[error("Invalid compression level")]
    InvalidCompressionLevel,
}

#[cfg(feature = "python_bindings")]
impl std::convert::From<Crunch64Error> for PyErr {
    fn from(err: Crunch64Error) -> PyErr {
        PyRuntimeError::new_err(err.to_string())
    }
}

#[cfg(feature = "python_bindings")]
#[pymodule]
fn crunch64(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(yay0::python_bindings::decompress_yay0, m)?)?;
    m.add_function(wrap_pyfunction!(yay0::python_bindings::compress_yay0, m)?)?;
    m.add_function(wrap_pyfunction!(yaz0::python_bindings::decompress_yaz0, m)?)?;
    m.add_function(wrap_pyfunction!(yaz0::python_bindings::compress_yaz0, m)?)?;
    m.add_function(wrap_pyfunction!(mio0::python_bindings::decompress_mio0, m)?)?;
    m.add_function(wrap_pyfunction!(mio0::python_bindings::compress_mio0, m)?)?;
    m.add_function(wrap_pyfunction!(gzip::python_bindings::compress_gzip, m)?)?;
    Ok(())
}
