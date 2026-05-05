use byteorder::{LE, ReadBytesExt};
use smallvec::{SmallVec, smallvec};

use crate::{Error, INLINE_DATA_SIZE, Result, UvarReadExtensions, crc32, crc32_length};
use std::{borrow::Cow, fmt::Debug, io, num::NonZero};

const BPS_HEADER: &[u8] = b"BPS1";

#[allow(non_camel_case_types)]
type ivar = i128;

pub(crate) trait BpsReadExtensions {
    fn read_ivar(&mut self) -> io::Result<ivar>;
}

impl<T: UvarReadExtensions> BpsReadExtensions for T {
    fn read_ivar(&mut self) -> io::Result<ivar> {
        let raw = self.read_uvar()?;
        let bit = raw & 0x1;
        let magnitude = (raw >> 1).cast_signed();
        if bit != 0 {
            Ok(-magnitude)
        } else {
            Ok(magnitude)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Record {
    SourceRead(NonZero<usize>),
    TargetRead(SmallVec<[u8; INLINE_DATA_SIZE]>),
    SourceCopy { length: NonZero<usize>, offset: i64 },
    TargetCopy { length: NonZero<usize>, offset: i64 },
}

impl Record {
    fn parse<T: io::Read>(mut bps: T) -> Result<Self> {
        let raw = bps.read_uvar()?;
        let action = (raw & 0x3) as u8;
        let length = NonZero::new(((raw >> 2) as usize) + 1).unwrap();
        let result = match action {
            0 => Self::SourceRead(length),
            1 => {
                let mut data = smallvec![0; length.into()];
                bps.read_exact(&mut data)?;
                Self::TargetRead(data)
            }
            2 => Self::SourceCopy {
                length,
                offset: i64::try_from(bps.read_ivar()?)
                    .map_err(|_| Error::VariableIntegerOverflow("record offset"))?,
            },
            3 => Self::TargetCopy {
                length,
                offset: i64::try_from(bps.read_ivar()?)
                    .map_err(|_| Error::VariableIntegerOverflow("record offset"))?,
            },
            _ => unreachable!(),
        };
        Ok(result)
    }

    fn apply<T: io::Read + io::Seek, U: io::Read + io::Write + io::Seek>(
        &self,
        mut source: T,
        mut target: U,
    ) -> io::Result<()> {
        let data = match *self {
            Record::SourceRead(length) => {
                let old_source_offset = source.stream_position()?;
                let old_target_offset = target.stream_position()?;

                let eof = target.seek(io::SeekFrom::End(0))?;
                source.seek(io::SeekFrom::Start(eof))?;

                let mut buf: SmallVec<[_; INLINE_DATA_SIZE]> = smallvec![0; length.into()];
                source.read_exact(&mut buf)?;

                source.seek(io::SeekFrom::Start(old_source_offset))?;
                target.seek(io::SeekFrom::Start(old_target_offset))?;

                Cow::Owned(buf)
            }
            Record::TargetRead(ref data) => Cow::Borrowed(data),
            Record::SourceCopy { length, offset } => {
                source.seek_relative(offset)?;

                let mut buf: SmallVec<[_; INLINE_DATA_SIZE]> = smallvec![0; length.into()];
                source.read_exact(&mut buf)?;
                Cow::Owned(buf)
            }
            Record::TargetCopy { length, offset } => {
                let start_pos = target.seek(io::SeekFrom::Current(offset))?;
                let eof = target.seek(io::SeekFrom::End(0))?;
                target.seek(io::SeekFrom::Start(start_pos))?;

                let mut buf: SmallVec<[_; INLINE_DATA_SIZE]> =
                    SmallVec::with_capacity(length.into());
                for i in 0..length.into() {
                    let read_pos = start_pos + i as u64;
                    if read_pos >= eof {
                        buf.push(
                            buf[usize::try_from(read_pos - eof).expect("RLE TargetCopy overflow")],
                        );
                        target.seek_relative(1)?;
                    } else {
                        buf.push(target.read_u8()?);
                    }
                }
                Cow::Owned(buf)
            }
        };

        let target_pos = target.stream_position()?;
        target.seek(io::SeekFrom::End(0))?;
        target.write_all(&data)?;
        target.seek(io::SeekFrom::Start(target_pos))?;

        Ok(())
    }
}

#[derive(Clone, Default, Debug)]
pub struct File {
    source_size: u64,
    target_size: u64,

    metadata: String,
    records: Vec<Record>,

    source_checksum: u32,
    target_checksum: u32,
}

impl File {
    pub fn parse<T: io::Read + io::Seek>(mut bps: T) -> Result<Self> {
        let header = {
            let mut header = [0; BPS_HEADER.len()];
            bps.read_exact(&mut header)?;
            header
        };
        if header != BPS_HEADER {
            return Err(Error::InvalidHeader);
        }

        let source_size = bps
            .read_uvar()?
            .try_into()
            .map_err(|_| Error::VariableIntegerOverflow("input filesize"))?;
        let target_size = bps
            .read_uvar()?
            .try_into()
            .map_err(|_| Error::VariableIntegerOverflow("output filesize"))?;
        let metadata_size = bps
            .read_uvar()?
            .try_into()
            .map_err(|_| Error::VariableIntegerOverflow("metadata size"))?;
        let metadata_start = bps.stream_position()?;

        bps.seek(io::SeekFrom::End(-12))?;
        let record_end = bps.stream_position()?;
        let source_checksum = bps.read_u32::<LE>()?;
        let target_checksum = bps.read_u32::<LE>()?;
        let checksum_end = bps.stream_position()?;
        let patch_checksum = bps.read_u32::<LE>()?;

        bps.seek(io::SeekFrom::Start(0))?;
        let actual_checksum = crc32_length(&mut bps, Some(checksum_end))?;
        if patch_checksum != actual_checksum {
            return Err(Error::InvalidInputChecksum {
                expected: patch_checksum,
                actual: actual_checksum,
            });
        }

        bps.seek(io::SeekFrom::Start(metadata_start))?;
        let metadata = if metadata_size > 0 {
            let mut buf = vec![0; metadata_size];
            bps.read_exact(&mut buf)?;
            String::from_utf8(buf).map_err(Error::InvalidMetadata)?
        } else {
            String::new()
        };

        let mut records = Vec::new();
        while bps.stream_position()? < record_end {
            let record = Record::parse(&mut bps)?;
            records.push(record);
        }

        Ok(Self {
            source_size,
            target_size,
            metadata,
            records,
            source_checksum,
            target_checksum,
        })
    }

    pub fn apply<T: io::Read + io::Seek, U: io::Read + io::Write + io::Seek>(
        &self,
        mut source: T,
        mut target: U,
    ) -> Result<()> {
        source.seek(io::SeekFrom::End(0))?;
        let source_size = source.stream_position()?;
        if self.source_size != source_size {
            return Err(Error::InvalidInputSize {
                expected: self.source_size,
                actual: source_size,
            });
        }

        source.seek(io::SeekFrom::Start(0))?;
        let source_checksum = crc32(&mut source)?;
        if self.source_checksum != source_checksum {
            return Err(Error::InvalidInputChecksum {
                expected: self.source_checksum,
                actual: source_checksum,
            });
        }

        source.seek(io::SeekFrom::Start(0))?;
        for record in &self.records {
            record.apply(&mut source, &mut target)?;
        }

        target.seek(io::SeekFrom::End(0))?;
        let target_size = target.stream_position()?;
        if self.target_size != target_size {
            return Err(Error::InvalidOutputSize {
                expected: self.target_size,
                actual: target_size,
            });
        }

        target.seek(io::SeekFrom::Start(0))?;
        let target_checksum = crc32(&mut target)?;
        if self.target_checksum != target_checksum {
            return Err(Error::InvalidOutputChecksum {
                expected: self.target_checksum,
                actual: target_checksum,
            });
        }

        Ok(())
    }

    #[must_use]
    pub fn metadata(&self) -> &str {
        &self.metadata
    }
}
