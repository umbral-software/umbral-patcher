use byteorder::{LE, ReadBytesExt};
use smallvec::{SmallVec, smallvec};

use crate::{Error, INLINE_DATA_SIZE, Result, UvarReadExtensions, crc32, crc32_length};
use std::{fmt::Debug, io, iter};

const UPS_HEADER: &[u8] = b"UPS1";

pub(crate) trait UpsReadExtensions {
    fn read_or_zero(&mut self, buf: &mut [u8]) -> io::Result<()>;
}

impl<T: io::Read> UpsReadExtensions for T {
    fn read_or_zero(&mut self, mut buf: &mut [u8]) -> io::Result<()> {
        loop {
            match self.read(buf) {
                Ok(0) => {
                    buf.fill(0);
                    return Ok(());
                }
                Ok(bytes) => {
                    buf = &mut buf[bytes..];
                    if buf.is_empty() {
                        return Ok(());
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
    }
}

/// A UPS record
#[derive(Clone, PartialEq, Eq)]
pub struct Record {
    skip: usize,
    data: SmallVec<[u8; INLINE_DATA_SIZE]>,
}

#[allow(clippy::len_without_is_empty)] // The concept of 'empty' doesn't exist for a single record
impl Record {
    fn parse<T: io::Read>(mut ups: T) -> Result<Self> {
        let skip = ups
            .read_uvar()?
            .try_into()
            .map_err(|_| Error::VariableIntegerOverflow("Record 'skip' size"))?;
        let mut data = SmallVec::new();
        loop {
            let value = ups.read_u8()?;
            data.push(value);
            if value == 0 {
                break;
            }
        }

        Ok(Self { skip, data })
    }

    /// Applies a single record
    pub fn apply<T: io::Read, U: io::Write>(&self, mut input: T, mut output: U) -> io::Result<()> {
        if self.skip > 0 {
            let mut buf: SmallVec<[_; INLINE_DATA_SIZE]> = smallvec![0; self.skip];
            input.read_or_zero(&mut buf)?;
            output.write_all(&buf)?;
        }

        let mut buf: SmallVec<[_; INLINE_DATA_SIZE]> = smallvec![0; self.data.len()];
        input.read_or_zero(&mut buf)?;
        for (i, p) in iter::zip(buf.iter_mut(), self.data.iter()) {
            *i ^= p;
        }
        output.write_all(&buf)?;

        Ok(())
    }

    /// The size of this record's skip value
    #[must_use]
    pub fn skip_len(&self) -> usize {
        self.skip
    }

    /// The size of this record's data payload
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl Debug for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Record")
            .field("skip", &self.skip)
            .field("size", &self.data.len())
            .finish_non_exhaustive()
    }
}

/// A parsed UPS file
#[derive(Clone, Default, Debug)]
pub struct File {
    input_size: u64,
    output_size: u64,

    records: Vec<Record>,

    input_checksum: u32,
    output_checksum: u32,
}

impl File {
    /// Parse a UPS file
    pub fn parse<T: io::Read + io::Seek>(mut ups: T) -> Result<Self> {
        let header = {
            let mut header = [0; UPS_HEADER.len()];
            ups.read_exact(&mut header)?;
            header
        };
        if header != UPS_HEADER {
            return Err(Error::InvalidHeader);
        }

        let input_size = ups
            .read_uvar()?
            .try_into()
            .map_err(|_| Error::VariableIntegerOverflow("input filesize"))?;
        let output_size = ups
            .read_uvar()?
            .try_into()
            .map_err(|_| Error::VariableIntegerOverflow("output filesize"))?;
        let record_start = ups.stream_position()?;

        ups.seek(io::SeekFrom::End(-12))?;
        let record_end = ups.stream_position()?;
        let input_checksum = ups.read_u32::<LE>()?;
        let output_checksum = ups.read_u32::<LE>()?;
        let checksum_end = ups.stream_position()?;
        let patch_checksum = ups.read_u32::<LE>()?;

        ups.seek(io::SeekFrom::Start(0))?;
        let actual_checksum = crc32_length(&mut ups, Some(checksum_end))?;

        if patch_checksum != actual_checksum {
            return Err(Error::InvalidInputChecksum {
                expected: patch_checksum,
                actual: actual_checksum,
            });
        }

        ups.seek(io::SeekFrom::Start(record_start))?;
        let mut records = Vec::new();
        while ups.stream_position()? < record_end {
            let record = Record::parse(&mut ups)?;
            records.push(record);
        }
        if let Some(record) = records.last_mut() {
            record.data.pop();
        }

        Ok(Self {
            input_size,
            output_size,
            records,
            input_checksum,
            output_checksum,
        })
    }

    /// Apply the contained UPS records to an input file and generate a patched file
    pub fn apply<T: io::Read + io::Seek, U: io::Read + io::Write + io::Seek>(
        &self,
        mut input: T,
        mut output: U,
    ) -> Result<()> {
        input.seek(io::SeekFrom::End(0))?;
        let input_size = input.stream_position()?;
        if self.input_size != input_size {
            return Err(Error::InvalidInputSize {
                expected: self.input_size,
                actual: input_size,
            });
        }

        input.seek(io::SeekFrom::Start(0))?;
        let input_checksum = crc32(&mut input)?;
        if self.input_checksum != input_checksum {
            return Err(Error::InvalidInputChecksum {
                expected: self.input_checksum,
                actual: input_checksum,
            });
        }

        input.seek(io::SeekFrom::Start(0))?;
        for record in &self.records {
            record.apply(&mut input, &mut output)?;
        }

        let output_size = output.stream_position()?;
        if self.output_size != output_size {
            return Err(Error::InvalidOutputSize {
                expected: self.output_size,
                actual: output_size,
            });
        }

        output.seek(io::SeekFrom::Start(0))?;
        let output_checksum = crc32(&mut output)?;
        if self.output_checksum != output_checksum {
            return Err(Error::InvalidOutputChecksum {
                expected: self.output_checksum,
                actual: output_checksum,
            });
        }

        Ok(())
    }

    /// Inspect the records contained in this UPS file
    pub fn records(&self) -> impl Iterator<Item = &Record> {
        self.records.iter()
    }
}
