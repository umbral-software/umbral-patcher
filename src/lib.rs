use std::{
    error,
    fmt::{Debug, Display},
    result,
};

const IPS_EOF: &[u8] = b"EOF";
const IPS_HEADER: &[u8] = b"PATCH";

type Result<T> = result::Result<T, Error>;

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Error {
    InvalidHeader,
    UnexpectedDataEOF,
    UnexpectedIPSEOF,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidHeader => write!(f, "IPS header invalid"),
            Error::UnexpectedDataEOF => write!(f, "unexpected end-of-file when modifying data"),
            Error::UnexpectedIPSEOF => write!(f, "unexpected end-of-file while parsing IPS"),
        }
    }
}

impl error::Error for Error {}

#[derive(Clone)]
pub enum Record {
    Normal { offset: u32, data: Vec<u8> },
    RLE { offset: u32, size: u16, data: u8 },
}

impl Record {
    fn len(&self) -> usize {
        match self {
            Record::Normal { data, .. } => 5 + data.len(),
            Record::RLE { .. } => 8,
        }
    }
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

fn slice_data(data: &mut [u8], offset: u32, size: u32) -> Result<&mut [u8]> {
    data.get_mut((offset as usize)..(offset as usize + size as usize))
        .ok_or(Error::UnexpectedDataEOF)
}

pub fn apply_record(data: &mut [u8], record: Record) -> Result<()> {
    match record {
        Record::Normal {
            offset,
            data: new_data,
        } => slice_data(data, offset, new_data.len() as u32)?.copy_from_slice(&new_data),
        Record::RLE {
            offset,
            size,
            data: new_data,
        } => slice_data(data, offset, size as u32)?.fill(new_data),
    }
    Ok(())
}

pub fn apply_ips(data: &mut [u8], ips: &[u8]) -> Result<()> {
    for record in parse_ips(ips)? {
        apply_record(data, record)?;
    }

    Ok(())
}

fn parse_ips_record(ips: &[u8]) -> Result<Option<Record>> {
    let offset_bytes = ips.get(..3).ok_or(Error::UnexpectedIPSEOF)?;
    if offset_bytes == IPS_EOF {
        Ok(None)
    } else {
        let offset = u32::from_be_bytes([0, offset_bytes[0], offset_bytes[1], offset_bytes[2]]);
        let size_bytes = ips.get(3..5).ok_or(Error::UnexpectedIPSEOF)?;
        let size = u16::from_be_bytes([size_bytes[0], size_bytes[1]]);
        if size > 0 {
            let data_bytes = ips
                .get(5..(5 + size as usize))
                .ok_or(Error::UnexpectedIPSEOF)?;
            Ok(Some(Record::Normal {
                offset,
                data: Vec::from(data_bytes),
            }))
        } else {
            let rle_size_bytes = ips.get(5..7).ok_or(Error::UnexpectedIPSEOF)?;
            let rle_size = u16::from_be_bytes([rle_size_bytes[0], rle_size_bytes[1]]);
            let data = *ips.get(7).ok_or(Error::UnexpectedIPSEOF)?;
            Ok(Some(Record::RLE {
                offset,
                size: rle_size,
                data,
            }))
        }
    }
}

pub fn parse_ips(ips: &[u8]) -> Result<impl IntoIterator<Item = Record>> {
    let header = ips.get(..IPS_HEADER.len()).ok_or(Error::UnexpectedIPSEOF)?;
    if header != IPS_HEADER {
        return Err(Error::InvalidHeader);
    }

    let mut records = Vec::new();
    let mut offset = IPS_HEADER.len();

    while let Some(record) = parse_ips_record(ips.get(offset..).ok_or(Error::UnexpectedIPSEOF)?)? {
        offset += record.len();
        records.push(record);
    }

    Ok(records)
}
