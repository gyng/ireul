use std::marker::PhantomData;

use byteorder::{ByteOrder, ReadBytesExt};

pub enum Error {
    Truncated,
}

pub struct Reader<'a, B> where B: ByteOrder {
    buffer: &'a [u8],
    offset: usize,
    _marker: PhantomData<B>,
}

impl<'a, B> Reader<'a, B> where B: ByteOrder {
    pub fn new(buffer: &'a [u8]) -> Reader<'a, B> {
        Reader {
            buffer: buffer,
            offset: 0,
            _marker: PhantomData,
        }
    }

    pub fn read_u8(&mut self) -> Result<u8, Error> {
        if self.buffer[self.offset..].len() < 1 {
            return Err(Error::Truncated);
        }

        let value = self.buffer[self.offset..][0];
        self.offset += 1;
        Ok(value)
    }

    pub fn read_u32(&mut self) -> Result<u32, Error> {
        if self.buffer[self.offset..].len() < 4 {
            return Err(Error::Truncated);
        }

        let value = B::read_u32(&self.buffer[self.offset..]);
        self.offset += 4;
        Ok(value)
    }

    pub fn read_buffer(&mut self, len: usize) -> Result<&'a [u8], Error> {
        if self.buffer[self.offset..].len() < len {
            return Err(Error::Truncated);
        }
        let buf = &self.buffer[self.offset..][..len];
        self.offset += len;
        Ok(buf)
    }
}

