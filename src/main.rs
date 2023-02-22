mod tchdb;
mod vnum;

use std::env;

use crate::tchdb::{Buckets, Record, TCHDB};

// TODO: better implementation?
fn as_same_type_of<T: From<S>, S>(_: T, v: S) -> T {
    From::from(v)
}

fn main() {
    let path = env::args().take(2).last().unwrap();
    let mut tchdb = TCHDB::open(&path);
    println!("{:?}", &tchdb.header);

    let buckets: Buckets = tchdb.read_buckets();
    println!("bucket length: {}", buckets.0.len());
    for (i, pos) in buckets.0.iter().enumerate().filter(|&(_, n)| n.0 != 0) {
        println!(
            "bucket {} pos: {:#01x}",
            i,
            pos.0 * as_same_type_of(pos.0, tchdb.alignment)
        );
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

    for record in tchdb.read_records() {
        println!("{:?}", &record);
        if let Record::Record { key, .. } = record {
            println!("calculated hash: {:?}", tchdb.hash(&key));
        }
    }
}
