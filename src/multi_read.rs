use std::{
    cell::RefCell,
    io::{Read, Seek, SeekFrom},
    marker::PhantomData,
    rc::Rc,
};

use binrw::{BinReaderExt, Endian};

use crate::binrw_types::U32orU64;

use super::{Header, RecordSpace, TCHDB};

pub struct MultiRead<R>(Rc<RefCell<R>>);

impl<R> MultiRead<R> {
    pub fn new(reader: R) -> Self {
        MultiRead(Rc::new(RefCell::new(reader)))
    }

    pub fn into_inner(self) -> R {
        match Rc::try_unwrap(self.0) {
            Ok(refcell) => refcell.into_inner(),
            Err(_) => panic!("failed to unwrap MultiRead"),
        }
    }
}

impl<R: Read> Read for MultiRead<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().read(buf)
    }
}

impl<R: Seek> Seek for MultiRead<R> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.0.borrow_mut().seek(pos)
    }
}

impl<R> Clone for MultiRead<R> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<U, R> TCHDB<U, R> {
    pub fn into_multi(self) -> TCHDB<U, MultiRead<R>> {
        TCHDB {
            reader: MultiRead::new(self.reader),
            endian: self.endian,
            header: self.header,
            bucket_offset: self.bucket_offset,
            free_block_pool_offset: self.free_block_pool_offset,
            bucket_type: self.bucket_type,
        }
    }
}

impl<U, R> TCHDB<U, MultiRead<R>> {
    pub fn into_inner(self) -> TCHDB<U, R> {
        TCHDB {
            reader: self.reader.into_inner(),
            endian: self.endian,
            header: self.header,
            bucket_offset: self.bucket_offset,
            free_block_pool_offset: self.free_block_pool_offset,
            bucket_type: self.bucket_type,
        }
    }
}

impl<U, R: Clone> TCHDB<U, R> {
    pub fn read_record_spaces_multi(&mut self) -> RecordSpaceMultiIter<U, R> {
        RecordSpaceMultiIter::new(self.reader.clone(), self.endian, &self.header)
    }
}
pub struct RecordSpaceMultiIter<U, R> {
    reader: R,
    endian: Endian,
    file_size: u64,
    next_pos: u64,
    _bucket_type: PhantomData<U>,
}

impl<U, R> RecordSpaceMultiIter<U, R> {
    fn new(reader: R, endian: Endian, header: &Header) -> Self {
        RecordSpaceMultiIter {
            reader,
            endian,
            file_size: header.file_size,
            next_pos: header.first_record,
            _bucket_type: PhantomData,
        }
    }
}

impl<U: U32orU64, R: Read + Seek> Iterator for RecordSpaceMultiIter<U, R> {
    type Item = RecordSpace<U>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_pos >= self.file_size {
            return None;
        }

        self.reader.seek(SeekFrom::Start(self.next_pos)).unwrap();
        match self
            .reader
            .read_type_args(self.endian, (self.next_pos, false))
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
