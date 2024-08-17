use crate::Crunch64Error;

pub fn compress(bytes: &[u8]) -> Result<Box<[u8]>, Crunch64Error> {
    match vpk0::encode_bytes(bytes) {
        Ok(bytes) => Ok(bytes.into_boxed_slice()),
        Err(e) => Err(Crunch64Error::Vpk0(e.to_string())),
    }
}

pub fn decompress(bytes: &[u8]) -> Result<Box<[u8]>, Crunch64Error> {
    match vpk0::decode_bytes(bytes) {
        Ok(bytes) => Ok(bytes.into_boxed_slice()),
        Err(e) => Err(Crunch64Error::Vpk0(e.to_string())),
    }
}