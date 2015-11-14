use std::sync::mpsc::{
    self,
    SyncSender,
    TrySendError,
    RecvError
};

mod enqueue_track;

use oggutil::OggTrack;
pub use self::enqueue_track::EnqueueTrackError;

pub enum CoreProxyCommand {
    EnqueueTrack(OggTrack),
}

pub enum CoreProxyResponse {
    Unit,
}

pub struct CoreProxyRequest {
    pub response_queue: mpsc::SyncSender<CoreProxyResponse>,
    pub command: CoreProxyCommand,
}

// 

// pub trait ProxyCommand<Arg>: Serialize + Deserialize {
//
//     type Value: Serialize + Deserialize;
//     type Error: Serialize + Deserialize + 
// }

// pub struct CoreProxyRequest<C> {
//     pub response_queue: mpsc::SyncSender<CoreProxyResponse>,
//     pub command: C,
// }


