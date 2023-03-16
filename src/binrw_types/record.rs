use std::mem;

use binrw::BinRead;

use super::lazy_load::Lazy;
use super::vnum::VNum;

use super::RecordOffset;

#[derive(BinRead, Debug)]
#[br(import(offset: u64, alignment_power: u8))]
pub struct Record<B>
where
    B: BinRead,
    <B as BinRead>::Args<'static>: Default,
{
    #[br(calc = offset)]
    pub offset: u64,
    #[br(args(alignment_power))]
    pub meta: RecordMeta<B>,
    #[br(args {lazy: true, inner: (meta.value_size.value,)})]
    pub value: Lazy<RecordValue, (u32,)>,
    #[br(calc = offset + meta.size() + meta.value_size.value as u64 + meta.padding_size as u64)]
    pub next_record: u64,
}

#[derive(BinRead, Debug)]
#[br(import(alignment_power: u8))]
pub struct RecordMeta<B>
where
    B: BinRead,
    <B as BinRead>::Args<'static>: Default,
{
    pub hash_value: u8,
    #[br(args(alignment_power))]
    pub left_chain: RecordOffset<B>,
    #[br(args(alignment_power))]
    pub right_chain: RecordOffset<B>,
    pub padding_size: u16,
    pub key_size: VNum<u32>,
    pub value_size: VNum<u32>,
    #[br(count = key_size.value)]
    pub key: Vec<u8>,
}

impl<B> RecordMeta<B>
where
    B: BinRead,
    <B as BinRead>::Args<'static>: Default,
{
    fn size(&self) -> u64 {
        1 + mem::size_of::<B>() as u64 * 2
            + 2
            + self.key_size.size as u64
            + self.value_size.size as u64
            + self.key_size.value as u64
    }
}

#[derive(BinRead, Debug)]
#[br(import(count: u32))]
pub struct RecordValue(#[br(count = count)] Vec<u8>);

impl RecordValue {
    pub fn into_value(self) -> Vec<u8> {
        self.0
    }
}
