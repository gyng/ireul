use std::io;
use byteorder::{BigEndian, ReadBytesExt};

use super::super::{RequestType, Request};
use super::model::{};

use ::proto::{self, Deserialize, Serialize};


/// Skips to the end of the currently playing track
#[derive(Debug, Clone)]
pub struct StatusRequest;

impl Deserialize for StatusRequest {
    fn read(buf: &mut io::Cursor<Vec<u8>>) -> io::Result<Self> {
        let type_id = try!(buf.read_u16::<BigEndian>());
        if type_id != proto::TYPE_STRUCT {
            return Err(io::Error::new(io::ErrorKind::Other, "unexpected type"));
        }
        let field_count = try!(buf.read_u32::<BigEndian>());
        for _ in 0..field_count {
            let _: String = try!(Deserialize::read(buf));
            try!(proto::skip_entity(buf));
        }

        Ok(StatusRequest)
    }
}

impl Request for StatusRequest {
    type Value = ();
    type Error = StatusError;

    fn req_type(&self) -> RequestType {
        RequestType::FastForward
    }
}

pub type StatusResult = Result<(), StatusError>;

#[derive(Debug, Clone)]
pub struct StatusError;

impl Serialize for StatusError {
    fn write(&self, buf: &mut io::Cursor<Vec<u8>>) -> io::Result<()> {
        try!(Serialize::write(&(), buf));
        Ok(())
    }
}
