use std::sync::mpsc::{TrySendError, RecvError};
use oggutil::OggTrack;

pub enum EnqueueTrackError {
    SendError(TrySendError<OggTrack>),
    RecvError(RecvError)
}

impl From<TrySendError<OggTrack>> for EnqueueTrackError {
    #[inline]
    fn from(e: TrySendError<OggTrack>) -> Self {
        EnqueueTrackError::SendError(e)
    }
}

impl From<RecvError> for EnqueueTrackError {
    #[inline]
    fn from(e: RecvError) -> Self {
        EnqueueTrackError::RecvError(e)
    }
}