use std::io::{Read, Seek, SeekFrom};

use binrw::{BinRead, BinResult, Endian, NamedArgs};

#[derive(NamedArgs, Clone)]
pub struct LazyArgs<Inner> {
    pub lazy: bool,
    pub inner: Inner,
}

#[derive(Debug)]
pub enum Lazy<T, A> {
    Unread {
        offset: u64,
        endian: Endian,
        args: A,
    },
    Read(T),
}

impl<T, A> BinRead for Lazy<T, A>
where
    for<'a> T: BinRead<Args<'a> = A>,
    for<'a> T::Args<'a>: Default,
{
    type Args<'a> = LazyArgs<T::Args<'a>>;

    #[inline]
    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: Endian,
        args: Self::Args<'_>,
    ) -> BinResult<Self> {
        if args.lazy {
            let offset = reader.stream_position()?;
            Ok(Lazy::Unread {
                offset,
                endian,
                args: args.inner,
            })
        } else {
            let value = <T>::read_options(reader, endian, args.inner)?;
            Ok(Lazy::Read(value))
        }
    }
}

impl<T, A> Lazy<T, A>
where
    for<'a> T: BinRead<Args<'a> = A>,
    for<'a> T::Args<'a>: Default,
    A: Copy,
{
    pub fn read_value<R: Read + Seek>(&mut self, reader: &mut R) {
        if let Lazy::Unread {
            offset,
            endian,
            args,
        } = self
        {
            reader.seek(SeekFrom::Start(*offset)).unwrap();
            let value = <T>::read_options(reader, *endian, *args).unwrap();
            *self = Lazy::Read(value);
        }
    }

    pub fn into_value(self) -> T {
        match self {
            Lazy::Read(value) => value,
            Lazy::Unread { .. } => panic!("must to call read_record_value first"),
        }
    }
}
