use std::io::{Read, Seek, SeekFrom};

use binrw::{BinRead, BinReaderExt, BinResult, Endian, VecArgs};

use super::vnum::VNum;

use super::RecordOffset;

#[derive(Debug)]
pub struct Record<B>
where
    B: BinRead,
    <B as BinRead>::Args<'static>: Default,
{
    pub meta: RecordMeta<B>,
    pub value: RecordValue,
    pub next_record: u64,
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
        let value_offset = reader.stream_position()?;
        let value_size = meta.value_size.0;
        let next_record = value_offset + value_size as u64 + meta.padding_size as u64;

        Ok(Record {
            meta,
            value: RecordValue::Offset {
                offset: value_offset,
                size: value_size,
            },
            next_record,
        })
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

#[derive(Debug)]
pub enum RecordValue {
    Offset { offset: u64, size: u32 },
    Value(Vec<u8>),
}

impl RecordValue {
    pub fn read_value<R: Read + Seek>(&self, reader: &mut R) -> Vec<u8> {
        match self {
            RecordValue::Offset { offset, size } => {
                reader.seek(SeekFrom::Start(*offset)).unwrap();
                reader
                    .read_ne_args(VecArgs {
                        count: *size as usize,
                        inner: (),
                    })
                    .unwrap()
            }
            RecordValue::Value(value) => value.clone(),
        }
    }
}
