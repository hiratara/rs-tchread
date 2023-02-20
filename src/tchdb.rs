use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    mem,
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
pub struct BucketElem(pub u32);

#[derive(BinRead, Debug)]
#[br(import(bucket_number: u64))]
pub struct Buckets(#[br(count = bucket_number)] pub Vec<BucketElem>); // also needs u64 instance

#[derive(BinRead, Debug)]
pub struct FreeBlockPoolElement {
    pub offset: VNum<u32>,
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
pub enum Record {
    #[br(magic = 0xc8u8)]
    Record {
        hash_value: u8,
        left_chain: BucketElem,
        right_chain: BucketElem,
        padding_size: u16,
        key_size: VNum<u32>,
        value_size: VNum<u32>,
        #[br(count = key_size.0)]
        key: Vec<u8>,
        #[br(count = value_size.0)]
        value: Vec<u8>,
        #[br(count = padding_size)]
        padding: Vec<u8>,
    },
    #[br(magic = 0xb0u8)]
    FreeBlock {
        block_size: u32,
        #[br(count = block_size - 5)]
        padding: Vec<u8>,
    },
}

pub struct TCHDB<T> {
    pub reader: T,
    pub header: Header,
    pub alignment: u32,
    pub bucket_offset: u64, // always be 256
    pub free_block_pool_offset: u64,
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

impl<T> TCHDB<T>
where
    T: Read + Seek + Sized,
{
    pub fn new(mut reader: T) -> Self {
        reader.seek(SeekFrom::Start(0)).unwrap();
        let header: Header = reader.read_ne().unwrap();

        let alignment = 2u32.pow(header.alignment_power as u32);
        let bucket_offset = reader.stream_position().unwrap();
        debug_assert_eq!(bucket_offset, 256);

        let free_block_pool_offset =
            bucket_offset + header.bucket_number * mem::size_of::<BucketElem>() as u64;

        TCHDB {
            reader,
            header,
            alignment,
            bucket_offset,
            free_block_pool_offset,
        }
    }

    pub fn read_buckets(&mut self) -> Buckets {
        self.reader
            .seek(SeekFrom::Start(self.bucket_offset))
            .unwrap();
        let buckets = self
            .reader
            .read_ne_args((self.header.bucket_number,))
            .unwrap();

        debug_assert_eq!(
            self.reader.stream_position().unwrap(),
            self.free_block_pool_offset
        );

        buckets
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

    pub fn read_records(&mut self) -> Vec<Record> {
        self.reader
            .seek(SeekFrom::Start(self.header.first_record))
            .unwrap();

        let mut records = Vec::with_capacity(self.header.record_number as usize);

        loop {
            let record: Record = self.reader.read_ne().unwrap();
            records.push(record);

            let pos = self.reader.stream_position().unwrap();
            if pos >= self.header.file_size {
                break;
            }
        }

        records
    }

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
