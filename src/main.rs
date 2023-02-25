mod tchdb;
mod vnum;

use std::{
    env,
    fmt::LowerHex,
    io::{Read, Seek},
    ops::Shl,
};

use binrw::BinRead;
use tchdb::TCHDB;

use crate::tchdb::{Buckets, RecordSpace, TCHDBImpl};

fn main() {
    let path = env::args().take(2).last().unwrap();
    match TCHDB::open(&path) {
        TCHDB::Large(tchdb) => run_with_tchdb(tchdb),
        TCHDB::Small(tchdb) => run_with_tchdb(tchdb),
    }
}

fn run_with_tchdb<B, R>(mut tchdb: TCHDBImpl<B, R>)
where
    B: 'static + BinRead + Copy + std::fmt::Debug + Eq + Shl<u8, Output = B> + LowerHex + Into<u64>,
    <B as BinRead>::Args<'static>: Default,
    R: Read + Seek,
{
    println!("{:?}", &tchdb.header);

    let buckets: Buckets<B> = tchdb.read_buckets();
    println!("bucket length: {}", buckets.0.len());
    for (i, pos) in buckets
        .0
        .into_iter()
        .enumerate()
        .filter(|(_, n)| n.offset() != 0)
    {
        println!("bucket {} pos: {:#01x}", i, pos.offset());
    }

    println!(
        "free_block_pool offset: {:#01x}",
        tchdb.free_block_pool_offset,
    );
    for elem in tchdb.read_free_block_pool().into_iter() {
        println!(
            "free_block_pool: offset={:#01x}, size={}",
            &elem.offset.0 << tchdb.header.alignment_power,
            &elem.size.0
        );
    }

    for record in tchdb.read_record_spaces() {
        println!("{:?}", &record);
        if let RecordSpace::Record(record) = record {
            let key = tchdb.hash(&record.key);
            println!("calculated hash: {:?}", key);
            println!("got record: {:?}", tchdb.get(&key));
        }
    }
}
