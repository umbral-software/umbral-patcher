use std::{fmt::Debug, io, vec};
use crate::Result;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Record {

}

impl Record {
    fn parse<T: io::Read>(mut bps: T) -> Self {
        todo!()
    }

    fn apply(&self, data: &mut Vec<u8>) {
        todo!()
    }
}

#[derive(Clone, Default, Debug)]
pub struct File {
    records: Vec<Record>,
}

impl File {
    pub fn parse<T: io::Read>(mut bps: T) -> Result<Self> {
        todo!()
    }

    pub fn apply(&self, data: &mut Vec<u8>) {
        for record in &self.records {
            record.apply(data);
        }
    }
}

impl IntoIterator for File {
    type Item = Record;

    type IntoIter = vec::IntoIter<Record>;

    fn into_iter(self) -> Self::IntoIter {
        self.records.into_iter()
    }
}
