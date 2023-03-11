use std::ops::Shl;

use binrw::BinRead;

use super::vnum::VNum;

#[derive(BinRead, Debug)]
pub struct Header {
    #[br(count = 32, assert(magic_number.starts_with(b"ToKyO CaBiNeT")))]
    pub magic_number: Vec<u8>,
    #[br(assert(database_type == 0))]
    pub database_type: u8,
    pub additional_flags: u8,
    pub alignment_power: u8,
    pub free_block_pool_power: u8,
    #[br(pad_after = 3)]
    pub options: u8,
    pub bucket_number: u64,
    pub record_number: u64,
    pub file_size: u64,
    #[br(pad_after = 56)]
    pub first_record: u64,
    #[br(count = 128)]
    pub opaque_region: Vec<u8>,
}

#[derive(BinRead, Clone, Copy, Debug)]
#[br(import(alignment_power: u8))]
pub struct RecordOffset<B>
where
    B: BinRead,
    <B as BinRead>::Args<'static>: Default,
{
    value: B,
    #[br(calc = alignment_power)]
    alignment_power: u8,
}

impl<B> RecordOffset<B>
where
    B: BinRead + Copy + Shl<u8, Output = B> + Into<u64>,
    <B as BinRead>::Args<'static>: Default,
{
    pub fn offset(&self) -> u64 {
        self.value.into() << self.alignment_power
    }

    pub fn is_empty(&self) -> bool {
        self.value.into() <= 0
    }
}

#[derive(BinRead, Debug)]
#[br(import(alignment_power: u8, bucket_number: u64))]
pub struct Buckets<B>(
    #[br(count = bucket_number, args{inner: (alignment_power, )})] pub Vec<RecordOffset<B>>,
)
where
    B: 'static + BinRead,
    <B as BinRead>::Args<'static>: Default;

#[derive(BinRead, Debug)]
pub struct FreeBlockPoolElement {
    pub offset: VNum<u32>, // TODO: recorded as the difference of the former free block and as the quotient by the alignment
    pub size: VNum<u32>,
}

#[derive(BinRead, Debug)]
#[br(import(alignment_power: u8))]
pub struct Record<B>
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
    #[br(count = value_size.0)]
    pub value: Vec<u8>,
    #[br(count = padding_size)]
    pub padding: Vec<u8>,
}

#[derive(BinRead, Debug)]
pub struct FreeBlock {
    pub block_size: u32,
    #[br(count = block_size - 5)]
    pub padding: Vec<u8>,
}

#[derive(BinRead, Debug)]
#[br(import(alignment_power: u8))]
pub enum RecordSpace<B>
where
    B: BinRead,
    <B as BinRead>::Args<'static>: Default,
{
    #[br(magic = 0xc8u8)]
    Record(#[br(args(alignment_power))] Record<B>),
    #[br(magic = 0xb0u8)]
    FreeBlock(FreeBlock),
}
