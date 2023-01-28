use std::fs::File;

use binread::{BinRead, BinReaderExt, NullString};



#[derive(BinRead, Debug)]
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

fn main() {
    let mut file = File::open("casket.tch").unwrap();
    let header: Header = file.read_be().unwrap();
    println!("{:?}", &header);
}
