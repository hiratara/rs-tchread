mod lazy_load;
mod record;
mod vnum;

use std::fmt::Debug;

use binrw::BinRead;
use num_traits::int::PrimInt;

pub use self::record::Record;

use vnum::VNum;

/// u32 or u64 value
pub trait U32orU64: BinRead<Args<'static> = ()> + Debug + PrimInt + 'static {}

impl<T> U32orU64 for T where T: BinRead<Args<'static> = ()> + Debug + PrimInt + 'static {}

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

pub struct RecordOffset<U: U32orU64> {
    value: U,
}

impl<U: U32orU64> RecordOffset<U> {
    #[inline]
    pub fn offset(&self, alignment_power: u8) -> u64 {
        self.value.to_u64().unwrap() << alignment_power
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.value <= U::zero()
    }
}

#[derive(BinRead, Debug)]
#[br(import(bucket_number: u64))]
pub struct Buckets<U: U32orU64>(#[br(count = bucket_number)] pub Vec<RecordOffset<U>>);

#[derive(BinRead, Debug)]
pub struct FreeBlockPoolElement {
    pub offset: VNum<u32>, // TODO: recorded as the difference of the former free block and as the quotient by the alignment
    pub size: VNum<u32>,
}

#[derive(BinRead, Debug)]
pub struct FreeBlock {
    pub block_size: u32,
    #[br(count = block_size - 5)]
    pub padding: Vec<u8>,
}

#[derive(BinRead, Debug)]
#[br(import(offset: u64, read_value: bool))]
pub enum RecordSpace<U: U32orU64> {
    #[br(magic = 0xc8u8)]
    // add the length of magic to the offset
    Record(#[br(args(offset + 1, read_value))] Record<U>),
    #[br(magic = 0xb0u8)]
    FreeBlock(FreeBlock),
}
