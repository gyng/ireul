extern crate byteorder;

mod slice;

use std::mem;
use std::ops;
use std::borrow::{Borrow, BorrowMut, ToOwned};
use std::io::{Cursor, BufRead};
use std::marker::PhantomData;
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

pub struct OggPageBuf {
	inner: Vec<u8>,
}

pub struct OggPage {
	inner: Slice,
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
		let (h_len, b_len) = try!(OggPage::measure(&buf));
        buf.truncate((h_len + b_len) as usize);
        Ok(OggPageBuf { inner: buf })
	}
}


impl OggPage {
	// The following (private!) function allows unchecked construction of a
	// ogg page from a u8 slice.
    fn from_u8_slice_unchecked(s: &[u8]) -> &OggPage {
        unsafe { mem::transmute(s) }
    }

	// The following (private!) function allows unchecked construction of a
	// mutable ogg page from a mutable u8 slice.
    fn from_u8_slice_unchecked_mut(s: &mut [u8]) -> &mut OggPage {
        unsafe { mem::transmute(s) }
    }

    fn as_u8_slice(&self) -> &[u8] {
        unsafe { mem::transmute(self) }
    }

    fn as_u8_slice_mut(&mut self) -> &mut [u8] {
        unsafe { mem::transmute(self) }
    }

    pub fn new(buf: &[u8]) -> Result<&OggPage, OggPageCheckError> {
        let (h_len, b_len) = try!(OggPage::measure(buf));
        let page_length = (h_len + b_len) as usize;
        Ok(OggPage::from_u8_slice_unchecked(&buf[0..page_length]))
    }

    pub fn new_mut(buf: &mut [u8]) -> Result<&mut OggPage, OggPageCheckError> {
        let (h_len, b_len) = try!(OggPage::measure(buf));
        let page_length = (h_len + b_len) as usize;
        Ok(OggPage::from_u8_slice_unchecked_mut(&mut buf[0..page_length]))
    }


    pub fn measure(buf: &[u8]) -> Result<(u64, u64), OggPageCheckError> {
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

		Ok((cursor.position(), body_len))
	}

	pub fn position(&mut self) -> u64 {
		let self_buf = self.as_u8_slice();
		let mut cur = Cursor::new(&self_buf[POSITION_OFFSET..POSITION_OFFSET+8]);
		cur.read_u64::<LittleEndian>().unwrap()
	}

	pub fn set_position(&mut self, granule: u64) {
		let mut tx = self.begin();
		tx.set_position(granule);
	}

	pub fn serial(&mut self) -> u32 {
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