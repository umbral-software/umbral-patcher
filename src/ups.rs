use byteorder::{LE, ReadBytesExt};
use smallvec::{SmallVec, smallvec};

use crate::{Error, INLINE_DATA_SIZE, Result};
use std::{
    fmt::Debug,
    io::{self, Read, Seek},
    iter,
};

const UPS_HEADER: &[u8] = b"UPS1";

#[allow(non_camel_case_types)]
type uvar = usize;

pub(crate) trait UpsReadExtensions {
    fn read_or_zero(&mut self, buf: &mut [u8]) -> io::Result<()>;
    fn read_uvar(&mut self) -> io::Result<uvar>;
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

    fn read_uvar(&mut self) -> io::Result<uvar> {
        let mut result = 0;
        let mut shift = 0;
        loop {
            let octet = self.read_u8()?;
            if 0 != octet & 0x80 {
                result += ((octet & 0x7F) as uvar) << shift;
                break;
            }
            result += ((octet | 0x80) as uvar) << shift;
            shift += 7;
        }
        Ok(result)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Record {
    skip: uvar,
    data: SmallVec<[u8; INLINE_DATA_SIZE]>,
}

impl Record {
    fn parse<T: io::Read>(mut ups: T) -> io::Result<Self> {
        let skip = ups.read_uvar()?;
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

    fn apply<T: io::Read, U: io::Write>(&self, mut input: T, mut output: U) -> io::Result<()> {
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
}

impl Debug for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Record")
            .field("skip", &self.skip)
            .field("size", &self.data.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Default, Debug)]
pub struct File {
    input_size: uvar,
    output_size: uvar,

    records: Vec<Record>,

    input_crc: u32,
    output_crc: u32,
}

impl File {
    pub fn parse<T: io::Read + io::Seek>(ups: T) -> Result<Self> {
        // BufReader results in a considerable performance improvement
        // Probabally because of the amount of uvar reads?
        let mut ups = io::BufReader::new(ups);

        let header = {
            let mut header = [0; UPS_HEADER.len()];
            ups.read_exact(&mut header)?;
            header
        };
        if header != UPS_HEADER {
            return Err(Error::InvalidHeader);
        }

        let input_size = ups.read_uvar()?;
        let output_size = ups.read_uvar()?;

        let record_start = ups.stream_position()?;
        ups.seek(io::SeekFrom::End(-12))?;
        let record_end = ups.stream_position()?;

        let input_crc = ups.read_u32::<LE>()?;
        let output_crc = ups.read_u32::<LE>()?;
        let patch_crc = ups.read_u32::<LE>()?;
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
            input_crc,
            output_crc,
        })
    }

    pub fn apply<T: io::Read, U: io::Write>(&self, mut input: T, mut output: U) -> io::Result<()> {
        for record in self.records.iter() {
            record.apply(&mut input, &mut output)?;
        }
        Ok(())
    }
}
