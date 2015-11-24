use std::sync::mpsc::{TrySendError, RecvError};

use serde::{ser, de};
use ogg::{OggTrackBuf, OggTrack};
use super::super::{RequestType, Request, RpcError};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FastForward {
    TrackBoundary = 0,
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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FastForwardRequest {
    pub kind: FastForward,
}

impl Request for FastForwardRequest {
    type Value = ();
    type Error = FastForwardError;

    fn req_type(&self) -> RequestType {
        RequestType::FastForward
    }
}

pub type FastForwardResult = Result<(), FastForwardError>;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FastForwardError;
//
// pub enum TrackSkipToEndError {
//     __Void,
// }
