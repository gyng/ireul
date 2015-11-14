use std::mem;
use std::ops;
use std::borrow::{Borrow, BorrowMut, ToOwned};
use std::io::{Cursor, BufRead};
use std::marker::PhantomData;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use byteorder::Error as ByteOrderError;

use ::slice::Slice;

#[derive(Debug)]
pub enum VorbisHeaderCheckError {
    BadCapture,
    BadVersion,
}

pub struct VorbisHeaderBuf {
    inner: Vec<u8>,
}

pub struct VorbisHeader {
    inner: Slice,
}

impl ops::Deref for VorbisHeaderBuf {
    type Target = VorbisHeader;

    fn deref<'a>(&'a self) -> &'a VorbisHeader {
        VorbisHeader::from_u8_slice_unchecked(&self.inner)
    }
}

impl Borrow<VorbisHeader> for VorbisHeaderBuf {
    fn borrow(&self) -> &VorbisHeader {
        VorbisHeader::from_u8_slice_unchecked(&self.inner)
    }
}

impl BorrowMut<VorbisHeader> for VorbisHeaderBuf {
    fn borrow_mut(&mut self) -> &mut VorbisHeader {
        VorbisHeader::from_u8_slice_unchecked_mut(&mut self.inner)
    }
}

impl ToOwned for VorbisHeader {
    type Owned = VorbisHeaderBuf;
    
    fn to_owned(&self) -> VorbisHeaderBuf {
        VorbisHeaderBuf { inner: self.inner.to_owned() }
    }
}

impl VorbisHeader {
    // The following (private!) function allows unchecked construction of a
    // ogg page from a u8 slice.
    fn from_u8_slice_unchecked(s: &[u8]) -> &VorbisHeader {
        unsafe { mem::transmute(s) }
    }

    // The following (private!) function allows unchecked construction of a
    // mutable ogg page from a mutable u8 slice.
    fn from_u8_slice_unchecked_mut(s: &mut [u8]) -> &mut VorbisHeader {
        unsafe { mem::transmute(s) }
    }

    pub fn as_u8_slice(&self) -> &[u8] {
        unsafe { mem::transmute(self) }
    }

    fn as_u8_slice_mut(&mut self) -> &mut [u8] {
        unsafe { mem::transmute(self) }
    }

    pub fn check(buf: &[u8]) -> Result<(), VorbisHeaderCheckError> {
        unimplemented!();
    }

    pub fn from_page(page: &OggPage) -> Result<VorbisHeader, VorbisHeaderCheckError> {
        page.packets(packets)
    }

    pub fn new(buf: &[u8]) -> Result<&VorbisHeader, VorbisHeaderCheckError> {
        try!(VorbisHeader::check(buf));
        Ok(VorbisHeader::from_u8_slice_unchecked(buf))
    }

    pub fn new_mut(buf: &mut [u8]) -> Result<&mut VorbisHeader, VorbisHeaderCheckError> {
        try!(VorbisHeader::check(buf));
        Ok(VorbisHeader::from_u8_slice_unchecked_mut(buf))
    }

    pub fn identification_header(&self) -> Option<IdentificationHeader> {
        None
    }
}

struct IdentificationHeader {
    vorbis_version: u32,
    audio_channels: u8,
    audio_sample_rate: u32,
    bitrate_maximum: u32,
    bitrate_nominal: u32,
    bitrate_minimum: u32,
    // ....
}
