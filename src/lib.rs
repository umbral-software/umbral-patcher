use std::{
    error,
    fmt::{Debug, Display},
    io, result,
};

use crc::{CRC_32_ISO_HDLC, Crc};

pub mod bps;
pub mod ips;
pub mod ups;

const INLINE_DATA_SIZE: usize = 16;
static CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

#[cfg(test)]
mod tests;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    InvalidHeader,
    InvalidInputChecksum { expected: u32, actual: u32 },
    InvalidInputSize { expected: u64, actual: u64 },
    InvalidOutputChecksum { expected: u32, actual: u32 },
    InvalidOutputSize { expected: u64, actual: u64 },
    IO(io::Error),
    VariableIntegerOverflow(&'static str),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidHeader => write!(f, "Patch header invalid"),
            Error::InvalidInputChecksum { expected, actual } => write!(
                f,
                "Input checksum invalid; expected: {expected:x}, Actual: {actual:x}"
            ),
            Error::InvalidInputSize { expected, actual } => write!(
                f,
                "Input size invalid; Expected: {expected}, Actual: {actual}"
            ),
            Error::InvalidOutputChecksum { expected, actual } => write!(
                f,
                "Output checksum invalid; expected: {expected:x}, Actual: {actual:x}"
            ),
            Error::InvalidOutputSize { expected, actual } => write!(
                f,
                "Output size invalid; Expected: {expected}, Actual: {actual}"
            ),
            Error::IO(inner) => write!(f, "I/O error \"{inner}\""),
            Error::VariableIntegerOverflow(what) => write!(
                f,
                "A variable-length integer for '{what}' could not be represented"
            ),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::IO(inner) => Some(inner),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error::IO(value)
    }
}

fn crc32<T: io::Read>(data: T) -> io::Result<u32> {
    crc32_length(data, None)
}

fn crc32_length<T: io::Read>(mut data: T, length: Option<usize>) -> io::Result<u32> {
    let mut digest = CRC32.digest();
    let mut buf = [0; 4096];
    let mut total_bytes = 0;
    loop {
        let mut bytes = data.read(&mut buf)?;
        if bytes > 0 {
            total_bytes += bytes;
            let early_done = if let Some(length) = length
                && total_bytes > length
            {
                bytes -= total_bytes - length;
                true
            } else {
                false
            };

            digest.update(&buf[..bytes]);

            if early_done {
                break;
            }
        } else {
            break;
        }
    }
    Ok(digest.finalize())
}
