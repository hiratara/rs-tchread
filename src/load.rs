use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use binrw::{io::BufReader, BinReaderExt, Endian};

use crate::{binrw_types::Header, TCHDB};

pub enum TCHDBLoaded<R> {
    Small(TCHDB<u32, R>),
    Large(TCHDB<u64, R>),
}

pub fn open_with_endian<T>(path: T, endian: Endian) -> TCHDBLoaded<BufReader<File>>
where
    T: AsRef<Path>,
{
    let file = File::open(path).unwrap();
    let file = BufReader::new(file);
    load_with_endian(file, endian)
}

pub fn open<T>(path: T) -> TCHDBLoaded<BufReader<File>>
where
    T: AsRef<Path>,
{
    open_with_endian(path, Endian::Little)
}

pub fn load_with_endian<R: Read + Seek>(mut reader: R, endian: Endian) -> TCHDBLoaded<R> {
    reader.seek(SeekFrom::Start(0)).unwrap();
    let header: Header = reader.read_type(endian).unwrap();

    if header.options & 0x01 == 0x01 {
        TCHDBLoaded::Large(TCHDB::new(reader, endian, header))
    } else {
        TCHDBLoaded::Small(TCHDB::new(reader, endian, header))
    }
}
