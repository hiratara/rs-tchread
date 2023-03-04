use std::{
    cell::RefCell,
    io::{Read, Seek, SeekFrom},
    rc::Rc,
};

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
