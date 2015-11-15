extern crate byteorder;

mod slice;
mod vorbis;

use std::mem;
use std::ops;
use std::borrow::{Borrow, BorrowMut, ToOwned};
use std::io::{Cursor, BufRead};
use std::marker::PhantomData;
use std::borrow::Cow;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use byteorder::Error as ByteOrderError;
use slice::Slice;

const OGG_PAGE_CAPTURE: &'static [u8] = b"OggS";
const POSITION_OFFSET: usize = 6;
const SERIAL_OFFSET: usize = 14;

#[derive(Debug)]
pub enum OggPageCheckError {
    TooShort,
    BadCapture,
    BadVersion,
    BadCrc,
}

#[derive(Clone)]
pub struct OggTrackBuf {
    inner: Vec<u8>,
}

pub struct OggTrack {
    inner: Slice,
}

impl ops::Deref for OggTrackBuf {
    type Target = OggTrack;

    fn deref<'a>(&'a self) -> &'a OggTrack {
        OggTrack::from_u8_slice_unchecked(&self.inner)
    }
}

impl Borrow<OggTrack> for OggTrackBuf {
    fn borrow(&self) -> &OggTrack {
        OggTrack::from_u8_slice_unchecked(&self.inner)
    }
}

impl AsRef<OggTrack> for OggTrackBuf {
    fn as_ref(&self) -> &OggTrack {
        OggTrack::from_u8_slice_unchecked(&self.inner)
    }
}

impl AsMut<OggTrack> for OggTrackBuf {
    fn as_mut(&mut self) -> &mut OggTrack {
        OggTrack::from_u8_slice_unchecked_mut(&mut self.inner)
    }
}

impl BorrowMut<OggTrack> for OggTrackBuf {
    fn borrow_mut(&mut self) -> &mut OggTrack {
        OggTrack::from_u8_slice_unchecked_mut(&mut self.inner)
    }
}

impl ToOwned for OggTrack {
    type Owned = OggTrackBuf;

    fn to_owned(&self) -> OggTrackBuf {
        OggTrackBuf { inner: self.inner.to_owned() }
    }
}

impl OggTrackBuf {
    pub fn new(buf: Vec<u8>) -> Result<OggTrackBuf, OggPageCheckError> {
        try!(OggTrack::new(&buf));
        Ok(OggTrackBuf { inner: buf })
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.inner
    }
}

impl OggTrack {
    pub fn new(buf: &[u8]) -> Result<&OggTrack, OggPageCheckError>  {
        let mut offset = 0;
        while offset < buf.len() {
            let page = try!(OggPage::new(&buf[offset..]));
            offset += page.as_u8_slice().len();
        }
        Ok(OggTrack::from_u8_slice_unchecked(buf))
    }

    /// The following function allows unchecked construction of a ogg track
    /// from a u8 slice.  This is private because it does not maintain
    /// the OggPage invariant.
    fn from_u8_slice_unchecked(s: &[u8]) -> &OggTrack {
        unsafe { mem::transmute(s) }
    }

    /// The following (private!) function allows unchecked construction of a
    /// mutable ogg page from a mutable u8 slice.  This is private because it
    /// does not maintain the OggPage invariant.
    fn from_u8_slice_unchecked_mut(s: &mut [u8]) -> &mut OggTrack {
        unsafe { mem::transmute(s) }
    }

    pub fn as_u8_slice(&self) -> &[u8] {
        &self.inner
    }

    pub fn pages(&self) -> TrackPageIter {
        TrackPageIter {
            data: self.as_u8_slice(),
            offset: 0,
        }
    }

    pub fn pages_mut(&self) -> TrackPageIterMut {
        // TODO: impl
        unimplemented!();
    }
}

pub struct TrackPageIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for TrackPageIter<'a> {
    type Item = &'a OggPage;

    fn next(&mut self) -> Option<&'a OggPage> {
        if self.data.len() == self.offset {
            return None;
        }
        let page = OggPage::new(&self.data[self.offset..]).unwrap();
        self.offset += page.as_u8_slice().len();
        Some(page)
    }
}

// TODO: iterate &mut OggPage
pub struct TrackPageIterMut<'a> {
    data: &'a mut [u8],
    offset: usize,
}

#[derive(Clone)]
pub struct OggPageBuf {
    inner: Vec<u8>,
}

pub struct OggPage {
    inner: Slice,
}

impl AsRef<OggPage> for OggPageBuf {
    fn as_ref(&self) -> &OggPage {
        OggPage::from_u8_slice_unchecked(&self.inner)
    }
}

impl AsMut<OggPage> for OggPageBuf {
    fn as_mut(&mut self) -> &mut OggPage {
        OggPage::from_u8_slice_unchecked_mut(&mut self.inner)
    }
}

impl ops::Deref for OggPageBuf {
    type Target = OggPage;

    fn deref<'a>(&'a self) -> &'a OggPage {
        OggPage::from_u8_slice_unchecked(&self.inner)
    }
}

impl Borrow<OggPage> for OggPageBuf {
    fn borrow(&self) -> &OggPage {
        OggPage::from_u8_slice_unchecked(&self.inner)
    }
}

impl BorrowMut<OggPage> for OggPageBuf {
    fn borrow_mut(&mut self) -> &mut OggPage {
        OggPage::from_u8_slice_unchecked_mut(&mut self.inner)
    }
}

impl ToOwned for OggPage {
    type Owned = OggPageBuf;

    fn to_owned(&self) -> OggPageBuf {
        OggPageBuf { inner: self.inner.to_owned() }
    }
}

impl OggPageBuf {
    pub fn new(mut buf: Vec<u8>) -> Result<OggPageBuf, OggPageCheckError>  {
        let slice_len = try!(OggPage::measure_whole(&buf)).len();
        buf.truncate(slice_len);
        Ok(OggPageBuf { inner: buf })
    }

    pub fn into_cow(self) -> Cow<'static, OggPage> {
        Cow::Owned(self)
    }
}

impl OggPage {
    /// The following function allows unchecked construction of a ogg page
    /// from a u8 slice.  This is private because it does not maintain
    /// the OggPage invariant.
    fn from_u8_slice_unchecked(s: &[u8]) -> &OggPage {
        unsafe { mem::transmute(s) }
    }

    /// The following (private!) function allows unchecked construction of a
    /// mutable ogg page from a mutable u8 slice.  This is private because it
    /// does not maintain the OggPage invariant.
    fn from_u8_slice_unchecked_mut(s: &mut [u8]) -> &mut OggPage {
        unsafe { mem::transmute(s) }
    }

    pub fn as_u8_slice(&self) -> &[u8] {
        unsafe { mem::transmute(self) }
    }

    /// Mutably borrow the underlying storage.  This is private because it
    /// does not maintain the OggPage invariant.
    fn as_u8_slice_mut(&mut self) -> &mut [u8] {
        unsafe { mem::transmute(self) }
    }

    pub fn new(buf: &[u8]) -> Result<&OggPage, OggPageCheckError> {
        let buffer = try!(OggPage::measure_whole(buf));
        Ok(OggPage::from_u8_slice_unchecked(buffer))
    }

    pub fn new_mut(buf: &mut [u8]) -> Result<&mut OggPage, OggPageCheckError> {
        let page_length = {
            let (hbuf, bbuf) = try!(OggPage::measure(buf));
            hbuf.len() + bbuf.len()
        };
        Ok(OggPage::from_u8_slice_unchecked_mut(&mut buf[0..page_length]))
    }

    fn measure(buf: &[u8]) -> Result<(&[u8], &[u8]), OggPageCheckError> {
        impl From<ByteOrderError> for OggPageCheckError {
            fn from(e: ByteOrderError) -> OggPageCheckError {
                match e {
                    ByteOrderError::UnexpectedEOF => OggPageCheckError::TooShort,
                    ByteOrderError::Io(_) => panic!("no I/O allowed"),
                }
            }
        }

        let mut cursor = Cursor::new(buf);

        if buf.len() < 27 {
            return Err(OggPageCheckError::TooShort);
        }
        if &buf[0..4] != OGG_PAGE_CAPTURE {
            return Err(OggPageCheckError::BadCapture);
        }

        cursor.consume(4);  // capture sequence
        if try!(cursor.read_u8()) != 0 {
            return Err(OggPageCheckError::BadVersion);
        }

        // flags(1) + granule(8) + serial(4) + page_seq(4) + csum(4)
        cursor.consume(1 + 8 + 4 + 4 + 4);

        let page_segments = try!(cursor.read_u8());

        let mut body_len = 0;
        for _ in 0..page_segments {
            body_len += try!(cursor.read_u8()) as u64;
        }

        let total_len = (cursor.position() + body_len) as usize;
        if buf.len() < total_len {
            return Err(OggPageCheckError::TooShort);
        }

        let h_end = cursor.position() as usize;
        let b_end = h_end + body_len as usize;
        Ok((
            &buf[0..h_end],
            &buf[h_end..b_end],
        ))
    }

    fn measure_whole(buf: &[u8]) -> Result<&[u8], OggPageCheckError> {
        let page_length = {
            let (h_buf, b_buf) = try!(OggPage::measure(buf));
            h_buf.len() + b_buf.len()
        };
        Ok(&buf[0..page_length])
    }

    fn measure_whole_mut(buf: &mut [u8]) -> Result<&mut [u8], OggPageCheckError> {
        let page_length = {
            let (hbuf, bbuf) = try!(OggPage::measure(buf));
            hbuf.len() + bbuf.len()
        };
        Ok(&mut buf[0..page_length])
    }

    pub fn position(&self) -> u64 {
        let self_buf = self.as_u8_slice();
        let mut cur = Cursor::new(&self_buf[POSITION_OFFSET..POSITION_OFFSET+8]);
        cur.read_u64::<LittleEndian>().unwrap()
    }

    pub fn set_position(&mut self, granule: u64) {
        let mut tx = self.begin();
        tx.set_position(granule);
    }

    pub fn serial(&self) -> u32 {
        let self_buf = self.as_u8_slice();
        let mut cur = Cursor::new(&self_buf[SERIAL_OFFSET..SERIAL_OFFSET+4]);
        cur.read_u32::<LittleEndian>().unwrap()
    }

    pub fn set_serial(&mut self, serial: u32) {
        let mut tx = self.begin();
        tx.set_serial(serial);
    }

    fn recompute_checksum(&mut self) {
        unimplemented!();
    }

    pub fn begin<'a>(&'a mut self) -> ChecksumGuard<'a> {
        ChecksumGuard {
            page: self,
            _marker: PhantomData,
        }
    }

    /// Am iterator of packet slices
    pub fn packets<'a>(&'a self) -> Packets<'a> {
        Packets { page: &self, packet: 0 }
    }

    pub fn into_cow<'a>(&'a self) -> Cow<'a, OggPage> {
        Cow::Borrowed(self)
    }
}

pub struct Packets<'a> {
    page: &'a OggPage,
    packet: usize,
}

impl<'a> Iterator for Packets<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        let packet_slice = self.page;
        unimplemented!();
    }
}

pub struct ChecksumGuard<'a> {
    page: &'a mut OggPage,
    _marker: PhantomData<&'a ()>,
}

impl<'a> ChecksumGuard<'a> {
    pub fn set_position(&mut self, granule: u64) {
        let self_buf = self.page.as_u8_slice_mut();
        let mut cur = Cursor::new(&mut self_buf[POSITION_OFFSET..POSITION_OFFSET+8]);
        cur.write_u64::<LittleEndian>(granule).unwrap();
    }

    pub fn set_serial(&mut self, serial: u32) {
        let self_buf = self.page.as_u8_slice_mut();
        let mut cur = Cursor::new(&mut self_buf[SERIAL_OFFSET..SERIAL_OFFSET+4]);
        cur.write_u32::<LittleEndian>(serial).unwrap();
    }
}

impl<'a> Drop for ChecksumGuard<'a> {
    fn drop(&mut self) {
        self.page.recompute_checksum();
    }
}


pub struct Recapture([u8; 4]);

impl Recapture {
    pub fn new() -> Recapture {
        Recapture([0; 4])
    }

    pub fn push_byte(&mut self, byte: u8) {
        let mut buf = [0; 4];
        buf[0] = self.0[1];
        buf[1] = self.0[2];
        buf[2] = self.0[3];
        buf[3] = byte;
        *self = Recapture(buf);
    }

    pub fn is_captured(&self) -> bool {
        &self.0 == OGG_PAGE_CAPTURE
    }
}

#[cfg(test)]
mod tests {
    use super::Recapture;

    #[test]
    fn test_capture() {
        let mut cap = Recapture::new();
        cap.push_byte(b'O');
        assert_eq!(false, cap.is_captured());
        cap.push_byte(b'g');
        assert_eq!(false, cap.is_captured());
        cap.push_byte(b'g');
        assert_eq!(false, cap.is_captured());
        cap.push_byte(b'S');
        assert_eq!(true, cap.is_captured());
        cap.push_byte(b'g');
        assert_eq!(false, cap.is_captured());
        cap.push_byte(b'g');
        assert_eq!(false, cap.is_captured());
        cap.push_byte(b'S');
        assert_eq!(false, cap.is_captured());
        cap.push_byte(b'O');
        assert_eq!(false, cap.is_captured());
        cap.push_byte(b'g');
        assert_eq!(false, cap.is_captured());
        cap.push_byte(b'g');
        assert_eq!(false, cap.is_captured());
        cap.push_byte(b'S');
        assert_eq!(true, cap.is_captured());
    }
}
