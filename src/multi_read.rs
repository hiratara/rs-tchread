use std::{
    cell::RefCell,
    io::{Read, Seek, SeekFrom},
    marker::PhantomData,
    rc::Rc,
};

use binrw::{BinRead, BinReaderExt, Endian};

use super::{Header, RecordSpace, TCHDBImpl};

pub struct MultiRead<R>(Rc<RefCell<R>>);

impl<R> MultiRead<R> {
    pub fn new(reader: R) -> Self {
        MultiRead(Rc::new(RefCell::new(reader)))
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

impl<B, R: Clone> TCHDBImpl<B, R> {
    pub fn read_record_spaces_multi(&mut self) -> RecordSpaceMultiIter<R, B> {
        RecordSpaceMultiIter::new(self.reader.clone(), self.endian, &self.header)
    }
}
pub struct RecordSpaceMultiIter<R, B> {
    reader: R,
    endian: Endian,
    file_size: u64,
    alignment_power: u8,
    next_pos: u64,
    _bucket_type: PhantomData<B>,
}

impl<R, B> RecordSpaceMultiIter<R, B> {
    fn new(reader: R, endian: Endian, header: &Header) -> Self {
        RecordSpaceMultiIter {
            reader,
            endian,
            file_size: header.file_size,
            alignment_power: header.alignment_power,
            next_pos: header.first_record,
            _bucket_type: PhantomData,
        }
    }
}

impl<R, B> Iterator for RecordSpaceMultiIter<R, B>
where
    R: Read + Seek,
    B: BinRead,
    <B as BinRead>::Args<'static>: Default,
{
    type Item = RecordSpace<B>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_pos >= self.file_size {
            return None;
        }

        self.reader.seek(SeekFrom::Start(self.next_pos)).unwrap();
        match self
            .reader
            .read_type_args(self.endian, (self.next_pos, self.alignment_power))
            .unwrap()
        {
            RecordSpace::FreeBlock(free_block) => {
                self.next_pos = self.reader.stream_position().unwrap();
                Some(RecordSpace::FreeBlock(free_block))
            }
            RecordSpace::Record(record) => {
                self.next_pos = record.next_record;
                Some(RecordSpace::Record(record))
            }
        }
    }
}
