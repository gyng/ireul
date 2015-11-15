extern crate bincode;
extern crate ogg;
extern crate ogg_clock;
extern crate rustc_serialize;
extern crate serde;
extern crate ireul_interface;

use std::sync::mpsc::{self};
use std::net::TcpListener;
use std::collections::VecDeque;
use std::io::{self, Read};
use std::fs::File;

use ogg::{OggTrackBuf, OggPageBuf};
use ogg_clock::OggClock;
use ireul_interface::proxy::{
    SIZE_LIMIT,
    RequestWrapper,
    RequestType,
    EnqueueTrackRequest,
    EnqueueTrackError,
};

mod icecastwriter;

use icecastwriter::{
    IceCastWriter,
    IceCastWriterOptions,
};

fn main() {
    let icecast_options = IceCastWriterOptions {
        endpoint: "lollipop.hiphop:8000",
        mount: "/ireul",
        user_pass: Some("source:x"),
        ..Default::default()
    };
    let connector = IceCastWriter::new(icecast_options).unwrap();

    let mut file = File::open("howbigisthis.ogg").unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    let offline_track = OggTrackBuf::new(buffer).unwrap();

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
        println!("copied page");
    }
}

struct Core {
    control: TcpListener,
    output: OutputManager,
    proxy_tx: mpsc::SyncSender<RequestWrapper>,
    proxy_rx: mpsc::Receiver<RequestWrapper>,
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

    fn enqueue_track(&mut self, req: EnqueueTrackRequest) -> Result<(), EnqueueTrackError> {
        // TODO: validate position granule is strictly increasing
        //       and begins with zero.
        //
        // TODO: ensure sample rate is equal to our ogg-clock's sample rate,
        //       using the vorbis identification header.
        //
        self.output.play_queue.push_back(req.track);

        Ok(())
    }

    fn enqueue_track_helper(&mut self, req: &[u8]) -> Vec<u8> {
        let res = bincode::serde::deserialize(req)
            .map_err(|_| EnqueueTrackError::RemoteSerdeError)
            .and_then(|req| self.enqueue_track(req));

        bincode::serde::serialize(&res, SIZE_LIMIT).unwrap()
    }

    fn handle_command(&mut self, req_wr: RequestWrapper) {
        let RequestWrapper {
            response_queue: response_queue,
            req_type: req_type,
            req_buf: req_buf,
        } = req_wr;

        let response = match req_type {
            RequestType::EnqueueTrack => {
                self.enqueue_track_helper(&req_buf)
            },
        };
        response_queue.send(response).unwrap();
    }

    fn tick(&mut self) {
        loop {
            match self.proxy_rx.try_recv() {
                Ok(cmd) => self.handle_command(cmd),
                Err(err) => break,
            }
        }
        self.output.copy_page();
    }
}

/// Connects to IceCast and holds references to streamable content.
struct OutputManager {
    // TODO: this needs a helper to fix OggPage positions, so that the stream
    //       starts with position of zero and strictly increases with time,
    //       by the number of samples played.
    //
    // TODO: this needs a helper to fix OggPage sequences, so that the stream
    //       starts with a sequence of zero and increases by one each time a
    //       page is emitted.
    //
    connector: IceCastWriter,
    clock: OggClock,
    playing_offline: bool,
    buffer: VecDeque<OggPageBuf>,
    play_queue: VecDeque<OggTrackBuf>,
    offline_track: OggTrackBuf,
}

impl OutputManager {
    fn fill_buffer(&mut self) {
        let track = match self.play_queue.pop_front() {
            Some(track) => track,
            None => self.offline_track.clone(),
        };
        self.buffer.extend(track.pages().map(|x| x.to_owned()));
    }

    fn get_next_page(&mut self) -> OggPageBuf {
        if self.buffer.is_empty() {
            self.fill_buffer();
        }
        self.buffer.pop_front().unwrap()
    }

    fn copy_page(&mut self) {
        let page = self.get_next_page();
        self.clock.wait(&page).unwrap();
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

