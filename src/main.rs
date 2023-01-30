use std::{fs::File, io::{Seek, SeekFrom}};

use binread::{BinRead, BinReaderExt};



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
    first_record: u64,
    /*
    #[br(count = 128)]
    opaque_region: Vec<u8>,
    */
}

#[derive(BinRead, Debug)]
#[br(little)]
struct Record {
    #[br(assert(magic_number == 0xc8))]
    magic_number: u8,
    hash_value: u8,
    #[br(count = 4)]
    left_chain: Vec<u8>,
    #[br(count = 4)]
    right_chain: Vec<u8>,
    padding_size: u16,
    // #[br(count = 1)]
    // key_size: Vec<u8>,
    // #[br(count = 1)]
    // value_size: Vec<u8>,
    // #[br(count = key_size)]
    // key: Vec<u8>,
    // #[br(count = value_size)]
    // value: Vec<u8>,
    // #[br(count = padding_size)]
    // padding: Vec<u8>,
}

fn main() {
    let mut file = File::open("casket.tch").unwrap();
    let header: Header = file.read_ne().unwrap();
    println!("{:?}", &header);

    file.seek(SeekFrom::Start(header.first_record)).unwrap();
    let record: Record = file.read_ne().unwrap();
    println!("{:?}", &record);
}
