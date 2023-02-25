use std::{
    io::{Read, Seek},
    ops::{Add, AddAssign, Mul, ShlAssign, Sub},
};

use binrw::{BinRead, BinResult, Endian};

#[derive(Debug)]
pub struct VNum<T>(pub T);

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
        let mut num = T::from(0);
        let mut base = T::from(1);

        loop {
            let x = T::from(<u8>::read_options(reader, endian, args)?);
            if x < T::from(0xA0) {
                num += x * base;
                break;
            }
            num += base * (T::from(0xFF) - x);
            base <<= 7;
        }

        Ok(VNum(num))
    }
}
