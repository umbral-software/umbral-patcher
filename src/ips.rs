use std::{fmt::Debug, io};

use crate::{Error, Result};
use byteorder::{BE, ByteOrder, ReadBytesExt};
use smallvec::SmallVec;

const IPS_EOF: &[u8] = b"EOF";
const IPS_HEADER: &[u8] = b"PATCH";

#[derive(Clone, PartialEq, Eq)]
pub enum Record {
    Normal {
        offset: u32,
        data: SmallVec<[u8; crate::INLINE_DATA_SIZE]>,
    },
    RLE {
        offset: u32,
        size: u16,
        data: u8,
    },
}

impl Record {
    fn offset(&self) -> u32 {
        match self {
            Record::Normal { offset, .. } | Record::RLE { offset, .. } => *offset,
        }
    }

    fn len(&self) -> u16 {
        match self {
            Record::Normal { data, .. } => u16::try_from(data.len()).unwrap(),
            Record::RLE { size, .. } => *size,
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
                .finish_non_exhaustive(),
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
        apply_record(data, record);
    }

    Ok(())
}

pub fn apply_record(data: &mut Vec<u8>, record: Record) {
    let begin = record.offset() as usize;
    let len = record.len() as usize;
    let end = begin + len;
    if end > data.len() {
        data.resize(end, 0);
    }
    let slice = data.get_mut(begin..end).unwrap();

    match record {
        Record::Normal { data: new_data, .. } => {
            slice.copy_from_slice(&new_data);
        }
        Record::RLE { data: new_data, .. } => {
            slice.fill(new_data);
        }
    }
}

pub fn parse_ips<T: io::Read>(mut ips: T) -> Result<impl IntoIterator<Item = Record>> {
    let header = {
        let mut header = [0; IPS_HEADER.len()];
        ips.read_exact(&mut header).map_err(Error::IO)?;
        header
    };
    if header != IPS_HEADER {
        return Err(Error::InvalidHeader);
    }

    let mut records = Vec::new();

    while let Some(record) = parse_ips_record(&mut ips).map_err(Error::IO)? {
        records.push(record);
    }

    Ok(records)
}

fn parse_ips_record<T: io::Read>(mut ips: T) -> io::Result<Option<Record>> {
    let offset_bytes = {
        let mut offset_bytes = [0; 3];
        ips.read_exact(&mut offset_bytes)?;
        offset_bytes
    };

    if offset_bytes == IPS_EOF {
        Ok(None)
    } else {
        let offset = BE::read_u24(&offset_bytes);
        let size = ips.read_u16::<BE>()?;
        if size > 0 {
            let data = {
                let mut data_bytes = SmallVec::from_elem(0, size as usize);
                ips.read_exact(&mut data_bytes)?;
                data_bytes
            };
            Ok(Some(Record::Normal { offset, data }))
        } else {
            let size = ips.read_u16::<BE>()?;
            let data = ips.read_u8()?;
            Ok(Some(Record::RLE {
                offset,
                size,
                data,
            }))
        }
    }
}
