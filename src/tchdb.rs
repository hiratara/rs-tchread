use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    marker::PhantomData,
    mem,
    ops::Shl,
    path::Path,
};

use binread::{BinRead, BinReaderExt};

use crate::vnum::VNum;

#[derive(BinRead, Debug)]
#[br(little)]
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

#[derive(BinRead, Debug)]
#[br(import(alignment_power: u8))]
pub struct RecordOffset<B>
where
    B: BinRead<Args = ()>,
{
    value: B,
    #[br(calc = alignment_power)]
    alignment_power: u8,
}

impl<B> RecordOffset<B>
where
    B: BinRead<Args = ()> + Copy + Shl<u8, Output = B> + Into<u64>,
{
    pub fn offset(&self) -> u64 {
        (self.value << self.alignment_power).into()
    }
}

#[derive(BinRead, Debug)]
#[br(import(alignment_power: u8, bucket_number: u64))]
pub struct Buckets<B>(#[br(count = bucket_number, args(alignment_power))] pub Vec<RecordOffset<B>>)
where
    B: BinRead<Args = ()>;

#[derive(BinRead, Debug)]
pub struct FreeBlockPoolElement {
    pub offset: VNum<u32>, // TODO: recorded as the difference of the former free block and as the quotient by the alignment
    pub size: VNum<u32>,
}

#[derive(Debug)]
pub struct KeyWithHash<'a> {
    key: &'a [u8],
    idx: u64,
    hash: u8,
}

#[derive(BinRead, Debug)]
#[br(little)]
#[br(import(alignment_power: u8))]
pub struct Record<B>
where
    B: BinRead<Args = ()>,
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
#[br(little)]
pub struct FreeBlock {
    pub block_size: u32,
    #[br(count = block_size - 5)]
    pub padding: Vec<u8>,
}

#[derive(BinRead, Debug)]
#[br(little)]
#[br(import(alignment_power: u8))]
pub enum RecordSpace<B>
where
    B: BinRead<Args = ()>,
{
    #[br(magic = 0xc8u8)]
    Record(#[br(args(alignment_power))] Record<B>),
    #[br(magic = 0xb0u8)]
    FreeBlock(FreeBlock),
}

pub struct TCHDBImpl<B, R> {
    pub reader: R,
    pub header: Header,
    pub bucket_offset: u64, // always be 256
    pub free_block_pool_offset: u64,
    _bucket_type: PhantomData<B>,
}

impl<B, R> TCHDBImpl<B, R> {
    pub fn hash<'a>(&self, key: &'a [u8]) -> KeyWithHash<'a> {
        let mut idx: u64 = 19780211;
        for &b in key {
            idx = idx.wrapping_mul(37).wrapping_add(b as u64);
        }
        idx %= self.header.bucket_number;

        let mut hash: u32 = 751;
        for &b in key.into_iter().rev() {
            hash = hash.wrapping_mul(31) ^ b as u32;
        }

        KeyWithHash {
            key,
            idx,
            hash: hash as u8,
        }
    }
}

impl<B, R> TCHDBImpl<B, R>
where
    R: Read + Seek,
{
    fn new(mut reader: R, header: Header) -> Self {
        let bucket_offset = reader.stream_position().unwrap();
        debug_assert_eq!(bucket_offset, 256);

        let free_block_pool_offset =
            bucket_offset + header.bucket_number * mem::size_of::<B>() as u64;

        TCHDBImpl {
            reader,
            header,
            bucket_offset,
            free_block_pool_offset,
            _bucket_type: PhantomData,
        }
    }

    pub fn read_free_block_pool(&mut self) -> Vec<FreeBlockPoolElement> {
        self.reader
            .seek(SeekFrom::Start(self.free_block_pool_offset))
            .unwrap();

        let pool_size = 2usize.pow(self.header.free_block_pool_power as u32);
        let mut pool = Vec::with_capacity(pool_size);
        loop {
            let elem: FreeBlockPoolElement = self.reader.read_ne().unwrap();
            if elem.offset.0 == 0 && elem.size.0 == 0 {
                break;
            }
            pool.push(elem);
        }
        pool
    }
}

impl<B, R> TCHDBImpl<B, R>
where
    B: BinRead<Args = ()> + Copy + Shl<u8, Output = B> + Into<u64>,
    R: Read + Seek,
{
    pub fn read_buckets(&mut self) -> Buckets<B> {
        self.reader
            .seek(SeekFrom::Start(self.bucket_offset))
            .unwrap();
        let buckets = self
            .reader
            .read_ne_args((self.header.alignment_power, self.header.bucket_number))
            .unwrap();

        debug_assert_eq!(
            self.reader.stream_position().unwrap(),
            self.free_block_pool_offset
        );

        buckets
    }

    pub fn read_record_spaces(&mut self) -> Vec<RecordSpace<B>> {
        self.reader
            .seek(SeekFrom::Start(self.header.first_record))
            .unwrap();

        let mut records = Vec::with_capacity(self.header.record_number as usize);

        loop {
            let record: RecordSpace<B> = self
                .reader
                .read_ne_args((self.header.alignment_power,))
                .unwrap();
            records.push(record);

            let pos = self.reader.stream_position().unwrap();
            if pos >= self.header.file_size {
                break;
            }
        }

        records
    }

    fn read_bucket(&mut self, idx: u64) -> RecordOffset<B> {
        let pos = self.bucket_offset + mem::size_of::<B>() as u64 * idx;
        self.reader.seek(SeekFrom::Start(pos)).unwrap();
        self.reader
            .read_ne_args((self.header.alignment_power,))
            .unwrap()
    }

    fn read_record(&mut self, rec_off: RecordOffset<B>) -> RecordSpace<B> {
        self.reader
            .seek(SeekFrom::Start(rec_off.offset().into()))
            .unwrap();
        self.reader
            .read_ne_args((self.header.alignment_power,))
            .unwrap()
    }

    pub fn get(&mut self, key: &KeyWithHash) -> RecordSpace<B> {
        let rec_off = self.read_bucket(key.idx);
        self.read_record(rec_off)
    }
}

pub enum TCHDB<R> {
    Small(TCHDBImpl<u32, R>),
    Large(TCHDBImpl<u64, R>),
}

impl TCHDB<File> {
    pub fn open<T>(path: T) -> Self
    where
        T: AsRef<Path>,
    {
        let file = File::open(path).unwrap();
        TCHDB::new(file)
    }
}

impl<R> TCHDB<R>
where
    R: Read + Seek,
{
    pub fn new(mut reader: R) -> Self {
        reader.seek(SeekFrom::Start(0)).unwrap();
        let header: Header = reader.read_ne().unwrap();

        if header.options & 0x01 == 0x01 {
            TCHDB::Large(TCHDBImpl::new(reader, header))
        } else {
            TCHDB::Small(TCHDBImpl::new(reader, header))
        }
    }
}
