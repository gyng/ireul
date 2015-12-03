use std::io;

use byteorder::{BigEndian, ReadBytesExt};

use ogg::{OggTrackBuf};

use super::super::{RequestType, Request};
use ::proto::{self, Deserialize, Serialize};


pub struct EnqueueTrackRequest {
    pub track: OggTrackBuf,
}

impl Deserialize for EnqueueTrackRequest {
    fn read(buf: &mut io::Cursor<Vec<u8>>) -> io::Result<Self> {
        let type_id = try!(buf.read_u16::<BigEndian>());
        if type_id != proto::TYPE_STRUCT {
            return Err(io::Error::new(io::ErrorKind::Other, "unexpected type"));
        }

        let field_count = try!(buf.read_u32::<BigEndian>());

        let mut track: Option<Vec<u8>> = None;
        for _ in 0..field_count {
            let field_name: String = try!(Deserialize::read(buf));
            match &field_name[..] {
                "track" => {
                    track = Some(try!(Deserialize::read(buf)));
                },
                _ => try!(proto::skip_entity(buf)),
            }
        }

        let track = match track {
            Some(track) => track,
            None => return Err(io::Error::new(io::ErrorKind::Other, "missing field: track")),
        };

        let track = try!(OggTrackBuf::new(track)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "invalid ogg")));

        Ok(EnqueueTrackRequest {
            track: track,
        })
    }
}


impl Request for EnqueueTrackRequest {
    type Value = ();
    type Error = EnqueueTrackError;

    fn req_type(&self) -> RequestType {
        RequestType::EnqueueTrack
    }
}

pub type EnqueueTrackResult = Result<u64, EnqueueTrackError>;

#[derive(Debug, Clone)]
pub enum EnqueueTrackError {
    InvalidTrack = 1,

    BadSampleRate = 2,

    Full = 3,
}

impl EnqueueTrackError {
    pub fn to_u32(&self) -> u32 {
        self.clone() as u32
    }

    pub fn from_u32(val: u32) -> Option<EnqueueTrackError> {
        match val {
            1 => Some(EnqueueTrackError::InvalidTrack),
            2 => Some(EnqueueTrackError::BadSampleRate),
            3 => Some(EnqueueTrackError::Full),
            _ => None
        }
    }
}

impl Serialize for EnqueueTrackError {
    fn write(&self, buf: &mut io::Cursor<Vec<u8>>) -> io::Result<()> {
        try!(Serialize::write(&self.to_u32(), buf));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use super::{
        EnqueueTrackResult,
        EnqueueTrackError,
    };
    use ::proto::Serialize;

    fn serialize<T: Serialize>(item: &T) -> Vec<u8> {
        let mut buffer = io::Cursor::new(Vec::new());
        Serialize::write(item, &mut buffer).unwrap();
        buffer.into_inner()
    }

    #[test]
    fn test_serialize() {
        let ok_val: EnqueueTrackResult = Ok(0xD825959D752F9A3E);
        assert_eq!(&serialize(&ok_val)[..], &[
            // Result::Ok type
            0x00, 0x85,
            // u64 type
            0x00, 0x83,
            // value of 0xD825959D752F9A3E_u64
            0xd8, 0x25, 0x95, 0x9d, 0x75, 0x2f, 0x9a, 0x3e
        ]);

        let err_inv_track: EnqueueTrackResult = Err(EnqueueTrackError::InvalidTrack);
        assert_eq!(&serialize(&err_inv_track)[..], &[
            // Result::Err type
            0x00, 0x86,
            // u32 type
            0x00, 0x82,
            // value of 1_u32
            0x00, 0x00, 0x00, 0x01,
        ]);

        let err_bad_samp: EnqueueTrackResult = Err(EnqueueTrackError::BadSampleRate);
        assert_eq!(&serialize(&err_bad_samp)[..], &[
            // Result::Err type
            0x00, 0x86,
            // u32 type
            0x00, 0x82,
            // value of 2_u32
            0x00, 0x00, 0x00, 0x02,
        ]);

        let err_full: EnqueueTrackResult = Err(EnqueueTrackError::Full);
        assert_eq!(&serialize(&err_full)[..], &[
            // Result::Err type
            0x00, 0x86,
            // u32 type
            0x00, 0x82,
            // value of 3_u32
            0x00, 0x00, 0x00, 0x03,
        ]);
    }

}
