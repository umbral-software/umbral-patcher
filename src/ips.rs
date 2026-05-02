use std::{fmt::Debug, io, num::NonZero};

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
        size: NonZero<u16>,
        data: u8,
    },
}

#[allow(clippy::len_without_is_empty)] // The concept of 'empty' doesn't exist for a single record
impl Record {
    pub(crate) fn parse<T: io::Read>(mut ips: T) -> Result<Option<Record>> {
        let offset = ips.read_u24::<BE>()?;

        if offset == BE::read_u24(IPS_EOF) {
            Ok(None)
        } else {
            let size = ips.read_u16::<BE>()?;
            if size > 0 {
                let data = {
                    let mut data_bytes = SmallVec::from_elem(0, size as usize);
                    ips.read_exact(&mut data_bytes)?;
                    data_bytes
                };
                Ok(Some(Record::Normal { offset, data }))
            } else {
                let size = {
                    let size = ips.read_u16::<BE>()?;
                    NonZero::new(size).ok_or(Error::ZeroSizedHunk)?
                };
                let data = ips.read_u8()?;
                Ok(Some(Record::RLE { offset, size, data }))
            }
        }
    }

    pub fn apply(&self, data: &mut Vec<u8>) {
        let begin = self.offset() as usize;
        let len = self.len() as usize;
        let end = begin + len;
        if end > data.len() {
            data.resize(end, 0);
        }
        let slice = data.get_mut(begin..end).unwrap();

        match self {
            Record::Normal { data: new_data, .. } => {
                slice.copy_from_slice(new_data);
            }
            Record::RLE { data: new_data, .. } => {
                slice.fill(*new_data);
            }
        }
    }

    pub fn len(&self) -> u16 {
        match self {
            Record::Normal { data, .. } => u16::try_from(data.len()).unwrap(),
            Record::RLE { size, .. } => (*size).into(),
        }
    }

    pub fn offset(&self) -> u32 {
        match self {
            Record::Normal { offset, .. } | Record::RLE { offset, .. } => *offset,
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

#[derive(Clone, Default, Debug)]
pub struct File {
    pub(crate) records: Vec<Record>,
}

impl File {
    pub fn parse<T: io::Read>(mut ips: T) -> Result<Self> {
        let header = {
            let mut header = [0; IPS_HEADER.len()];
            ips.read_exact(&mut header)?;
            header
        };
        if header != IPS_HEADER {
            return Err(Error::InvalidHeader);
        }

        let mut records = Vec::new();
        while let Some(record) = Record::parse(&mut ips)? {
            records.push(record);
        }

        Ok(Self { records })
    }

    pub fn apply<T: io::Read, U: io::Write>(&self, mut input: T, mut output: U) -> io::Result<()> {
        let mut data = Vec::new();
        input.read_to_end(&mut data)?;

        for record in &self.records {
            record.apply(&mut data);
        }

        output.write_all(&data)?;

        Ok(())
    }
}
