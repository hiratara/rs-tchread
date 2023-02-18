use std::{
    fs::File,
    io::{Read, Seek},
    path::Path,
};

use binread::{BinRead, BinReaderExt};

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
#[br(import(bucket_number: u64))]
pub struct Buckets(#[br(count = bucket_number)] pub Vec<u32>); // also needs u64 instance

pub struct TCHDB<T> {
    pub reader: T,
    pub header: Header,
    pub bucket_offset: u64, // always be 256
    pub alignment: u32,
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
        let header: Header = reader.read_ne().unwrap();

        let alignment = 2u32.pow(header.alignment_power as u32);
        let bucket_offset = reader.stream_position().unwrap();
        debug_assert_eq!(bucket_offset, 256);

        TCHDB {
            reader,
            header,
            bucket_offset,
            alignment,
        }
    }

    pub fn read_buckets(&mut self) -> Buckets {
        self.reader
            .read_ne_args((self.header.bucket_number,))
            .unwrap()
    }
}
