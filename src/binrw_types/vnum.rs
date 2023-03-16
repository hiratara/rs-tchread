use std::{
    io::{Read, Seek},
    ops::{Add, AddAssign, Mul, ShlAssign, Sub},
};

use binrw::{BinRead, BinResult, Endian};

#[derive(Debug)]
pub struct VNum<T> {
    pub value: T,
    pub size: u32,
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
        let mut length = 0;

        loop {
            length += 1;
            let x = T::from(<u8>::read_options(reader, endian, args)?);
            if x < T::from(0x80) {
                value += x * base;
                break;
            }
            value += base * (T::from(0xFF) - x);
            base <<= 7;
        }

        Ok(VNum {
            value,
            size: length,
        })
    }
}
