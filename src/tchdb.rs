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

pub struct TCHDB<T>
where
    T: Read + Seek + Sized,
{
    pub file: T,
    pub header: Header,
    pub bucket_offset: u64, // always be 256
}

impl TCHDB<File> {
    pub fn open<T>(path: T) -> Self
    where
        T: AsRef<Path>,
    {
        let mut file = File::open("casket.tch").unwrap();
        let header: Header = file.read_ne().unwrap();

        let bucket_offset = file.stream_position().unwrap();
        debug_assert_eq!(bucket_offset, 256);

        TCHDB {
            file,
            header,
            bucket_offset,
        }
    }
}
