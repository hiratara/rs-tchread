mod tchdb;

use crate::tchdb::{Buckets, TCHDB};

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

    for record in tchdb.read_records() {
        println!("{:?}", &record);
    }
}
