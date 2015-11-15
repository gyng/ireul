use std::sync::mpsc::{TrySendError, RecvError};

use serde::{ser, de};
use ogg::{OggTrackBuf, OggTrack};
use super::super::{RequestType, Request, RpcError};

pub struct EnqueueTrackRequest {
    pub track: OggTrackBuf,
}

impl ser::Serialize for EnqueueTrackRequest {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: ser::Serializer,
    {
        serializer.visit_bytes(self.track.as_u8_slice())
    }
}

impl de::Deserialize for EnqueueTrackRequest {
    fn deserialize<D>(deserializer: &mut D) -> Result<EnqueueTrackRequest, D::Error>
        where D: de::Deserializer,
    {
        deserializer.visit(EnqueueTrackRequestVisitor)
    }
}

struct EnqueueTrackRequestVisitor;

impl de::Visitor for EnqueueTrackRequestVisitor {
    type Value = EnqueueTrackRequest;

    fn visit_bytes<E>(&mut self, bytes: &[u8]) -> Result<EnqueueTrackRequest, E>
        where E: de::Error,
    {
        match OggTrack::new(bytes) {
            Ok(val) => Ok(EnqueueTrackRequest { track: val.to_owned() }),
            Err(err) => {
                let msg = format!("invalid ogg page: {:?}", err);
                Err(de::Error::syntax(&msg))
            }
        }
    }

    fn visit_byte_buf<E>(&mut self, bytes: Vec<u8>) -> Result<EnqueueTrackRequest, E>
        where E: de::Error,
    {
        match OggTrackBuf::new(bytes) {
            Ok(val) => Ok(EnqueueTrackRequest { track: val }),
            Err(err) => {
                let msg = format!("invalid ogg page: {:?}", err);
                Err(de::Error::syntax(&msg))
            }
        }
    }
}

impl Request for EnqueueTrackRequest {
    type Value = ();
    type Error = EnqueueTrackError;

    fn req_type(&self) -> RequestType {
        RequestType::EnqueueTrack
    }
}

#[derive(Serialize, Deserialize)]
pub enum EnqueueTrackError {
    InvalidTrack,

    BadSampleRate,
}
