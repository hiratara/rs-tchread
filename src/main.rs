mod tchdb;

use std::{
    fmt::LowerHex,
    io::{self, Read, Seek, Write},
    ops::Shl,
};

use binrw::BinRead;
use structopt::StructOpt;
use tchdb::TCHDB;

use crate::tchdb::{
    binrw_types::{Buckets, RecordSpace},
    TCHDBImpl,
};

#[derive(StructOpt)]
enum Command {
    Test { path: String },
    Get { path: String, key: String },
    GetTrace { path: String, key: String },
    DumpBucket { path: String, bucket_number: u64 },
    List { path: String },
}

fn main() {
    match Command::from_args() {
        Command::Test { path } => run_test(&path),
        Command::Get { path, key } => run_get(&path, &key),
        Command::GetTrace { path, key } => run_get_trace(&path, &key),
        Command::DumpBucket {
            path,
            bucket_number,
        } => run_dump_bucket(&path, bucket_number),
        Command::List { path } => run_list(&path),
    }
}

fn run_test(path: &str) {
    match TCHDB::open_multi(&path) {
        TCHDB::Large(tchdb) => run_test_with_tchdb(tchdb),
        TCHDB::Small(tchdb) => run_test_with_tchdb(tchdb),
    }
}

fn run_test_with_tchdb<B, R>(mut tchdb: TCHDBImpl<B, R>)
where
    B: 'static + BinRead + Copy + std::fmt::Debug + Eq + Shl<u8, Output = B> + LowerHex + Into<u64>,
    <B as BinRead>::Args<'static>: Default,
    R: Read + Seek + Clone,
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

    for record in tchdb.read_record_spaces_multi() {
        println!("{:?}", &record);
        if let RecordSpace::Record(record) = record {
            let key = tchdb.hash(&record.key);
            println!("calculated hash: {:?}", key);
            println!("got record: {:?}", tchdb.get_record(&key));
        }
    }

    println!("XXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");

    for c in 'a'..='z' {
        let value = tchdb.get(&c.to_string());
        println!("{:?} => {:?}", c, value);
    }

    let value = tchdb.get("NOT_EXIST");
    println!("NOT_EXIST => {:?}", value);
}

fn run_get(path: &str, key: &str) {
    match TCHDB::open(&path) {
        TCHDB::Large(tchdb) => run_get_with_tchdb(tchdb, key),
        TCHDB::Small(tchdb) => run_get_with_tchdb(tchdb, key),
    }
}

fn run_get_with_tchdb<B, R>(mut tchdb: TCHDBImpl<B, R>, key: &str)
where
    B: 'static + BinRead + Copy + std::fmt::Debug + Eq + Shl<u8, Output = B> + LowerHex + Into<u64>,
    <B as BinRead>::Args<'static>: Default,
    R: Read + Seek,
{
    if let Some(value) = tchdb.get(key) {
        println!("{}", value);
    }
}

fn run_get_trace(path: &str, key: &str) {
    match TCHDB::open(&path) {
        TCHDB::Large(tchdb) => run_get_trace_with_tchdb(tchdb, key),
        TCHDB::Small(tchdb) => run_get_trace_with_tchdb(tchdb, key),
    }
}

fn run_get_trace_with_tchdb<B, R>(mut tchdb: TCHDBImpl<B, R>, key: &str)
where
    B: 'static + BinRead + Copy + std::fmt::Debug + Eq + Shl<u8, Output = B> + LowerHex + Into<u64>,
    <B as BinRead>::Args<'static>: Default,
    R: Read + Seek,
{
    let (key_with_hash, found, visited_records) = tchdb.get_detail(key);
    println!("bucket: {}", key_with_hash.idx);
    println!("hash: {}", key_with_hash.hash);

    let len = visited_records.len();
    for (i, r) in visited_records.into_iter().enumerate() {
        println!(
            "record {}: hash={}, key={}",
            i + 1,
            r.hash_value,
            String::from_utf8(r.key).unwrap(),
        );
        if found && i == len - 1 {
            println!("{}", String::from_utf8(r.value).unwrap());
        }
    }
}

fn run_dump_bucket(path: &str, bucket_number: u64) {
    match TCHDB::open(&path) {
        TCHDB::Large(tchdb) => run_dump_bucket_with_tchdb(tchdb, bucket_number),
        TCHDB::Small(tchdb) => run_dump_bucket_with_tchdb(tchdb, bucket_number),
    }
}

fn run_dump_bucket_with_tchdb<B, R>(mut tchdb: TCHDBImpl<B, R>, bucket_number: u64)
where
    B: 'static + BinRead + Copy + std::fmt::Debug + Eq + Shl<u8, Output = B> + LowerHex + Into<u64>,
    <B as BinRead>::Args<'static>: Default,
    R: Read + Seek,
{
    let records = tchdb.dump_bucket(bucket_number);
    for (i, r) in records.into_iter().enumerate() {
        println!(
            "record {}: hash={}, key={}",
            i + 1,
            r.hash_value,
            String::from_utf8(r.key).unwrap(),
        );
    }
}

fn run_list(path: &str) {
    match TCHDB::open_multi(&path) {
        TCHDB::Large(tchdb) => run_list_with_tchdb(tchdb),
        TCHDB::Small(tchdb) => run_list_with_tchdb(tchdb),
    }
}

fn run_list_with_tchdb<B, R>(mut tchdb: TCHDBImpl<B, R>)
where
    B: 'static + BinRead + Copy + std::fmt::Debug + Eq + Shl<u8, Output = B> + LowerHex + Into<u64>,
    <B as BinRead>::Args<'static>: Default,
    R: Read + Seek + Clone,
{
    let mut stdout = io::stdout().lock();
    for record in tchdb.read_record_spaces_multi() {
        if let RecordSpace::Record(record) = record {
            stdout.write_all(&record.key).unwrap();
            stdout.write_all(b"\n").unwrap();
        }
    }
}
