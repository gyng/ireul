extern crate ogg;
extern crate ogg_clock;
extern crate rustc_serialize;

use std::sync::mpsc::{self, TrySendError, RecvError};
use std::net::TcpListener;
use std::collections::VecDeque;
use std::path::Path;
use std::io::{self, Read};
use std::fs::{self, File};
use std::net::{self, TcpStream};

use ogg::{OggPage, OggPageBuf, OggPageCheckError};
use ogg_clock::OggClock;

mod proxy;
mod oggutil;
mod icecastwriter;

use icecastwriter::{
    IceCastWriter,
    IceCastWriterOptions,
};
use oggutil::OggTrack;
use proxy::{
    CoreProxyRequest,
    CoreProxyResponse,
    CoreProxyCommand,
    EnqueueTrackError,
};

fn make_ogg_track(buffer: &[u8]) -> Result<OggTrack, OggPageCheckError> {
    let mut offset = 0;
    let mut pages = Vec::new();
    while offset < buffer.len() {
        let page: &OggPage = try!(OggPage::new(&buffer[offset..]));
        offset += page.as_u8_slice().len();
        pages.push(page.to_owned());
    }
    Ok(OggTrack {
        pages: pages,
    })
}

fn main() {
    let icecast_options = IceCastWriterOptions {
        endpoint: "lollipop.hiphop:8000",
        mount: "/ireul",
        user_pass: Some("source:3ay4fgdzkkcaokmo9e8k"),
        ..Default::default()
    };
    let mut connector = IceCastWriter::new(icecast_options).unwrap();

    let mut file = File::open("howbigisthis.ogg").unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    let offline_track = make_ogg_track(&buffer).unwrap();

    let output_manager = OutputManager {
        connector: connector,
        clock: OggClock::new(48000),
        playing_offline: false,
        buffer: VecDeque::new(),
        play_queue: VecDeque::new(),
        offline_track: offline_track,
    };

    let control = TcpListener::bind("0.0.0.0:3001").unwrap();
    let mut core = Core::new(control, output_manager).unwrap();
    loop {
        core.tick();
    }
    // Handler::new("/tmp/ireul-core").unwrap().start().unwrap();
}

enum CoreNotify {}

struct Core {
    control: TcpListener,
    output: OutputManager,
    proxy_tx: mpsc::SyncSender<CoreProxyRequest>,
    proxy_rx: mpsc::Receiver<CoreProxyRequest>,
}

impl Core {
    fn new(control: TcpListener, om: OutputManager) -> io::Result<Core> {
        let (tx, rx) = mpsc::sync_channel(5);
        Ok(Core {
            control: control,
            output: om,
            proxy_tx: tx,
            proxy_rx: rx,
        })
    }

    fn get_proxy(&mut self) -> CoreProxy {
        CoreProxy {
            sender: self.proxy_tx.clone(),
        }
    }

    fn enqueue_track(&mut self, track: OggTrack) {
        self.output.play_queue.push_back(track);
    }

    fn tick(&mut self) {
        loop {
            if let Ok(cmd) = self.proxy_rx.try_recv() {
                let CoreProxyRequest {
                    response_queue: response_queue,
                    command: command,
                } = cmd;
                match command {
                    CoreProxyCommand::EnqueueTrack(track) => {
                        self.enqueue_track(track);
                        response_queue.send(CoreProxyResponse::Unit).unwrap()
                    },
                    // _ => {
                    //     // respond with error
                    // }
                }
            } else {
                break;
            }
        }
        self.output.copy_page();
    }
}

/// Connects to IceCast and holds references to streamable content.
struct OutputManager {
    connector: IceCastWriter,
    clock: OggClock,
    playing_offline: bool,
    buffer: VecDeque<OggPageBuf>,
    play_queue: VecDeque<OggTrack>,
    offline_track: OggTrack,
}

impl OutputManager {
    fn fill_buffer(&mut self) {
        let track = match self.play_queue.pop_front() {
            Some(track) => track,
            None => self.offline_track.clone(),
        };
        self.buffer.extend(track.pages.into_iter());
    }

    fn get_next_page(&mut self) -> OggPageBuf {
        if self.buffer.is_empty() {
            self.fill_buffer();
        }
        self.buffer.pop_front().unwrap()
    }

    fn copy_page(&mut self) {
        let page = self.get_next_page();
        self.connector.send_ogg_page(&page).unwrap();
    }
}

// impl OutputManager {
//     pub fn new(backup_track: OggTrack) -> OutputManager {
//         OutputManager {
//             connector: IceCastConnector,
//             clock: OggClock::new(48000),
//             play_queue: VecDeque::new(),
//             offline_track: backup_track,
//         }
//     }
// }

struct CoreProxy {
    sender: mpsc::SyncSender<CoreProxyRequest>
}

impl CoreProxy {
    pub fn enqueue_track(&mut self, track: OggTrack) -> Result<(), EnqueueTrackError> {
        let (tx, rx) = mpsc::sync_channel(1);
        let req = CoreProxyRequest {
            response_queue: tx,
            command: CoreProxyCommand::EnqueueTrack(track),
        };
        match self.sender.try_send(req) {
            Ok(()) => (),
            Err(TrySendError::Full(req)) => {
                let track = match req.command {
                    CoreProxyCommand::EnqueueTrack(track) => track,
                    // _ => unreachable!(),
                };
                
                let full = TrySendError::Full(track);
                return Err(EnqueueTrackError::SendError(full));
            },
            Err(TrySendError::Disconnected(req)) => {
                let track = match req.command {
                    CoreProxyCommand::EnqueueTrack(track) => track,
                    // _ => unreachable!(),
                };

                let disconnected = TrySendError::Disconnected(track);
                return Err(EnqueueTrackError::SendError(disconnected));
            },
        };
        match rx.recv() {
            Ok(res) => Ok(()),
            Err(err) => Err(From::from(err))
        }
    }
}

enum AddTrackError {
    SendError(mpsc::TrySendError<OggTrack>),
}
