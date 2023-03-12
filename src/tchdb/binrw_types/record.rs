use binrw::BinRead;

use crate::tchdb::vnum::VNum;

use super::RecordOffset;

#[derive(BinRead, Debug)]
#[br(import(alignment_power: u8))]
pub struct Record<B>
where
    B: BinRead,
    <B as BinRead>::Args<'static>: Default,
{
    #[br(args(alignment_power))]
    pub meta: RecordMeta<B>,
    #[br(args(meta.value_size.0, meta.padding_size))]
    pub value: RecordValue,
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
    #[br(count = key_size.0)]
    pub key: Vec<u8>,
}

#[derive(BinRead, Debug)]
#[br(import(value_size: u32, padding_size: u16))]
pub struct RecordValue {
    #[br(count = value_size)]
    pub value: Vec<u8>,
    #[br(count = padding_size)]
    pub padding: Vec<u8>,
}
