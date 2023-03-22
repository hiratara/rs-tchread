use std::{
    fmt::LowerHex,
    io::{self, BufWriter, Read, Seek, Write},
    ops::Shl,
    path::Path,
};

use binrw::{BinRead, Endian};
use structopt::StructOpt;

use tchread::{
    binrw_types::{Buckets, RecordSpace},
    load::{self, TCHDBLoaded},
    TCHDB,
};

#[derive(StructOpt)]
/// A tool to read TokyoCabinet hash database files
struct Command {
    #[structopt(long)]
    /// Read a bigendian file (which violates the specification)
    bigendian: bool,
    #[structopt(subcommand)]
    sub_command: SubCommand,
}

#[derive(StructOpt)]
enum SubCommand {
    #[structopt(setting(structopt::clap::AppSettings::Hidden))]
    Test(Test),
    Get(Get),
    TraceToGet(TraceToGet),
    DumpBucket(DumpBucket),
    List(List),
    Inspect(Inspect),
}

fn main() {
    let command = Command::from_args();
    let endian = if command.bigendian {
        Endian::Big
    } else {
        Endian::Little
    };
    match command.sub_command {
        SubCommand::Test(test) => test.run_with_endian(endian),
        SubCommand::Get(get) => get.run_with_endian(endian),
        SubCommand::TraceToGet(trace_to_get) => trace_to_get.run_with_endian(endian),
        SubCommand::DumpBucket(dump_bucket) => dump_bucket.run_with_endian(endian),
        SubCommand::List(list) => list.run_with_endian(endian),
        SubCommand::Inspect(inspect) => inspect.run_with_endian(endian),
    }
}

trait Executer {
    fn run_with_endian(&self, endian: Endian) {
        let path = self.path();
        match load::open_with_endian(&path, endian) {
            TCHDBLoaded::Large(tchdb) => self.run_large(tchdb),
            TCHDBLoaded::Small(tchdb) => self.run_small(tchdb),
        }
    }

    fn path(&self) -> &Path;

    fn run_large<R: Read + Seek>(&self, tchdb: TCHDB<u64, R>);

    fn run_small<R: Read + Seek>(&self, tchdb: TCHDB<u32, R>);
}

#[derive(StructOpt)]
struct Test {
    path: String,
}

impl Executer for Test {
    #[inline]
    fn path(&self) -> &Path {
        Path::new(&self.path)
    }

    fn run_large<R: Read + Seek>(&self, tchdb: TCHDB<u64, R>) {
        self.run_each_size(tchdb)
    }

    fn run_small<R: Read + Seek>(&self, tchdb: TCHDB<u32, R>) {
        self.run_each_size(tchdb)
    }
}

impl Test {
    fn run_each_size<B, R>(&self, mut tchdb: TCHDB<B, R>)
    where
        B: 'static
            + BinRead
            + Copy
            + std::fmt::Debug
            + Eq
            + Shl<u8, Output = B>
            + LowerHex
            + Into<u64>,
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
            .filter(|(_, n)| n.offset(tchdb.header.alignment_power) != 0)
        {
            println!(
                "bucket {} pos: {:#01x}",
                i,
                pos.offset(tchdb.header.alignment_power)
            );
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

        let mut tchdb = tchdb.into_multi();
        for record in tchdb.read_record_spaces_multi() {
            println!("{:?}", &record);
            if let RecordSpace::Record(record) = record {
                let key = tchdb.hash(&record.key);
                println!("calculated hash: {:?}", key);
                println!("got record: {:?}", tchdb.get_record(&key));
            }
        }
        let mut tchdb = tchdb.into_inner();

        println!("XXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");

        for c in 'a'..='z' {
            let value = tchdb.get(&c.to_string());
            println!("{:?} => {:?}", c, value);
        }

        let value = tchdb.get("NOT_EXIST");
        println!("NOT_EXIST => {:?}", value);
    }
}

/// Print the value of a record
#[derive(StructOpt)]
struct Get {
    path: String,
    key: String,
}

impl Executer for Get {
    #[inline]
    fn path(&self) -> &Path {
        Path::new(&self.path)
    }

    fn run_large<R: Read + Seek>(&self, tchdb: TCHDB<u64, R>) {
        self.run_each_size(tchdb)
    }

    fn run_small<R: Read + Seek>(&self, tchdb: TCHDB<u32, R>) {
        self.run_each_size(tchdb)
    }
}

impl Get {
    fn run_each_size<B, R>(&self, mut tchdb: TCHDB<B, R>)
    where
        B: 'static
            + BinRead
            + Copy
            + std::fmt::Debug
            + Eq
            + Shl<u8, Output = B>
            + LowerHex
            + Into<u64>,
        <B as BinRead>::Args<'static>: Default,
        R: Read + Seek,
    {
        let stdout = io::stdout().lock();
        let mut stdout = BufWriter::new(stdout);

        if let Some(value) = tchdb.get(&self.key) {
            stdout.write_all(&value).unwrap();
            writeln!(stdout, "").unwrap();
        }
    }
}

/// Print all records traced to find the key
#[derive(StructOpt)]
struct TraceToGet {
    path: String,
    key: String,
}

impl Executer for TraceToGet {
    #[inline]
    fn path(&self) -> &Path {
        Path::new(&self.path)
    }

    fn run_large<R: Read + Seek>(&self, tchdb: TCHDB<u64, R>) {
        self.run_each_size(tchdb)
    }

    fn run_small<R: Read + Seek>(&self, tchdb: TCHDB<u32, R>) {
        self.run_each_size(tchdb)
    }
}

impl TraceToGet {
    fn run_each_size<B, R>(&self, mut tchdb: TCHDB<B, R>)
    where
        B: 'static
            + BinRead
            + Copy
            + std::fmt::Debug
            + Eq
            + Shl<u8, Output = B>
            + LowerHex
            + Into<u64>,
        <B as BinRead>::Args<'static>: Default,
        R: Read + Seek,
    {
        let stdout = io::stdout().lock();
        let mut stdout = BufWriter::new(stdout);

        let (key_with_hash, found, visited_records) = tchdb.get_detail(&self.key);
        writeln!(stdout, "bucket: {}", key_with_hash.idx).unwrap();
        writeln!(stdout, "hash: {}", key_with_hash.hash).unwrap();

        let len = visited_records.len();
        for (i, mut r) in visited_records.into_iter().enumerate() {
            write!(stdout, "record {}: hash={}, key=", i + 1, r.hash_value,).unwrap();
            stdout.write_all(&r.key).unwrap();
            writeln!(stdout, "").unwrap();
            if found && i == len - 1 {
                r.value.read_value(&mut tchdb.reader);
                let value = r.value.into_value().into_value();
                stdout.write_all(&value).unwrap();
                writeln!(stdout, "").unwrap();
            }
        }
    }
}

/// Print all records in the bucket
#[derive(StructOpt)]
struct DumpBucket {
    path: String,
    bucket_number: u64,
}

impl Executer for DumpBucket {
    #[inline]
    fn path(&self) -> &Path {
        Path::new(&self.path)
    }

    fn run_large<R: Read + Seek>(&self, tchdb: TCHDB<u64, R>) {
        self.run_each_size(tchdb)
    }

    fn run_small<R: Read + Seek>(&self, tchdb: TCHDB<u32, R>) {
        self.run_each_size(tchdb)
    }
}

impl DumpBucket {
    fn run_each_size<B, R>(&self, mut tchdb: TCHDB<B, R>)
    where
        B: 'static
            + BinRead
            + Copy
            + std::fmt::Debug
            + Eq
            + Shl<u8, Output = B>
            + LowerHex
            + Into<u64>,
        <B as BinRead>::Args<'static>: Default,
        R: Read + Seek,
    {
        let stdout = io::stdout().lock();
        let mut stdout = BufWriter::new(stdout);

        let records = tchdb.dump_bucket(self.bucket_number);
        for (i, r) in records.into_iter().enumerate() {
            write!(stdout, "record {}: hash={}, key=", i + 1, r.hash_value,).unwrap();
            stdout.write_all(&r.key).unwrap();
            writeln!(stdout, "").unwrap();
        }
    }
}

/// Print keys of all records, separated by line feeds
#[derive(StructOpt)]
struct List {
    path: String,
    #[structopt(long)]
    /// Print values of records also
    pv: bool,
}

impl Executer for List {
    #[inline]
    fn path(&self) -> &Path {
        Path::new(&self.path)
    }

    fn run_large<R: Read + Seek>(&self, tchdb: TCHDB<u64, R>) {
        self.run_each_size(tchdb)
    }

    fn run_small<R: Read + Seek>(&self, tchdb: TCHDB<u32, R>) {
        self.run_each_size(tchdb)
    }
}

impl List {
    fn run_each_size<B, R>(&self, mut tchdb: TCHDB<B, R>)
    where
        B: 'static
            + BinRead
            + Copy
            + std::fmt::Debug
            + Eq
            + Shl<u8, Output = B>
            + LowerHex
            + Into<u64>,
        <B as BinRead>::Args<'static>: Default,
        R: Read + Seek,
    {
        let stdout = io::stdout().lock();
        let mut stdout = BufWriter::new(stdout);

        for record in tchdb.read_record_spaces(self.pv) {
            if let RecordSpace::Record(record) = record {
                stdout.write_all(&record.key).unwrap();
                if self.pv {
                    stdout.write_all(b"\t").unwrap();
                    stdout
                        .write_all(&record.value.into_value().into_value())
                        .unwrap();
                }
                stdout.write_all(b"\n").unwrap();
            }
        }
    }
}

/// Traverse through and stat all records
#[derive(StructOpt)]
struct Inspect {
    path: String,
}

impl Executer for Inspect {
    #[inline]
    fn path(&self) -> &Path {
        Path::new(&self.path)
    }

    fn run_large<R: Read + Seek>(&self, tchdb: TCHDB<u64, R>) {
        self.run_each_size(tchdb)
    }

    fn run_small<R: Read + Seek>(&self, tchdb: TCHDB<u32, R>) {
        self.run_each_size(tchdb)
    }
}

impl Inspect {
    fn run_each_size<B, R>(&self, mut tchdb: TCHDB<B, R>)
    where
        B: 'static
            + BinRead
            + Copy
            + std::fmt::Debug
            + Eq
            + Shl<u8, Output = B>
            + LowerHex
            + Into<u64>,
        <B as BinRead>::Args<'static>: Default,
        R: Read + Seek,
    {
        let bucket_num;
        let empty_bucket_num;
        {
            let buckets: Buckets<B> = tchdb.read_buckets();
            bucket_num = buckets.0.len();
            empty_bucket_num = buckets.0.into_iter().filter(|b| b.is_empty()).count();
        }

        let mut record_num = 0u64;
        let mut record_no_children = 0u64;
        let mut record_one_child = 0u64;
        let mut record_two_children = 0u64;
        let mut key_length = 0.0f64;
        let mut value_length = 0.0f64;
        let mut padding_length = 0.0f64;
        let mut freeblock_num = 0u64;
        for record in tchdb.read_record_spaces(false) {
            match record {
                RecordSpace::Record(record) => {
                    record_num += 1;
                    key_length += record.key_size.0 as f64;
                    value_length += record.value_size.0 as f64;
                    padding_length += record.padding_size as f64;
                    if record.right_chain.is_empty() {
                        if record.left_chain.is_empty() {
                            record_no_children += 1;
                        } else {
                            record_one_child += 1;
                        }
                    } else {
                        if record.left_chain.is_empty() {
                            record_one_child += 1;
                        } else {
                            record_two_children += 1;
                        }
                    }
                }
                RecordSpace::FreeBlock(_) => {
                    freeblock_num += 1;
                }
            }
        }

        let stdout = io::stdout().lock();
        let mut stdout = BufWriter::new(stdout);

        writeln!(stdout, "# of buckets: {}", bucket_num).unwrap();
        writeln!(stdout, "# of empty buckets: {}", empty_bucket_num).unwrap();
        writeln!(stdout, "# of records: {}", record_num).unwrap();
        writeln!(
            stdout,
            "# of records without children: {}",
            record_no_children
        )
        .unwrap();
        writeln!(stdout, "# of records with one child: {}", record_one_child).unwrap();
        writeln!(
            stdout,
            "# of records with two children: {}",
            record_two_children
        )
        .unwrap();
        writeln!(
            stdout,
            "avg of key length: {}",
            key_length / record_num as f64
        )
        .unwrap();
        writeln!(
            stdout,
            "avg of value length: {}",
            value_length / record_num as f64
        )
        .unwrap();
        writeln!(
            stdout,
            "avg of padding length: {}",
            padding_length / record_num as f64
        )
        .unwrap();
        writeln!(stdout, "# of free blocks: {}", freeblock_num).unwrap();
    }
}
