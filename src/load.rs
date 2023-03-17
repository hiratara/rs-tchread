use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use binrw::{io::BufReader, BinReaderExt, Endian};

use crate::{binrw_types::Header, TCHDB};

pub enum TCHDBLoader<R> {
    Small(TCHDB<u32, R>),
    Large(TCHDB<u64, R>),
}

impl TCHDBLoader<BufReader<File>> {
    pub fn open_with_endian<T>(path: T, endian: Endian) -> Self
    where
        T: AsRef<Path>,
    {
        let file = File::open(path).unwrap();
        let file = BufReader::new(file);
        TCHDBLoader::load(file, endian)
    }

    pub fn open<T>(path: T) -> Self
    where
        T: AsRef<Path>,
    {
        Self::open_with_endian(path, Endian::Little)
    }
}

impl<R> TCHDBLoader<R>
where
    R: Read + Seek,
{
    pub fn load(mut reader: R, endian: Endian) -> Self {
        reader.seek(SeekFrom::Start(0)).unwrap();
        let header: Header = reader.read_type(endian).unwrap();

        if header.options & 0x01 == 0x01 {
            TCHDBLoader::Large(TCHDB::new(reader, endian, header))
        } else {
            TCHDBLoader::Small(TCHDB::new(reader, endian, header))
        }
    }
}
