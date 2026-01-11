use std::{
    cmp, error,
    fmt::{Debug, Display},
    io, result, slice,
};

use smallvec::SmallVec;

// Pretty arbitary size choice, but improves perf a lot over a std::vec
const INLINE_DATA_SIZE: usize = 64;

#[cfg(test)]
mod tests;

const IPS_EOF: &[u8] = b"EOF";
const IPS_HEADER: &[u8] = b"PATCH";

type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    InvalidHeader,
    UnexpectedDataEOF,
    IO(io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidHeader => write!(f, "IPS header invalid"),
            Error::UnexpectedDataEOF => write!(f, "unexpected end-of-file when modifying data"),
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

#[derive(Clone, PartialEq, Eq)]
pub enum Record {
    Normal {
        offset: u32,
        data: SmallVec<[u8; INLINE_DATA_SIZE]>,
    },
    RLE {
        offset: u32,
        size: u16,
        data: u8,
    },
}

impl Debug for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal { offset, data } => f
                .debug_struct("Normal")
                .field("offset", offset)
                .field("size", &data.len())
                .finish(),
            Self::RLE { offset, size, data } => f
                .debug_struct("RLE")
                .field("offset", offset)
                .field("size", size)
                .field("data", data)
                .finish(),
        }
    }
}

pub fn apply_ips<T: io::Read>(data: &mut Vec<u8>, ips: T) -> Result<()> {
    for record in parse_ips(ips)? {
        apply_record(data, record)?;
    }

    Ok(())
}

pub fn apply_record(data: &mut Vec<u8>, record: Record) -> Result<()> {
    match record {
        Record::Normal {
            offset,
            data: new_data,
        } => {
            let end_size = offset as usize + new_data.len();
            data.resize(cmp::max(data.len(), end_size), 0);
            data.get_mut(offset as usize..end_size)
                .ok_or(Error::UnexpectedDataEOF)?
                .copy_from_slice(&new_data)
        }
        Record::RLE {
            offset,
            size,
            data: new_data,
        } => {
            let end_size = offset as usize + size as usize;
            data.resize(cmp::max(data.len(), end_size), 0);
            data.get_mut(offset as usize..end_size)
                .ok_or(Error::UnexpectedDataEOF)?
                .fill(new_data)
        }
    }
    Ok(())
}

pub fn parse_ips<T: io::Read>(mut ips: T) -> Result<impl Iterator<Item = Record>> {
    let header = {
        let mut header = [0; IPS_HEADER.len()];
        ips.read_exact(&mut header).map_err(Error::IO)?;
        header
    };
    if header != IPS_HEADER {
        return Err(Error::InvalidHeader);
    }

    let mut records = Vec::new();

    while let Some(record) = parse_ips_record(&mut ips)? {
        records.push(record);
    }

    Ok(records.into_iter())
}

fn parse_ips_record<T: io::Read>(mut ips: T) -> Result<Option<Record>> {
    let offset_bytes = {
        let mut offset_bytes = [0; 3];
        ips.read_exact(&mut offset_bytes).map_err(Error::IO)?;
        offset_bytes
    };

    if offset_bytes == IPS_EOF {
        Ok(None)
    } else {
        let offset = u32::from_be_bytes([0, offset_bytes[0], offset_bytes[1], offset_bytes[2]]);
        let size = {
            let mut size_bytes = [0; 2];
            ips.read_exact(&mut size_bytes).map_err(Error::IO)?;
            u16::from_be_bytes([size_bytes[0], size_bytes[1]])
        };
        if size > 0 {
            let data = {
                let mut data_bytes = SmallVec::from_elem(0, size as usize);
                ips.read_exact(&mut data_bytes).map_err(Error::IO)?;
                data_bytes
            };
            Ok(Some(Record::Normal { offset, data }))
        } else {
            let rle_size = {
                let mut rle_size_bytes = [0; 2];
                ips.read_exact(&mut rle_size_bytes).map_err(Error::IO)?;
                u16::from_be_bytes([rle_size_bytes[0], rle_size_bytes[1]])
            };
            let data = {
                let mut data = 0;
                ips.read_exact(slice::from_mut(&mut data))
                    .map_err(Error::IO)?;
                data
            };
            Ok(Some(Record::RLE {
                offset,
                size: rle_size,
                data,
            }))
        }
    }
}
