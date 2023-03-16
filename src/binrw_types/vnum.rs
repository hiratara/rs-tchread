use std::{
    io::{Read, Seek},
    ops::{Add, AddAssign, Mul, ShlAssign, ShrAssign, Sub},
};

use binrw::{BinRead, BinResult, Endian};

#[derive(Debug)]
pub struct VNum<T>(pub T);

impl<T> VNum<T>
where
    T: ShrAssign<i32> + Eq + From<u32> + Copy,
{
    pub fn size(&self) -> u32 {
        let mut value = self.0;
        let mut size = 1;
        loop {
            value >>= 7;
            if value == From::from(0) {
                return size;
            }
            size += 1;
        }
    }
}

impl<T> BinRead for VNum<T>
where
    T: From<u8> + Ord + Add + Mul<Output = T> + Sub<Output = T> + ShlAssign<i32> + AddAssign + Copy,
{
    type Args<'a> = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: Endian,
        args: Self::Args<'_>,
    ) -> BinResult<Self> {
        let mut value = T::from(0);
        let mut base = T::from(1);

        loop {
            let x = T::from(<u8>::read_options(reader, endian, args)?);
            if x < T::from(0x80) {
                value += x * base;
                break;
            }
            value += base * (T::from(0xFF) - x);
            base <<= 7;
        }

        Ok(VNum(value))
    }
}
