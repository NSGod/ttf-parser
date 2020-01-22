use core::ops::Range;

use crate::{Error, Result};

pub trait FromData: Sized {
    /// Stores an object size in raw data.
    ///
    /// `mem::size_of` by default.
    ///
    /// Override when size of `Self` != size of a raw data.
    /// For example, when you are parsing `u16`, but storing it as `u8`.
    /// In this case `size_of::<Self>()` == 1, but `FromData::SIZE` == 2.
    const SIZE: usize = core::mem::size_of::<Self>();

    /// Parses an object from a raw data.
    ///
    /// This method **must** not panic and **must** not read past the bounds.
    fn parse(data: &[u8]) -> Self;
}

impl FromData for u8 {
    #[inline]
    fn parse(data: &[u8]) -> Self {
        data[0]
    }
}

impl FromData for i8 {
    #[inline]
    fn parse(data: &[u8]) -> Self {
        data[0] as i8
    }
}

impl FromData for u16 {
    #[inline]
    fn parse(data: &[u8]) -> Self {
        u16::from_be_bytes([data[0], data[1]])
    }
}

impl FromData for i16 {
    #[inline]
    fn parse(data: &[u8]) -> Self {
        i16::from_be_bytes([data[0], data[1]])
    }
}

impl FromData for u32 {
    #[inline]
    fn parse(data: &[u8]) -> Self {
        // For u32 it's faster to use TryInto, but for u16/i16 it's faster to index.
        use core::convert::TryInto;
        u32::from_be_bytes(data.try_into().unwrap())
    }
}

impl FromData for i32 {
    #[inline]
    fn parse(data: &[u8]) -> Self {
        // For i32 it's faster to use TryInto, but for u16/i16 it's faster to index.
        use core::convert::TryInto;
        i32::from_be_bytes(data.try_into().unwrap())
    }
}


// https://docs.microsoft.com/en-us/typography/opentype/spec/otff#data-types
#[derive(Clone, Copy, Debug)]
pub struct U24(pub u32);

impl FromData for U24 {
    const SIZE: usize = 3;

    #[inline]
    fn parse(data: &[u8]) -> Self {
        U24((data[0] as u32) << 16 | (data[1] as u32) << 8 | data[2] as u32)
    }
}


// https://docs.microsoft.com/en-us/typography/opentype/spec/otff#data-types
#[derive(Clone, Copy, Debug)]
pub struct F2DOT14(pub f32);

impl FromData for F2DOT14 {
    const SIZE: usize = 2;

    #[inline]
    fn parse(data: &[u8]) -> Self {
        F2DOT14(i16::parse(data) as f32 / 16384.0)
    }
}


// https://docs.microsoft.com/en-us/typography/opentype/spec/otff#data-types
#[derive(Clone, Copy, Debug)]
pub struct Fixed(pub f32);

impl FromData for Fixed {
    const SIZE: usize = 4;

    #[inline]
    fn parse(data: &[u8]) -> Self {
        Fixed(i32::parse(data) as f32 / 65536.0)
    }
}


pub trait TryFromData: Sized {
    /// Stores an object size in raw data.
    ///
    /// `mem::size_of` by default.
    ///
    /// Override when size of `Self` != size of a raw data.
    /// For example, when you are parsing `u16`, but storing it as `u8`.
    /// In this case `size_of::<Self>()` == 1, but `FromData::SIZE` == 2.
    const SIZE: usize = core::mem::size_of::<Self>();

    /// Parses an object from a raw data.
    fn try_parse(data: &[u8]) -> Result<Self>;
}


// Like `usize`, but for font.
pub trait FSize {
    fn to_usize(&self) -> usize;
}

impl FSize for u16 {
    #[inline]
    fn to_usize(&self) -> usize { *self as usize }
}

impl FSize for u32 {
    #[inline]
    fn to_usize(&self) -> usize { *self as usize }
}


#[derive(Clone, Copy)]
pub struct LazyArray<'a, T> {
    data: &'a [u8],
    phantom: core::marker::PhantomData<T>,
}

impl<'a, T: FromData> LazyArray<'a, T> {
    #[inline]
    pub fn new(data: &'a [u8]) -> Self {
        LazyArray {
            data,
            phantom: core::marker::PhantomData,
        }
    }

    pub fn at<L: FSize>(&self, index: L) -> T {
        let start = index.to_usize() * T::SIZE;
        let end = start + T::SIZE;
        T::parse(&self.data[start..end])
    }

    pub fn get<L: FSize>(&self, index: L) -> Option<T> {
        if index.to_usize() < self.len() {
            let start = index.to_usize() * T::SIZE;
            let end = start + T::SIZE;
            Some(T::parse(&self.data[start..end]))
        } else {
            None
        }
    }

    #[inline]
    pub fn last(&self) -> Option<T> {
        if !self.is_empty() {
            self.get(self.len() as u32 - 1)
        } else {
            None
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.data.len() / T::SIZE
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn binary_search(&self, x: &T) -> Option<T>
        where T: Ord
    {
        self.binary_search_by(|p| p.cmp(x))
    }

    #[inline]
    pub fn binary_search_by<F>(&self, mut f: F) -> Option<T>
        where F: FnMut(&T) -> core::cmp::Ordering
    {
        // Based on Rust std implementation.

        use core::cmp::Ordering;

        let mut size = self.len() as u32;
        if size == 0 {
            return None;
        }

        let mut base = 0;
        while size > 1 {
            let half = size / 2;
            let mid = base + half;
            // mid is always in [0, size), that means mid is >= 0 and < size.
            // mid >= 0: by definition
            // mid < size: mid = size / 2 + size / 4 + size / 8 ...
            let cmp = f(&self.at(mid));
            base = if cmp == Ordering::Greater { base } else { mid };
            size -= half;
        }

        // base is always in [0, size) because base <= mid.
        let value = self.at(base);
        let cmp = f(&value);
        if cmp == Ordering::Equal { Some(value) } else { None }
    }
}

impl<'a, T: FromData + core::fmt::Debug + Copy> core::fmt::Debug for LazyArray<'a, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_list().entries(self.into_iter()).finish()
    }
}

impl<'a, T: FromData> IntoIterator for LazyArray<'a, T> {
    type Item = T;
    type IntoIter = LazyArrayIter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        LazyArrayIter {
            data: self,
            index: 0,
        }
    }
}


pub struct LazyArrayIter<'a, T> {
    data: LazyArray<'a, T>,
    index: u32,
}

impl<'a, T: FromData> Iterator for LazyArrayIter<'a, T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.index += 1;
        self.data.get(self.index - 1)
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.data.get(n as u32)
    }
}


pub trait TrySlice<'a> {
    fn try_slice(&self, range: Range<usize>) -> Result<&'a [u8]>;
    fn try_slice_from<T: Offset>(&self, start: T) -> Result<&'a [u8]>;
}

impl<'a> TrySlice<'a> for &'a [u8] {
    #[inline]
    fn try_slice(&self, range: Range<usize>) -> Result<&'a [u8]> {
        self.get(range.clone()).ok_or_else(|| Error::SliceOutOfBounds)
    }

    #[inline]
    fn try_slice_from<T: Offset>(&self, start: T) -> Result<&'a [u8]> {
        self.get(start.to_usize()..).ok_or_else(|| Error::SliceOutOfBounds)
    }
}


#[derive(Clone, Copy)]
pub struct Stream<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Stream<'a> {
    #[inline]
    pub fn new(data: &'a [u8]) -> Self {
        Stream {
            data,
            offset: 0,
        }
    }

    #[inline]
    pub fn new_at(data: &'a [u8], offset: usize) -> Self {
        Stream {
            data,
            offset,
        }
    }

    #[inline]
    pub fn at_end(&self) -> bool {
        self.offset == self.data.len()
    }

    #[inline]
    pub fn offset(&self) -> usize {
        self.offset
    }

    #[inline]
    pub fn tail(&self) -> Result<&'a [u8]> {
        self.data.try_slice(self.offset..self.data.len())
    }

    #[inline]
    pub fn skip<T: FromData>(&mut self) {
        self.offset += T::SIZE;
    }

    #[inline]
    pub fn advance<L: FSize>(&mut self, len: L) {
        self.offset += len.to_usize();
    }

    #[inline]
    pub fn read<T: FromData>(&mut self) -> Result<T> {
        let start = self.offset;
        self.offset += T::SIZE;
        let end = self.offset;

        let data = self.data.try_slice(start..end)?;
        Ok(T::parse(data))
    }

    #[inline]
    pub fn try_read<T: TryFromData>(&mut self) -> Result<T> {
        let start = self.offset;
        self.offset += T::SIZE;
        let end = self.offset;

        let data = self.data.try_slice(start..end)?;
        T::try_parse(data)
    }

    #[inline]
    pub fn read_at<T: FromData>(data: &[u8], mut offset: usize) -> Result<T> {
        let start = offset;
        offset += T::SIZE;
        let end = offset;

        let data = data.try_slice(start..end)?;
        Ok(T::parse(data))
    }

    #[inline]
    pub fn read_bytes<L: FSize>(&mut self, len: L) -> Result<&'a [u8]> {
        let offset = self.offset;
        self.offset += len.to_usize();
        self.data.try_slice(offset..(offset + len.to_usize()))
    }

    #[inline]
    pub fn read_array<T: FromData, L: FSize>(&mut self, len: L) -> Result<LazyArray<'a, T>> {
        let len = len.to_usize() * T::SIZE;
        let data = self.read_bytes(len as u32)?;
        Ok(LazyArray::new(data))
    }

    #[inline]
    pub fn read_array16<T: FromData>(&mut self) -> Result<LazyArray<'a, T>> {
        let count: u16 = self.read()?;
        self.read_array(count)
    }

    #[inline]
    pub fn read_array32<T: FromData>(&mut self) -> Result<LazyArray<'a, T>> {
        let count: u32 = self.read()?;
        self.read_array(count)
    }
}


/// A "safe" stream.
///
/// Unlike `Stream`, `SafeStream` doesn't perform bounds checking on each read.
/// It leverages the type system, so we can sort of guarantee that
/// we do not read past the bounds.
///
/// For example, if we are iterating a `LazyArray` we already checked it's size
/// and we can't read past the bounds, so we can remove useless checks.
///
/// It's still not 100% guarantee, but it makes code easier to read and a bit faster.
/// And we still backed by the Rust's bounds checking.
#[derive(Clone, Copy)]
pub struct SafeStream<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> SafeStream<'a> {
    #[inline]
    pub fn new(data: &'a [u8]) -> Self {
        SafeStream {
            data,
            offset: 0,
        }
    }

    #[inline]
    pub fn read<T: FromData>(&mut self) -> T {
        T::parse(self.read_bytes(T::SIZE as u32))
    }

    #[inline]
    pub fn read_bytes<L: FSize>(&mut self, len: L) -> &'a [u8] {
        let offset = self.offset;
        self.offset += len.to_usize();
        &self.data[offset..(offset + len.to_usize())]
    }
}


pub trait Offset {
    fn to_usize(&self) -> usize;
    fn is_null(&self) -> bool { self.to_usize() == 0 }
}


#[derive(Clone, Copy, Debug)]
pub struct Offset16(pub u16);

impl Offset for Offset16 {
    fn to_usize(&self) -> usize {
        self.0 as usize
    }
}

impl FromData for Offset16 {
    #[inline]
    fn parse(data: &[u8]) -> Self {
        Offset16(SafeStream::new(data).read())
    }
}

impl FromData for Option<Offset16> {
    const SIZE: usize = Offset16::SIZE;

    #[inline]
    fn parse(data: &[u8]) -> Self {
        let offset = Offset16::parse(data);
        if offset.0 != 0 { Some(offset) } else { None }
    }
}


#[derive(Clone, Copy, Debug)]
pub struct Offset32(pub u32);

impl Offset for Offset32 {
    fn to_usize(&self) -> usize {
        self.0 as usize
    }
}

impl FromData for Offset32 {
    #[inline]
    fn parse(data: &[u8]) -> Self {
        Offset32(SafeStream::new(data).read())
    }
}

impl FromData for Option<Offset32> {
    const SIZE: usize = Offset32::SIZE;

    #[inline]
    fn parse(data: &[u8]) -> Self {
        let offset = Offset32::parse(data);
        if offset.0 != 0 { Some(offset) } else { None }
    }
}


/// Array of offsets from beginning of `data`.
#[derive(Clone, Copy)]
pub struct Offsets<'a, T: Offset> {
    data: &'a [u8],
    offsets: LazyArray<'a, T>, // [Offset16/Offset32]
}
