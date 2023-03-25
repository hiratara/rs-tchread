pub mod binrw_types;
pub mod load;
mod multi_read;

use std::{
    cmp::Ordering,
    io::{Read, Seek, SeekFrom},
    marker::PhantomData,
    mem,
};

use binrw::{BinReaderExt, Endian};
use binrw_types::U32orU64;

use self::binrw_types::{Buckets, FreeBlockPoolElement, Header, Record, RecordOffset, RecordSpace};

#[derive(Debug)]
pub struct KeyWithHash<'a> {
    pub key: &'a [u8],
    pub idx: u64,
    pub hash: u8,
}

pub struct TCHDB<U, R> {
    pub reader: R,
    pub endian: Endian,
    pub header: Header,
    pub bucket_offset: u64, // always be 256
    pub free_block_pool_offset: u64,
    bucket_type: PhantomData<fn() -> U>,
}

impl<U, R> TCHDB<U, R> {
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

impl<U, R: Seek> TCHDB<U, R> {
    pub fn read_record_spaces<'a>(&'a mut self, pv: bool) -> RecordSpaceIter<'a, U, R> {
        RecordSpaceIter::new(&mut self.reader, pv, self.endian, &self.header)
    }
}

impl<U, R: Read + Seek> TCHDB<U, R> {
    fn new(mut reader: R, endian: Endian, header: Header) -> Self {
        let bucket_offset = reader.stream_position().unwrap();
        debug_assert_eq!(bucket_offset, 256);

        let free_block_pool_offset =
            bucket_offset + header.bucket_number * mem::size_of::<U>() as u64;

        TCHDB {
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

impl<U: U32orU64, R: Read + Seek> TCHDB<U, R> {
    pub fn read_buckets(&mut self) -> Buckets<U> {
        self.reader
            .seek(SeekFrom::Start(self.bucket_offset))
            .unwrap();
        let buckets = self
            .reader
            .read_type_args(self.endian, (self.header.bucket_number,))
            .unwrap();

        debug_assert_eq!(
            self.reader.stream_position().unwrap(),
            self.free_block_pool_offset
        );

        buckets
    }

    fn read_bucket(&mut self, idx: u64) -> RecordOffset<U> {
        let pos = self.bucket_offset + mem::size_of::<U>() as u64 * idx;
        self.reader.seek(SeekFrom::Start(pos)).unwrap();
        self.reader.read_type(self.endian).unwrap()
    }

    fn read_record_space(&mut self, rec_off: RecordOffset<U>, read_value: bool) -> RecordSpace<U> {
        let offset = rec_off.offset(self.header.alignment_power);
        self.reader.seek(SeekFrom::Start(offset)).unwrap();
        self.reader
            .read_type_args(self.endian, (offset, read_value))
            .unwrap()
    }

    pub fn get_record(&mut self, key: &KeyWithHash) -> Option<Record<U>> {
        let (found, mut log) = self.get_record_detail(key);
        if found {
            Some(log.remove(log.len() - 1))
        } else {
            None
        }
    }

    pub fn get_record_detail(&mut self, key: &KeyWithHash) -> (bool, Vec<Record<U>>) {
        let mut rec_off = self.read_bucket(key.idx);

        let mut visited_records = Vec::new();
        loop {
            if rec_off.is_empty() {
                return (false, visited_records);
            }

            let record = match self.read_record_space(rec_off, false) {
                RecordSpace::FreeBlock(_) => return (false, visited_records),
                RecordSpace::Record(r) => r,
            };

            if key.hash > record.hash_value {
                rec_off = record.left_chain;
                visited_records.push(record);
                continue;
            } else if key.hash < record.hash_value {
                rec_off = record.right_chain;
                visited_records.push(record);
                continue;
            }

            match key.key.cmp(&record.key) {
                Ordering::Greater => {
                    rec_off = record.left_chain;
                    visited_records.push(record);
                    continue;
                }
                Ordering::Less => {
                    rec_off = record.right_chain;
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
            Some(mut record) => {
                record.value.read_value(&mut self.reader);
                let value = record.value.into_value();
                Some(value.into_value())
            }
        }
    }

    pub fn get_detail<'a>(&mut self, key_str: &'a str) -> (KeyWithHash<'a>, bool, Vec<Record<U>>) {
        let key = self.hash(key_str.as_bytes());
        let (found, visited_records) = self.get_record_detail(&key);
        (key, found, visited_records)
    }

    pub fn dump_bucket(&mut self, bucket_number: u64) -> Vec<Record<U>> {
        let mut records = Vec::new();
        let rec_off = self.read_bucket(bucket_number);

        self.traverse_records(rec_off, &mut records);

        records
    }

    fn traverse_records(&mut self, rec_off: RecordOffset<U>, records: &mut Vec<Record<U>>) {
        if rec_off.is_empty() {
            return;
        }

        match self.read_record_space(rec_off, false) {
            RecordSpace::FreeBlock(_) => panic!(
                "unexpected freespace found: {}",
                rec_off.offset(self.header.alignment_power)
            ),
            RecordSpace::Record(record) => {
                let right = record.right_chain;
                let left = record.left_chain;
                self.traverse_records(right, records);
                records.push(record);
                self.traverse_records(left, records);
            }
        }
    }
}

pub struct RecordSpaceIter<'a, U, R> {
    reader: &'a mut R,
    pv: bool,
    endian: Endian,
    file_size: u64,
    next_pos: u64,
    bucket_type: PhantomData<fn() -> U>,
}

impl<'a, U, R: Seek> RecordSpaceIter<'a, U, R> {
    fn new(reader: &'a mut R, pv: bool, endian: Endian, header: &Header) -> Self {
        RecordSpaceIter {
            reader,
            pv,
            endian,
            file_size: header.file_size,
            next_pos: header.first_record,
            bucket_type: PhantomData,
        }
    }
}

impl<'a, U: U32orU64, R: Read + Seek> Iterator for RecordSpaceIter<'a, U, R> {
    type Item = RecordSpace<U>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_pos >= self.file_size {
            return None;
        }

        self.reader.seek(SeekFrom::Start(self.next_pos)).unwrap();
        match self
            .reader
            .read_type_args(self.endian, (self.next_pos, self.pv))
            .unwrap()
        {
            RecordSpace::FreeBlock(free_block) => {
                self.next_pos = self.reader.stream_position().unwrap();
                Some(RecordSpace::FreeBlock(free_block))
            }
            RecordSpace::Record(record) => {
                self.next_pos = record.next_record();
                Some(RecordSpace::Record(record))
            }
        }
    }
}
