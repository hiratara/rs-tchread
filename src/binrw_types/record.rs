use binrw::BinRead;

use super::lazy_load::Lazy;
use super::vnum::VNum;

use super::RecordOffset;

#[derive(BinRead, Debug)]
#[br(import(alignment_power: u8), stream = r)]
pub struct Record<B>
where
    B: BinRead,
    <B as BinRead>::Args<'static>: Default,
{
    #[br(args(alignment_power))]
    pub meta: RecordMeta<B>,
    #[br(calc = r.stream_position().unwrap() + meta.value_size.0 as u64 + meta.padding_size as u64)]
    pub next_record: u64,
    #[br(args {lazy: true, inner: (meta.value_size.0,)})]
    pub value: Lazy<RecordValue, (u32,)>,
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
#[br(import(count: u32))]
pub struct RecordValue(#[br(count = count)] Vec<u8>);

impl RecordValue {
    pub fn into_value(self) -> Vec<u8> {
        self.0
    }
}
