mod tchdb;

use std::io::{Seek, SeekFrom};

use binread::{BinRead, BinReaderExt};
use tchdb::VNum;

use crate::tchdb::{Buckets, FreeBlockPoolElement, TCHDB};

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
    let mut tchdb = TCHDB::open("casket.tch");
    println!("{:?}", &tchdb.header);

    let buckets: Buckets = tchdb.read_buckets();
    println!("bucket length: {}", buckets.0.len());
    for (i, pos) in buckets.0.iter().enumerate().filter(|&(_, &n)| n != 0) {
        println!("bucket {} pos: {:#01x}", i, pos * tchdb.alignment);
    }

    println!(
        "free_block_pool offset: {:#01x}",
        tchdb.free_block_pool_offset,
    );
    for elem in tchdb.read_free_block_pool().into_iter() {
        println!(
            "free_block_pool: offset={:#01x}, size={}",
            &elem.offset.0 * tchdb.alignment,
            &elem.size.0
        );
    }

    tchdb
        .reader
        .seek(SeekFrom::Start(tchdb.header.first_record))
        .unwrap();
    loop {
        let record: Record = tchdb.reader.read_ne().unwrap();
        println!("{:?}", &record);

        let pos = tchdb.reader.stream_position().unwrap();
        if pos >= tchdb.header.file_size {
            break;
        }
    }
}
