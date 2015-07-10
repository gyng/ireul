extern crate ogg;
extern crate mio;

use mio::buf::{RingBuf, MutBuf, Buf};
use ogg::{OggPage, OggPageBuf, OggPageCheckError};
use ogg::Recapture as OggPageRecapture;

pub enum PushError {
    Full,
}

#[derive(Debug)]
pub enum ProtocolError {
    /// Message is too long.
    TooLong,
}

pub enum PopError {
    /// More data is needed to pop an OggPage
    MoreData,

    /// Failed to parse an OggPage
    OggPageError(OggPageCheckError),
}

pub struct OggPageRingBuf(RingBuf);

impl OggPageRingBuf {
    pub fn new(capacity: usize) -> OggPageRingBuf {
        OggPageRingBuf(RingBuf::new(capacity))
    }

    pub fn is_full(&self) -> bool {
        self.0.is_full()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }
    
    pub fn mark(&mut self) {
        self.0.mark();
    }

    pub fn reset(&mut self) {
        self.0.reset();
    }

    pub fn bytes(&self) -> &[u8] {
        self.0.bytes()
    }

    pub fn resync(&mut self) -> Result<(), PopError> {
        let mut recap = OggPageRecapture::new();
        let mut is_captured = false;
        let mut consumed = 0;

        self.mark();
        while let Some(byte) = self.0.read_byte() {
            recap.push_byte(byte);
            consumed += 1;
            if recap.is_captured() {
                is_captured = true;
                break;
            }
        }
        self.reset();

        if !is_captured {
            return Err(PopError::MoreData);
        }

        // OggPageRecapture won't capture without consuming at least
        // four bytes.
        assert!(consumed > 4);

        // Advance to the beginning of the located page.
        Buf::advance(self, consumed - 4);
        Ok(())
    }

    pub fn pop_page(&mut self) -> Result<OggPageBuf, OggPageCheckError> {
        self.mark();
        let mut header_buf = [0; 512];
        // call this twice to handle non-contiguous reads?
        let _ = self.0.read_slice(&mut header_buf);
        self.reset();

        let (h_len, b_len) = try!(OggPage::measure(&header_buf));
        let page_size = (h_len + b_len) as usize;

        if Buf::remaining(self) < page_size {
            return Err(OggPageCheckError::TooShort);
        }

        let mut buf = vec![0; page_size];

        let mut bytes_read = 0;
        while bytes_read < page_size {
            bytes_read += self.read_slice(&mut buf[bytes_read..]);
        }

        Ok(OggPageBuf::new(buf).ok().expect("prechecked page failed checks"))
    }

    pub fn push_page(&mut self, page: &OggPage) -> Result<usize, PushError> {
        let buf = page.as_u8_slice();
        if MutBuf::remaining(&self.0) < buf.len() {
            return Err(PushError::Full);
        }

        let mut bytes_written = 0;
        while bytes_written < buf.len() {
            bytes_written += self.write_slice(&buf[bytes_written..]);
        }
        Ok(bytes_written)
    }
}

impl Buf for OggPageRingBuf {
    fn remaining(&self) -> usize {
        Buf::remaining(&self.0)
    }

    fn bytes(&self) -> &[u8] {
        Buf::bytes(&self.0)
    }

    fn advance(&mut self, cnt: usize) {
        Buf::advance(&mut self.0, cnt)
    }
}


impl MutBuf for OggPageRingBuf {
    fn remaining(&self) -> usize {
        MutBuf::remaining(&self.0)
    }

    fn advance(&mut self, cnt: usize) {
        MutBuf::advance(&mut self.0, cnt)
    }

    fn mut_bytes(&mut self) -> &mut [u8] {
        MutBuf::mut_bytes(&mut self.0)
    }
}
