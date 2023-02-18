use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    mem,
    ops::{Add, AddAssign, Mul, ShlAssign, Sub},
    path::Path,
};

use binread::{BinRead, BinReaderExt, BinResult, ReadOptions};

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

#[derive(Debug)]
pub struct VNum<T>(pub T);

impl<T> BinRead for VNum<T>
where
    T: From<u8> + Ord + Add + Mul<Output = T> + Sub<Output = T> + ShlAssign<i32> + AddAssign + Copy,
{
    type Args = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &ReadOptions,
        args: Self::Args,
    ) -> BinResult<Self> {
        let mut num = T::from(0);
        let mut base = T::from(1);

        loop {
            let x = T::from(<u8>::read_options(reader, options, args)?);
            if x < T::from(0xA0) {
                num += x * base;
                break;
            }
            num += base * (T::from(0xFF) - x);
            base <<= 7;
        }

        Ok(VNum(num))
    }
}

#[derive(BinRead, Debug)]
pub struct FreeBlockPoolElement {
    pub offset: VNum<u32>,
    pub size: VNum<u32>,
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
            bucket_offset + header.bucket_number * mem::size_of::<i32>() as u64;

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
}
