use std::io::{Read, Seek};

use binrw::{BinRead, BinResult, Endian};

use crate::tchdb::vnum::VNum;

use super::RecordOffset;

#[derive(Debug)]
pub struct Record<B>
where
    B: BinRead,
    <B as BinRead>::Args<'static>: Default,
{
    pub meta: RecordMeta<B>,
    pub value: RecordValue,
}

impl<B> BinRead for Record<B>
where
    B: BinRead,
    <B as BinRead>::Args<'static>: Default,
{
    type Args<'a> = (u8,);

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: Endian,
        args: Self::Args<'_>,
    ) -> BinResult<Self> {
        let meta = <RecordMeta<B>>::read_options(reader, endian, args)?;
        let value =
            <RecordValue>::read_options(reader, endian, (meta.value_size.0, meta.padding_size))?;

        Ok(Record { meta, value })
    }
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
