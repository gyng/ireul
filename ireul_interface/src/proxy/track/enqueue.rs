use ogg::{OggTrackBuf};
use super::super::{RequestType, Request};

pub struct EnqueueTrackRequest {
    pub track: OggTrackBuf,
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
    pub fn from_u32(val: u32) -> Option<EnqueueTrackError> {
        match val {
            1 => Some(EnqueueTrackError::InvalidTrack),
            2 => Some(EnqueueTrackError::BadSampleRate),
            3 => Some(EnqueueTrackError::Full),
            _ => None
        }
    }
}
