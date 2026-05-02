use crate::Result;
use std::{fmt::Debug, io};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Record {}

impl Record {
    fn parse<T: io::Read>(mut bps: T) -> Self {
        todo!()
    }

    fn apply<T: io::Read + io::Seek, U: io::Write + io::Seek>(
        &self,
        mut input: T,
        mut output: U,
    ) -> io::Result<()> {
        todo!()
    }
}

#[derive(Clone, Default, Debug)]
pub struct File {
    records: Vec<Record>,
}

impl File {
    pub fn parse<T: io::Read + io::Seek>(mut bps: T) -> Result<Self> {
        todo!()
    }

    pub fn apply<T: io::Read + io::Seek, U: io::Write + io::Seek>(
        &self,
        mut input: T,
        mut output: U,
    ) -> io::Result<()> {
        for record in self.records.iter() {
            record.apply(&mut input, &mut output)?;
        }
        Ok(())
    }
}
