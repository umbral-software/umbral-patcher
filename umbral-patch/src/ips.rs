use std::{fmt::Debug, io, num::NonZero};

use crate::{Error, Result};
use byteorder::{BE, ByteOrder, ReadBytesExt, WriteBytesExt};
use smallvec::SmallVec;

const IPS_EOF: &[u8] = b"EOF";
const IPS_HEADER: &[u8] = b"PATCH";

/// An IPS record
#[derive(Clone, PartialEq, Eq)]
pub enum Record {
    /// Record contains new data
    Normal {
        /// The offset the record should be written at
        offset: u32,
        /// The data that should be written
        data: SmallVec<[u8; crate::INLINE_DATA_SIZE]>,
    },
    /// Record contains new data in a Run-Length Encoded format
    RLE {
        /// The offset the record should be written at
        offset: u32,
        /// The amount of data that should be written
        size: NonZero<u16>,
        /// The value that should be written
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

    /// Applies a single record
    /// # Errors
    /// `IO` if any `io::Error` is generated from `output`
    pub fn apply<T: io::Write + io::Seek>(&self, mut output: T) -> io::Result<()> {
        output.seek(io::SeekFrom::Start(self.offset().into()))?;

        match *self {
            Record::Normal { ref data, .. } => {
                output.write_all(data)?;
            }
            Record::RLE { data: new_data, size, ..  } => {
                for _ in 0..size.into() {
                    output.write_u8(new_data)?;
                }
            }
        }
        Ok(())
    }

    /// The size of this record
    #[must_use]
    pub fn len(&self) -> u16 {
        match self {
            #[allow(clippy::cast_possible_truncation)]
            Record::Normal { data, .. } => data.len() as u16,
            Record::RLE { size, .. } => (*size).into(),
        }
    }

    /// The offset this record should be applied at
    #[must_use]
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

/// A parsed IPS file
#[derive(Clone, Default, Debug)]
pub struct File {
    pub(crate) records: Vec<Record>,
}

impl File {
    /// Parse an IPS file
    /// # Errors
    /// `InvalidHeader` if the patch header is invalid
    /// `ZeroSizedHunk` if a zero-length record is parsed
    /// `IO` if any `io::Error` is generated from accessing `ips`
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

    /// Apply the contained IPS records to an input file and generate a patched file
    /// # Errors
    /// `IO` if any `io::Error` is generated from accessing `input` or `output`
    /// Any error returned by `Record::apply`
    pub fn apply<T: io::Read, U: io::Write + io::Seek>(&self, mut input: T, mut output: U) -> io::Result<()> {
        io::copy(&mut input, &mut output)?;
        output.seek(io::SeekFrom::Start(0))?;

        for record in &self.records {
            record.apply(&mut output)?;
        }

        Ok(())
    }

    /// Inspect the records contained in this IPS file
    pub fn records(&self) -> impl Iterator<Item = &Record> {
        self.records.iter()
    }
}
