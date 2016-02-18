extern crate byteorder;

use std::mem;
use std::ops;
use std::str;
use std::convert;
use std::borrow::{Borrow, BorrowMut, ToOwned};
use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};

use ::reader;
use ::reader::Reader;
use ::slice::Slice;
use {OggPage};

#[derive(Debug)]
pub enum VorbisPacketCheckError {
    BadCapture,
    Invalid(&'static str),
    BadIdentificationHeader,
    BadIdentificationHeaderLength,
}

impl convert::From<str::Utf8Error> for VorbisPacketCheckError {
    fn from(_e: str::Utf8Error) -> VorbisPacketCheckError {
        VorbisPacketCheckError::Invalid("invalid utf8 in comment header")
    }
}

impl convert::From<reader::Error> for VorbisPacketCheckError {
    fn from(e: reader::Error) -> VorbisPacketCheckError {
        match e {
            reader::Error::Truncated => {
                VorbisPacketCheckError::Invalid("truncated comment header")
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum VorbisPacketType {
    Audio,
    IdentificationHeader,
    CommentHeader,
    SetupHeader,
}

impl VorbisPacketType {
    pub fn from_u8(n: u8) -> Option<VorbisPacketType> {
        match n {
            0 => Some(VorbisPacketType::Audio),
            1 => Some(VorbisPacketType::IdentificationHeader),
            3 => Some(VorbisPacketType::CommentHeader),
            5 => Some(VorbisPacketType::SetupHeader),
            _ => None
        }
    }
}

pub struct VorbisPacketBuf {
    inner: Vec<u8>,
}

pub struct VorbisPacket {
    inner: Slice,
}

impl VorbisPacketBuf {
    pub fn new(buf: Vec<u8>) -> Result<VorbisPacketBuf, VorbisPacketCheckError> {
        try!(VorbisPacket::check(&buf[..]));
        Ok(VorbisPacketBuf { inner: buf })
    }

    pub fn build_comment_packet(comments: &Comments) -> VorbisPacketBuf {
        let mut buf = Vec::new();
        buf.extend(b"\x03vorbis");
        write_comment_vendor(&mut buf, &comments.vendor);

        let mut comments_length = [0; 4];

        LittleEndian::write_u32(
            &mut comments_length[..],
            comments.comments.len() as u32);

        buf.extend(&comments_length);

        for &(ref key, ref val) in comments.comments.iter() {
            write_comment_tag(&mut buf, key, val);
        }

        buf.extend(b"\x01");

        VorbisPacketBuf { inner: buf }
    }
}

fn write_comment_tag(buf: &mut Vec<u8>, key: &str, val: &str) {
    let length = 1 + key.as_bytes().len() + val.as_bytes().len();
    let mut length_bytes = [0; 4];
    LittleEndian::write_u32(&mut length_bytes[..], length as u32);
    buf.extend(&length_bytes[..]);
    buf.extend(key.as_bytes());
    buf.extend(b"=");
    buf.extend(val.as_bytes());
}

fn write_comment_vendor(buf: &mut Vec<u8>, val: &str) {
    let mut length_bytes = [0; 4];
    LittleEndian::write_u32(&mut length_bytes[..], val.as_bytes().len() as u32);
    buf.extend(&length_bytes[..]);
    buf.extend(val.as_bytes());
}

impl ops::Deref for VorbisPacketBuf {
    type Target = VorbisPacket;

    fn deref<'a>(&'a self) -> &'a VorbisPacket {
        VorbisPacket::from_u8_slice_unchecked(&self.inner)
    }
}

impl AsRef<VorbisPacket> for VorbisPacketBuf {
    fn as_ref(&self) -> &VorbisPacket {
        VorbisPacket::from_u8_slice_unchecked(&self.inner)
    }
}

impl Borrow<VorbisPacket> for VorbisPacketBuf {
    fn borrow(&self) -> &VorbisPacket {
        VorbisPacket::from_u8_slice_unchecked(&self.inner)
    }
}

impl BorrowMut<VorbisPacket> for VorbisPacketBuf {
    fn borrow_mut(&mut self) -> &mut VorbisPacket {
        VorbisPacket::from_u8_slice_unchecked_mut(&mut self.inner)
    }
}

impl ToOwned for VorbisPacket {
    type Owned = VorbisPacketBuf;

    fn to_owned(&self) -> VorbisPacketBuf {
        VorbisPacketBuf { inner: self.inner.to_owned() }
    }
}

impl VorbisPacket {
    pub fn new(buf: &[u8]) -> Result<&VorbisPacket, VorbisPacketCheckError> {
        try!(VorbisPacket::check(buf));
        Ok(VorbisPacket::from_u8_slice_unchecked(buf))
    }

    pub fn new_mut(buf: &mut [u8]) -> Result<&mut VorbisPacket, VorbisPacketCheckError> {
        try!(VorbisPacket::check(buf));
        Ok(VorbisPacket::from_u8_slice_unchecked_mut(buf))
    }

    // The following (private!) function allows unchecked construction of a
    // ogg page from a u8 slice.
    fn from_u8_slice_unchecked(s: &[u8]) -> &VorbisPacket {
        unsafe { mem::transmute(s) }
    }

    // The following (private!) function allows unchecked construction of a
    // mutable ogg page from a mutable u8 slice.
    fn from_u8_slice_unchecked_mut(s: &mut [u8]) -> &mut VorbisPacket {
        unsafe { mem::transmute(s) }
    }

    pub fn as_u8_slice(&self) -> &[u8] {
        unsafe { mem::transmute(self) }
    }

    pub fn find_identification<'a, I>(iter: I) -> Result<&'a VorbisPacket, ()>
        where I: Iterator<Item=&'a OggPage>
    {
        for page in iter {
            for packet in page.raw_packets() {
                if let Ok(vpkt) = VorbisPacket::new(packet) {
                    if vpkt.identification_header().is_some() {
                        return Ok(vpkt);
                    }
                }
            }
        }
        Err(())
    }

    pub fn find_comments<'a, I>(iter: I) -> Result<&'a VorbisPacket, ()>
        where I: Iterator<Item=&'a OggPage>
    {
        for page in iter {
            for packet in page.raw_packets() {
                if let Ok(vpkt) = VorbisPacket::new(packet) {
                    if vpkt.comments().is_some() {
                        return Ok(vpkt);
                    }
                }
           }
        }
        Err(())
    }

    pub fn check(buf: &[u8]) -> Result<(), VorbisPacketCheckError> {
        if buf.len() < 1 {
            return Err(VorbisPacketCheckError::BadCapture)
        }
        match VorbisPacketType::from_u8(buf[0]) {
            None => {
                return Err(VorbisPacketCheckError::BadCapture);
            },
            Some(VorbisPacketType::IdentificationHeader) => {
                if buf.len() < 8 || &buf[1..7] != b"vorbis" {
                    return Err(VorbisPacketCheckError::BadCapture)
                }
                try!(VorbisPacket::parse_identification_header(buf));
            },
            Some(VorbisPacketType::CommentHeader) => {
                if buf.len() < 8 || &buf[1..7] != b"vorbis" {
                    return Err(VorbisPacketCheckError::BadCapture)
                }
                try!(VorbisPacket::parse_comment_header(buf));
            },
            _ => ()
        }

        Ok(())
    }

    pub fn identification_header(&self) -> Option<IdentificationHeader> {
        let buf = self.as_u8_slice();

        // We know the header is well-formed, so it must have a valid VorbisPacketType
        match VorbisPacketType::from_u8(buf[0]).unwrap() {
            VorbisPacketType::IdentificationHeader => {
                let id_header = VorbisPacket::parse_identification_header(buf)
                    .expect("identification header parse error: this shouldn't happen");
                Some(id_header)
            },
            _ => None
        }
    }

    pub fn comments(&self) -> Option<Comments> {
        let buf = self.as_u8_slice();

        // We know the header is well-formed, so it must have a valid VorbisPacketType
        match VorbisPacketType::from_u8(buf[0]).unwrap() {
            VorbisPacketType::CommentHeader => {
                let id_header = VorbisPacket::parse_comment_header(buf)
                    .expect("identification header parse error: this shouldn't happen");
                Some(id_header)
            },
            _ => None
        }
    }

    fn parse_identification_header(buf: &[u8]) -> Result<IdentificationHeader, VorbisPacketCheckError> {
        // Must only be called on IdentificationHeader packets.
        assert_eq!(VorbisPacketType::from_u8(buf[0]).unwrap(),
            VorbisPacketType::IdentificationHeader);

        if buf.len() < 30 {
            return Err(VorbisPacketCheckError::BadIdentificationHeaderLength)
        }

        let vorbis_version = LittleEndian::read_u32(&buf[7 .. 11]);
        let audio_channels = buf[11];
        let audio_sample_rate = LittleEndian::read_u32(&buf[12 .. 16]);

        if audio_channels <= 0 || audio_sample_rate <= 0 {
            // vorbis_version should = 0 to meet Vorbis I specification but it's not checked here
            return Err(VorbisPacketCheckError::BadIdentificationHeader)
        }

        let bitrate_maximum = LittleEndian::read_u32(&buf[16 .. 20]);
        let bitrate_nominal = LittleEndian::read_u32(&buf[20 .. 24]);
        let bitrate_minimum = LittleEndian::read_u32(&buf[24 .. 28]);

        let blocksize_byte = buf[28];
        let blocksize_0 = blocksize_byte & 0b00001111;
        let blocksize_1 = blocksize_byte >> 4;

        if blocksize_0 > blocksize_1 || buf[29] & 1 != 1 {
            // If blocksize 0 > blocksize 1 the file is undecodable
            // If framing flag is missing, the file is undecodable
            return Err(VorbisPacketCheckError::BadIdentificationHeader)
        }

        // It appears framing_flag takes up a byte by itself so buffer is useless
        // let buffer = framing_byte & 0b01111111;

        Ok(IdentificationHeader {
            vorbis_version: vorbis_version,
            audio_channels: audio_channels,
            audio_sample_rate: audio_sample_rate,
            bitrate_maximum: bitrate_maximum,
            bitrate_nominal: bitrate_nominal,
            bitrate_minimum: bitrate_minimum,
            blocksize_0: blocksize_0,
            blocksize_1: blocksize_1,
        })
    }

    fn parse_comment_header(buf: &[u8]) -> Result<Comments, VorbisPacketCheckError> {
        let mut reader = Reader::<LittleEndian>::new(buf);
        assert_eq!(reader.read_buffer(7).ok().unwrap(), b"\x03vorbis");

        let vendor_len = try!(reader.read_u32());
        let vendor_buf = try!(reader.read_buffer(vendor_len as usize));
        let vendor = try!(str::from_utf8(vendor_buf)).to_string();

        let comment_count = try!(reader.read_u32());
        let mut comments = Vec::new();

        for _ in 0..comment_count {
            let comment_len = try!(reader.read_u32());
            let comment_buf = try!(reader.read_buffer(comment_len as usize));
            let comment_str = try!(str::from_utf8(comment_buf));
            let (key, val) = try!(split_comment(comment_str));
            comments.push((key.to_string(), val.to_string()));
        }

        if (try!(reader.read_u8()) & 1) != 1 {
            return Err(VorbisPacketCheckError::Invalid("framing bit unset"))
        }

        Ok(Comments {
            vendor: vendor,
            comments: comments,
        })
    }
}

#[derive(Debug)]
pub struct IdentificationHeader {
    pub vorbis_version: u32,
    pub audio_channels: u8,
    pub audio_sample_rate: u32,
    pub bitrate_maximum: u32,
    pub bitrate_nominal: u32,
    pub bitrate_minimum: u32,
    pub blocksize_0: u8,
    pub blocksize_1: u8,
}

#[derive(Debug, Clone)]
pub struct Comments {
    pub vendor: String,
    pub comments: Vec<(String, String)>
}

fn split_comment(buffer: &str) -> Result<(&str, &str), VorbisPacketCheckError>{
    match buffer.find("=") {
        Some(idx) => {
            // TODO: validate key: 0x20 through 0x7D excluding 0x3D
            Ok((&buffer[..idx], &buffer[idx+1..]))
        }
        None => Err(VorbisPacketCheckError::Invalid("Invalid comment")),
    }
}


#[cfg(test)]
mod test {
    use {OggTrack};
    use super::{VorbisPacketBuf, VorbisPacket, Comments};

    #[test]
    fn test_parse_identification_header() {
        let header_buf = [
            0x01,                               // 0     packet type, 1 = id header
            0x76, 0x6f, 0x72, 0x62, 0x69, 0x73, // 1-6   vorbis
            0x00, 0x00, 0x00, 0x00,             // 7-10  version
            0x02,                               // 11    channels
            0x80, 0xbb, 0x00, 0x00,             // 12-15 sample_rate (48000)
            0x00, 0x00, 0x00, 0x00,             // 16-19 bitrate_minimum
            0x80, 0xb5, 0x01, 0x00,             // 20-23 bitrate_nominal
            0x00, 0x00, 0x00, 0x00,             // 24-27 bitrate_maximum
            0xb8,                               // 28    [blocksize_0][blocksize_1]
            0x01                                // 29    framing_flag
        ];

        let test_header = VorbisPacket::new(&header_buf).unwrap();
        let id_header = test_header.identification_header().unwrap();

        assert_eq!(id_header.vorbis_version, 0);
        assert_eq!(id_header.audio_channels, 2);
        assert_eq!(id_header.audio_sample_rate, 48000);
        assert_eq!(id_header.bitrate_maximum, 0);
        assert_eq!(id_header.bitrate_nominal, 112000);
        assert_eq!(id_header.bitrate_minimum, 0);
        assert_eq!(id_header.blocksize_0, 0b1000);
        assert_eq!(id_header.blocksize_1, 0b1011);
    }

    #[test]
    fn test_parse_non_identification_headers() {
        let not_an_id_header_buf = [
            0x00,
            0x76, 0x6f, 0x72, 0x62, 0x69, 0x73,
            0x00, 0x00, 0x00, 0x00,
        ];

        let negative_test_header = VorbisPacket::new(&not_an_id_header_buf).unwrap();
        let negative_id_header = negative_test_header.identification_header();
        assert!(negative_id_header.is_none());
    }

    #[test]
    fn test_parse_malformed_identification_header() {
        let malformed_header_buf = [0x01, 0x76, 0x6f, 0x72, 0x62, 0x69, 0x73];
        let malformed_test_header = VorbisPacket::new(&malformed_header_buf);
        assert!(malformed_test_header.is_err());
    }

    static SAMPLE_OGG: &'static [u8] = include_bytes!("../testdata/Hydrate-Kenny_Beltrey.ogg");

    static COMMENT_HEADER_VALID: &'static [u8] = &[
        0x03, b'v', b'o', b'r', b'b', b'i', b's',

        0x04, 0x00, 0x00, 0x00,  // vendor length = 4
        b't', b'e', b's', b't',

        0x02, 0x00, 0x00, 0x00,  // comment count
        0x04, 0x00, 0x00, 0x00,  // comment length = 4
        b'A', b'=', b'a', b'a',
        0x04, 0x00, 0x00, 0x00,  // comment length = 4
        b'B', b'=', b'b', b'b',

        0x01 // unset framing bit
    ];

    static COMMENT_HEADER_UNSET_FRAMING_BIT: &'static [u8] = &[
        0x03, b'v', b'o', b'r', b'b', b'i', b's',

        0x04, 0x00, 0x00, 0x00,  // vendor length = 4
        b't', b'e', b's', b't',

        0x02, 0x00, 0x00, 0x00,  // comment count
        0x04, 0x00, 0x00, 0x00,  // comment length = 4
        b'A', b'=', b'a', b'a',
        0x04, 0x00, 0x00, 0x00,  // comment length = 4
        b'B', b'=', b'b', b'b',

        0x00 // unset framing bit
    ];

    static COMMENT_HEADER_FRAMING_BIT_TRUNCATED: &'static [u8] = &[
        0x03, b'v', b'o', b'r', b'b', b'i', b's',

        0x04, 0x00, 0x00, 0x00,  // vendor length = 4
        b't', b'e', b's', b't',

        0x02, 0x00, 0x00, 0x00,  // comment count
        0x04, 0x00, 0x00, 0x00,  // comment length = 4
        b'A', b'=', b'a', b'a',
        0x04, 0x00, 0x00, 0x00,  // comment length = 4
        b'B', b'=', b'b', b'b',

        // truncated: missing framing bit
    ];

    static COMMENT_HEADER_TRUNCATED_MID_COMMENT: &'static [u8] = &[
        0x03, b'v', b'o', b'r', b'b', b'i', b's',

        0x04, 0x00, 0x00, 0x00,  // vendor length = 4
        b't', b'e', b's', b't',

        0x02, 0x00, 0x00, 0x00,  // comment count
        0x04, 0x00, 0x00, 0x00,  // comment length = 4
        b'A', b'=', b'a', b'a',
        0x04, 0x00, 0x00, 0x00,  // comment length = 4
        b'B', b'=',

        // truncated: the second comment should have continued, but didn't.
    ];

    static COMMENT_HEADER_TRUNCATED_COMMENTS: &'static [u8] = &[
        0x03, b'v', b'o', b'r', b'b', b'i', b's',

        0x04, 0x00, 0x00, 0x00,  // vendor length = 4
        b't', b'e', b's', b't',

        0x02, 0x00, 0x00, 0x00,  // comment count
        0x04, 0x00, 0x00, 0x00,  // comment length = 4
        b'A', b'=', b'a', b'a',

        // truncated: we should have a comment here, but we don't.
    ];

    fn comments_helper(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items
            .into_iter()
            .map(|&(k,v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn test_comment_from_ogg() {
        let track = OggTrack::new(SAMPLE_OGG).unwrap();
        let comm_packet = VorbisPacket::find_comments(track.pages()).unwrap();

        let comments = comm_packet.comments().unwrap();
        assert_eq!(comments.vendor, "Xiph.Org libVorbis I 20020713");
        assert_eq!(comments.comments, comments_helper(&[
            ("TITLE", "Hydrate - Kenny Beltrey"),
            ("ARTIST", "Kenny Beltrey"),
            ("ALBUM", "Favorite Things"),
            ("DATE", "2002"),
            ("COMMENT", "http://www.kahvi.org"),
            ("TRACKNUMBER", "2")
        ]));
    }

    #[test]
    fn test_rebuild_comment() {
        let track = OggTrack::new(SAMPLE_OGG).unwrap();
        let expected_packet = VorbisPacket::find_comments(track.pages()).unwrap();

        let comments = Comments {
            vendor: "Xiph.Org libVorbis I 20020713".to_string(),
            comments: comments_helper(&[
                ("TITLE", "Hydrate - Kenny Beltrey"),
                ("ARTIST", "Kenny Beltrey"),
                ("ALBUM", "Favorite Things"),
                ("DATE", "2002"),
                ("COMMENT", "http://www.kahvi.org"),
                ("TRACKNUMBER", "2")
            ]),
        };

        let pkt = VorbisPacketBuf::build_comment_packet(&comments);
        assert_eq!(
            expected_packet.as_u8_slice(),
            pkt.as_u8_slice());
    }


    #[test]
    fn test_parse_comment_header_valid() {
        let test_header = VorbisPacket::new(COMMENT_HEADER_VALID).unwrap();
        let comments = test_header.comments().unwrap();
        assert_eq!(comments.vendor, "test");
        assert_eq!(comments.comments.len(), 2);
    }

    #[test]
    fn test_parse_malformed_comment_header_unset_framing_bit() {
        VorbisPacket::new(COMMENT_HEADER_UNSET_FRAMING_BIT).err().unwrap();
    }

    #[test]
    fn test_parse_malformed_comment_header_framing_bit_truncated() {
        VorbisPacket::new(COMMENT_HEADER_FRAMING_BIT_TRUNCATED).err().unwrap();
    }

    #[test]
    fn test_parse_malformed_comment_header_truncated_mid_comment() {
        VorbisPacket::new(COMMENT_HEADER_TRUNCATED_MID_COMMENT).err().unwrap();
    }

    #[test]
    fn test_parse_malformed_comment_header_truncated_comments() {
        VorbisPacket::new(COMMENT_HEADER_TRUNCATED_COMMENTS).err().unwrap();
    }
}
