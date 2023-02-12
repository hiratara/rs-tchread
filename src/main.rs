use std::{fs::File, io::{Seek, SeekFrom, Read}};

use binread::{BinRead, BinReaderExt, ReadOptions, BinResult};

#[derive(Debug)]
struct VNum(u32);

impl BinRead for VNum {
    type Args = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &ReadOptions,
        args: Self::Args,
    ) -> BinResult<Self> {
        let mut num = 0u32;
        let mut base = 1i32;

        loop {
            let x = <i8>::read_options(reader, options, args)?;
            if x >= 0 {
                num += (x as i32 * base) as u32;
                break;
            }
            num += (base * (x + 1) as i32 * -1) as u32;
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
    key_size: VNum,
    value_size: VNum,
    #[br(count = key_size.0)]
    key: Vec<u8>,
    #[br(count = value_size.0)]
    value: Vec<u8>,
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
