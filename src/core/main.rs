extern crate ogg;

use std::sync::mpsc;
use std::net::TcpListener;
use std::collections::VecDeque;
use std::path::Path;
use std::io;

use ogg::OggPageBuf;

fn main() {
    // Handler::new("/tmp/ireul-core").unwrap().start().unwrap();
}

enum CoreNotify {}

struct OggTrack {
    pages: Vec<OggPageBuf>
}

struct IceCastConnector;

/// Connects to IceCast and holds references to streamable content.
struct OutputManager {
    connector: IceCastConnector,
    play_queue: VecDeque<OggTrack>,
    offline_track: OggTrack,
}

struct Core {
    control: TcpListener,
    output: OutputManager,
}

impl Core {
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Core> {
        unimplemented!();
    }

    pub fn register_client(&mut self, client: Client) {
        //
    }
}

enum Client {}

enum CoreProxyRequest {}

struct CoreProxy {
    sender: mpsc::SyncSender<CoreProxyRequest>
}

impl CoreProxy {
    pub fn add_track(&mut self, track: OggTrack) -> Result<(), AddTrackError> {
        unimplemented!();
        // self.sender.try_send(track))
    }
}

enum AddTrackError {
    SendError(mpsc::TrySendError<OggTrack>),
}