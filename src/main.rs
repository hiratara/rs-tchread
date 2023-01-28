use std::fs::File;

use binread::{BinRead, BinReaderExt};

#[derive(BinRead, Debug)]
struct Header {
    #[br(count = 32)]
    magic_number: Vec<u8>,
    #[br(count = 1)]
    database_type: Vec<u8>,
    #[br(count = 1)]
    additional_flags: Vec<u8>,
    #[br(count = 1)]
    alignment_power: Vec<u8>,
    #[br(count = 1)]
    free_block_pool_power: Vec<u8>,
    #[br(count = 1, pad_after = 3)]
    options: Vec<u8>,
    #[br(count = 8)]
    bucket_number: Vec<u8>,
    #[br(count = 8)]
    record_number: Vec<u8>,
    #[br(count = 8)]
    file_size: Vec<u8>,
    #[br(count = 8)]
    first_record: Vec<u8>,
    #[br(count = 128)]
    opaque_region: Vec<u8>,
}

fn main() {
    let mut file = File::open("casket.tch").unwrap();
    let header: Header = file.read_be().unwrap();
    println!("{:?}", &header);
}
