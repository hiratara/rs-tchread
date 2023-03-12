pub mod binrw_types;
mod multi_read;
mod vnum;

use std::{
    cmp::Ordering,
    fs::File,
    io::{Read, Seek, SeekFrom},
    marker::PhantomData,
    mem,
    ops::Shl,
    path::Path,
};

use binrw::{io::BufReader, BinRead, BinReaderExt, Endian, Error};

use self::{
    binrw_types::{Buckets, FreeBlockPoolElement, Header, Record, RecordOffset, RecordSpace},
    multi_read::MultiRead,
};

#[derive(Debug)]
pub struct KeyWithHash<'a> {
    pub key: &'a [u8],
    pub idx: u64,
    pub hash: u8,
}

pub struct TCHDBImpl<B, R> {
    pub reader: R,
    pub endian: Endian,
    pub header: Header,
    pub bucket_offset: u64, // always be 256
    pub free_block_pool_offset: u64,
    bucket_type: PhantomData<fn() -> B>,
}

impl<B, R> TCHDBImpl<B, R> {
    pub fn hash<'a>(&self, key: &'a [u8]) -> KeyWithHash<'a> {
        let mut idx: u64 = 19780211;
        for &b in key {
            idx = idx.wrapping_mul(37).wrapping_add(b as u64);
        }
        idx %= self.header.bucket_number;

        let mut hash: u32 = 751;
        for &b in key.into_iter().rev() {
            hash = hash.wrapping_mul(31) ^ b as u32;
        }

        KeyWithHash {
            key,
            idx,
            hash: hash as u8,
        }
    }
}

impl<B, R> TCHDBImpl<B, R>
where
    R: Seek,
{
    pub fn read_record_spaces<'a>(&'a mut self) -> RecordSpaceIter<'a, R, B> {
        RecordSpaceIter::new(&mut self.reader, self.endian, &self.header)
    }
}

impl<B, R> TCHDBImpl<B, R>
where
    R: Read + Seek,
{
    fn new(mut reader: R, endian: Endian, header: Header) -> Self {
        let bucket_offset = reader.stream_position().unwrap();
        debug_assert_eq!(bucket_offset, 256);

        let free_block_pool_offset =
            bucket_offset + header.bucket_number * mem::size_of::<B>() as u64;

        TCHDBImpl {
            reader,
            endian,
            header,
            bucket_offset,
            free_block_pool_offset,
            bucket_type: PhantomData,
        }
    }

    pub fn read_free_block_pool(&mut self) -> Vec<FreeBlockPoolElement> {
        self.reader
            .seek(SeekFrom::Start(self.free_block_pool_offset))
            .unwrap();

        let pool_size = 2usize.pow(self.header.free_block_pool_power as u32);
        let mut pool = Vec::with_capacity(pool_size);
        loop {
            let elem: FreeBlockPoolElement = self.reader.read_type(self.endian).unwrap();
            if elem.offset.0 == 0 && elem.size.0 == 0 {
                break;
            }
            pool.push(elem);
        }
        pool
    }
}

impl<B, R> TCHDBImpl<B, R>
where
    B: BinRead,
    <B as BinRead>::Args<'static>: Default,
    R: Read + Seek,
{
    pub fn read_buckets(&mut self) -> Buckets<B> {
        self.reader
            .seek(SeekFrom::Start(self.bucket_offset))
            .unwrap();
        let buckets = self
            .reader
            .read_type_args(
                self.endian,
                (self.header.alignment_power, self.header.bucket_number),
            )
            .unwrap();

        debug_assert_eq!(
            self.reader.stream_position().unwrap(),
            self.free_block_pool_offset
        );

        buckets
    }

    fn read_bucket(&mut self, idx: u64) -> RecordOffset<B> {
        let pos = self.bucket_offset + mem::size_of::<B>() as u64 * idx;
        self.reader.seek(SeekFrom::Start(pos)).unwrap();
        self.reader
            .read_type_args(self.endian, (self.header.alignment_power,))
            .unwrap()
    }
}

impl<B, R> TCHDBImpl<B, R>
where
    B: BinRead + Copy + Shl<u8, Output = B> + Into<u64>,
    <B as BinRead>::Args<'static>: Default,
    R: Read + Seek,
{
    fn read_record_space(&mut self, rec_off: RecordOffset<B>) -> RecordSpace<B> {
        self.reader.seek(SeekFrom::Start(rec_off.offset())).unwrap();
        self.reader
            .read_type_args(self.endian, (self.header.alignment_power,))
            .unwrap()
    }

    pub fn get_record(&mut self, key: &KeyWithHash) -> Option<Record<B>> {
        let (found, mut log) = self.get_record_detail(key);
        if found {
            Some(log.remove(log.len() - 1))
        } else {
            None
        }
    }

    pub fn get_record_detail(&mut self, key: &KeyWithHash) -> (bool, Vec<Record<B>>) {
        let mut rec_off = self.read_bucket(key.idx);

        let mut visited_records = Vec::new();
        loop {
            if rec_off.is_empty() {
                return (false, visited_records);
            }

            let record = match self.read_record_space(rec_off) {
                RecordSpace::FreeBlock(_) => return (false, visited_records),
                RecordSpace::Record(r) => r,
            };

            if key.hash > record.meta.hash_value {
                rec_off = record.meta.left_chain;
                visited_records.push(record);
                continue;
            } else if key.hash < record.meta.hash_value {
                rec_off = record.meta.right_chain;
                visited_records.push(record);
                continue;
            }

            match key.key.cmp(&record.meta.key) {
                Ordering::Greater => {
                    rec_off = record.meta.left_chain;
                    visited_records.push(record);
                    continue;
                }
                Ordering::Less => {
                    rec_off = record.meta.right_chain;
                    visited_records.push(record);
                    continue;
                }
                Ordering::Equal => {
                    visited_records.push(record);
                    return (true, visited_records);
                }
            }
        }
    }

    pub fn get(&mut self, key_str: &str) -> Option<Vec<u8>> {
        let key = self.hash(key_str.as_bytes());
        match self.get_record(&key) {
            None => None,
            Some(record) => {
                let value = record.value.read_value(&mut self.reader);
                Some(value)
            }
        }
    }

    pub fn get_detail<'a>(&mut self, key_str: &'a str) -> (KeyWithHash<'a>, bool, Vec<Record<B>>) {
        let key = self.hash(key_str.as_bytes());
        let (found, visited_records) = self.get_record_detail(&key);
        (key, found, visited_records)
    }

    pub fn dump_bucket(&mut self, bucket_number: u64) -> Vec<Record<B>> {
        let mut records = Vec::new();
        let rec_off = self.read_bucket(bucket_number);

        self.traverse_records(rec_off, &mut records);

        records
    }

    fn traverse_records(&mut self, rec_off: RecordOffset<B>, records: &mut Vec<Record<B>>) {
        if rec_off.is_empty() {
            return;
        }

        match self.read_record_space(rec_off) {
            RecordSpace::FreeBlock(_) => panic!("unexpected freespace found: {}", rec_off.offset()),
            RecordSpace::Record(record) => {
                let right = record.meta.right_chain;
                let left = record.meta.left_chain;
                self.traverse_records(right, records);
                records.push(record);
                self.traverse_records(left, records);
            }
        }
    }
}

pub enum TCHDB<R> {
    Small(TCHDBImpl<u32, R>),
    Large(TCHDBImpl<u64, R>),
}

impl TCHDB<BufReader<File>> {
    pub fn open_with_endian<T>(path: T, endian: Endian) -> Self
    where
        T: AsRef<Path>,
    {
        let file = File::open(path).unwrap();
        let file = BufReader::new(file);
        TCHDB::new(file, endian)
    }

    pub fn open<T>(path: T) -> Self
    where
        T: AsRef<Path>,
    {
        Self::open_with_endian(path, Endian::Little)
    }
}

impl TCHDB<MultiRead<BufReader<File>>> {
    pub fn open_multi_with_endian<T>(path: T, endian: Endian) -> Self
    where
        T: AsRef<Path>,
    {
        let file = File::open(path).unwrap();
        let file = BufReader::new(file);
        TCHDB::new(MultiRead::new(file), endian)
    }

    pub fn open_multi<T>(path: T) -> Self
    where
        T: AsRef<Path>,
    {
        Self::open_multi_with_endian(path, Endian::Little)
    }
}

impl<R> TCHDB<R>
where
    R: Read + Seek,
{
    pub fn new(mut reader: R, endian: Endian) -> Self {
        reader.seek(SeekFrom::Start(0)).unwrap();
        let header: Header = reader.read_type(endian).unwrap();

        if header.options & 0x01 == 0x01 {
            TCHDB::Large(TCHDBImpl::new(reader, endian, header))
        } else {
            TCHDB::Small(TCHDBImpl::new(reader, endian, header))
        }
    }
}

pub struct RecordSpaceIter<'a, R, B> {
    reader: &'a mut R,
    endian: Endian,
    file_size: u64,
    alignment_power: u8,
    bucket_type: PhantomData<fn() -> B>,
}

impl<'a, R: Seek, B> RecordSpaceIter<'a, R, B> {
    fn new(reader: &'a mut R, endian: Endian, header: &Header) -> Self {
        reader.seek(SeekFrom::Start(header.first_record)).unwrap();

        RecordSpaceIter {
            reader,
            endian,
            file_size: header.file_size,
            alignment_power: header.alignment_power,
            bucket_type: PhantomData,
        }
    }
}

impl<'a, R, B> Iterator for RecordSpaceIter<'a, R, B>
where
    R: Read + Seek,
    B: BinRead,
    <B as BinRead>::Args<'static>: Default,
{
    type Item = RecordSpace<B>;

    fn next(&mut self) -> Option<Self::Item> {
        match self
            .reader
            .read_type_args(self.endian, (self.alignment_power,))
        {
            Ok(record_space) => record_space,
            Err(Error::EnumErrors { pos, .. }) if pos == self.file_size => None,
            Err(error) => panic!("{}", error),
        }
    }
}
