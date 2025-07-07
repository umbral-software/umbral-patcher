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
    NotAnIPSFile,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotAnIPSFile => write!(f, "input was not a valid IPS file"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

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

fn slice_data(data: &mut [u8], offset: u32, size: u32) -> &mut [u8] {
    &mut data[(offset as usize)..(offset as usize + size as usize)]
}

pub fn apply_record(data: &mut [u8], record: Record) {
    println!("{record:#x?}");
    match record {
        Record::Normal {
            offset,
            data: new_data,
        } => slice_data(data, offset, new_data.len() as u32).copy_from_slice(&new_data),
        Record::RLE {
            offset,
            size,
            data: new_data,
        } => slice_data(data, offset, size as u32).fill(new_data),
    }
}

pub fn apply_ips(data: &mut [u8], ips: &[u8]) -> Result<()> {
    for record in parse_ips(ips)? {
        apply_record(data, record);
    }

    Ok(())
}

fn parse_ips_record(ips: &[u8]) -> Option<Record> {
    let offset_bytes = &ips[..3];
    if offset_bytes == IPS_EOF {
        None
    } else {
        let offset = u32::from_be_bytes([0, offset_bytes[0], offset_bytes[1], offset_bytes[2]]);
        let size_bytes = &ips[3..5];
        let size = u16::from_be_bytes([size_bytes[0], size_bytes[1]]);
        if size > 0 {
            let data_bytes = &ips[5..(5 + size as usize)];
            Some(Record::Normal {
                offset,
                data: Vec::from(data_bytes),
            })
        } else {
            let rle_size_bytes = &ips[5..7];
            let rle_size = u16::from_be_bytes([rle_size_bytes[0], rle_size_bytes[1]]);
            let data = ips[7];
            Some(Record::RLE {
                offset,
                size: rle_size,
                data,
            })
        }
    }
}

pub fn parse_ips(ips: &[u8]) -> Result<impl IntoIterator<Item = Record>> {
    if &ips[..IPS_HEADER.len()] != IPS_HEADER {
        return Err(Error::NotAnIPSFile);
    }

    let mut records = Vec::new();
    let mut offset = IPS_HEADER.len();

    while let Some(record) = parse_ips_record(&ips[offset..]) {
        offset += record.len();
        records.push(record);
    }

    Ok(records)
}
