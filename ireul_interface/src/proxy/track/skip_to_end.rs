use std::sync::mpsc::{TrySendError, RecvError};

use serde::{ser, de};
use ogg::{OggTrackBuf, OggTrack};
use super::super::{RequestType, Request, RpcError};

/// Skips to the end of the currently playing track
#[derive(Serialize, Deserialize)]
pub struct TrackSkipToEndRequest;

impl Request for TrackSkipToEndRequest {
    type Value = ();
    type Error = TrackSkipToEndError;

    fn req_type(&self) -> RequestType {
        RequestType::EnqueueTrack
    }
}

#[derive(Serialize, Deserialize)]
pub enum TrackSkipToEndError {
    // this should be moved, since everything will have it...
    RemoteSerdeError,
}
