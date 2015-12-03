use std::io;

use byteorder::{BigEndian, ReadBytesExt};

use super::super::{RequestType, Request};
use ::proto::{self, Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum FastForward {
    TrackBoundary = 0,
}

impl Deserialize for FastForward {
    fn read(buf: &mut io::Cursor<Vec<u8>>) -> io::Result<Self> {
        let num: u32 = try!(Deserialize::read(buf));
        FastForward::from_u32(num)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::Other, "unexpected FastForward value")
            })
    }
}

impl FastForward {
    pub fn from_u32(n: u32) -> Option<FastForward> {
        match n {
            0 => Some(FastForward::TrackBoundary),
            _ => None,
        }
    }
}

/// Skips to the end of the currently playing track
#[derive(Debug, Clone)]
pub struct FastForwardRequest {
    pub kind: FastForward,
}

impl Deserialize for FastForwardRequest {
    fn read(buf: &mut io::Cursor<Vec<u8>>) -> io::Result<Self> {
        let type_id = try!(buf.read_u16::<BigEndian>());
        if type_id != proto::TYPE_STRUCT {
            return Err(io::Error::new(io::ErrorKind::Other, "unexpected type"));
        }

        let field_count = try!(buf.read_u32::<BigEndian>());

        let mut kind: Option<FastForward> = None;
        for _ in 0..field_count {
            let field_name: String = try!(Deserialize::read(buf));
            match &field_name[..] {
                "kind" => {
                    kind = Some(try!(Deserialize::read(buf)));
                },
                _ => try!(proto::skip_entity(buf)),
            }
        }

        let kind = match kind {
            Some(kind) => kind,
            None => return Err(io::Error::new(io::ErrorKind::Other, "missing field: kind")),
        };

        Ok(FastForwardRequest {
            kind: kind,
        })
    }
}

impl Request for FastForwardRequest {
    type Value = ();
    type Error = FastForwardError;

    fn req_type(&self) -> RequestType {
        RequestType::FastForward
    }
}

pub type FastForwardResult = Result<(), FastForwardError>;

#[derive(Debug, Clone)]
pub struct FastForwardError;

impl Serialize for FastForwardError {
    fn write(&self, buf: &mut io::Cursor<Vec<u8>>) -> io::Result<()> {
        try!(Serialize::write(&(), buf));
        Ok(())
    }
}
