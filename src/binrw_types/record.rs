use std::mem;

use binrw::BinRead;

use super::lazy_load::Lazy;
use super::vnum::VNum;

use super::{RecordOffset, U32orU64};

#[derive(BinRead, Debug)]
#[br(import(offset: u64, read_value: bool))]
pub struct Record<U: U32orU64> {
    #[br(calc = offset)]
    pub offset: u64,
    pub hash_value: u8,
    pub left_chain: RecordOffset<U>,
    pub right_chain: RecordOffset<U>,
    pub padding_size: u16,
    pub key_size: VNum<u32>,
    pub value_size: VNum<u32>,
    #[br(count = key_size.0)]
    pub key: Vec<u8>,
    #[br(args {lazy: !read_value, inner: (value_size.0,)})]
    pub value: Lazy<RecordValue, (u32,)>,
}

impl<U: U32orU64> Record<U> {
    #[inline]
    pub fn next_record(&self) -> u64 {
        self.offset
            + 1
            + mem::size_of::<U>() as u64 * 2
            + 2
            + self.key_size.size() as u64
            + self.value_size.size() as u64
            + self.key_size.0 as u64
            + self.value_size.0 as u64
            + self.padding_size as u64
    }
}

#[derive(BinRead, Debug)]
#[br(import(count: u32))]
pub struct RecordValue(#[br(count = count)] Vec<u8>);

impl RecordValue {
    #[inline]
    pub fn into_value(self) -> Vec<u8> {
        self.0
    }
}
