use std::{
    error,
    fmt::{Debug, Display},
    io, result,
};

pub mod ips;

// Pretty arbitary size choice, but improves perf a lot over a std::vec
const INLINE_DATA_SIZE: usize = 64;

#[cfg(test)]
mod tests;

type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    InvalidHeader,
    IO(io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidHeader => write!(f, "Patch header invalid"),
            Error::IO(inner) => write!(f, "I/O error \"{inner}\""),
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
