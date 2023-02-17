use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    ops::{Add, AddAssign, Mul, ShlAssign, Sub},
};

use binread::{BinRead, BinReaderExt, BinResult, ReadOptions};

#[derive(Debug)]
struct VNum<T>(T);

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
#[br(little)]
struct Header {
    #[br(count = 32, assert(magic_number.starts_with(b"ToKyO CaBiNeT")))]
    magic_number: Vec<u8>,
    #[br(assert(database_type == 0))]
    database_type: u8,
    additional_flags: u8,
    alignment_power: u8,
    free_block_pool_power: u8,
    #[br(pad_after = 3)]
    options: u8,
    bucket_number: u64,
    record_number: u64,
    file_size: u64,
    #[br(pad_after = 56)]
    first_record: u64,
    #[br(count = 128)]
    opaque_region: Vec<u8>,
}

#[derive(BinRead, Debug)]
#[br(import(bucket_number: u64))]
struct Buckets(#[br(count = bucket_number)] Vec<u32>); // also needs u64 instance

#[derive(BinRead, Debug)]
struct FreeBlockPoolElement {
    offset: VNum<u32>,
    size: VNum<u32>,
}

#[derive(BinRead, Debug)]
#[br(little)]
enum Record {
    #[br(magic = 0xc8u8)]
    Record {
        hash_value: u8,
        #[br(count = 4)]
        left_chain: Vec<u8>,
        #[br(count = 4)]
        right_chain: Vec<u8>,
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

fn main() {
    let mut file = File::open("casket.tch").unwrap();
    let header: Header = file.read_ne().unwrap();
    println!("{:?}", &header);

    let alignment = 2u32.pow(header.alignment_power as u32);

    let buckets: Buckets = file.read_ne_args((header.bucket_number,)).unwrap();
    println!("bucket length: {}", buckets.0.len());
    for (i, pos) in buckets.0.iter().enumerate().filter(|&(_, &n)| n != 0) {
        println!("bucket {} pos: {:#01x}", i, pos * alignment);
    }

    println!(
        "free_block_pool offset: {:#01x}",
        file.stream_position().unwrap(),
        // 256 + header.bucket_number * mem::size_of::<i32>() as u64,
    );
    loop {
        let elem: FreeBlockPoolElement = file.read_ne().unwrap();
        if elem.offset.0 == 0 && elem.size.0 == 0 {
            break;
        }
        println!(
            "free_block_pool: offset={:#01x}, size={}",
            &elem.offset.0 * alignment,
            &elem.size.0
        );
    }

    file.seek(SeekFrom::Start(header.first_record)).unwrap();
    loop {
        let record: Record = file.read_ne().unwrap();
        println!("{:?}", &record);

        let pos = file.stream_position().unwrap();
        if pos >= header.file_size {
            break;
        }
    }
}
