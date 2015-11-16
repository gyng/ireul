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

use ogg::{OggTrack, OggTrackBuf, OggPageBuf};
use ogg_clock::OggClock;
use ireul_interface::proxy::{
    SIZE_LIMIT,
    RequestWrapper,
    RequestType,
    BinderError,
    EnqueueTrackRequest,
    EnqueueTrackError,
    TrackSkipToEndRequest,
    TrackSkipToEndError,
};

mod icecastwriter;

use icecastwriter::{
    IceCastWriter,
    IceCastWriterOptions,
};

fn main() {
    let mut icecast_options = IceCastWriterOptions::default();
    icecast_options
        .set_endpoint("lollipop.hiphop:8000")
        .set_mount("/ireul")
        .set_user_pass("source:x");

    let connector = IceCastWriter::new(icecast_options).unwrap();

    let mut file = File::open("howbigisthis.ogg").unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    let offline_track = OggTrackBuf::new(buffer).unwrap();

    let output_manager = OutputManager {
        connector: connector,
        cur_serial: 0,
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

fn validate_positions(track: &OggTrack) -> Result<(), ()> {
    let mut current = 0;
    let mut is_first = true;

    for page in track.pages() {
        let position = page.position();

        if is_first {
            is_first = false;

            if position != 0 {
                return Err(());
            }
        }

        if position < current {
            return Err(());
        }
        current = position;
    }

    Ok(())
}

fn check_sample_rate(req: u32, track: &OggTrack) -> Result<(), ()> {
    Err(())
}

fn update_serial(serial: u32, track: &mut OggTrack) {
    for page in track.pages_mut() {
        page.set_serial(serial);
    }
}

fn update_positions(start_pos: u64, track: &mut OggTrack) {
    for page in track.pages_mut() {
        let old_pos = page.position();
        page.set_position(start_pos + old_pos);
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
        let EnqueueTrackRequest { mut track } = req;

        try!(validate_positions(&track)
            .map_err(|()| EnqueueTrackError::InvalidTrack));

        try!(check_sample_rate(self.output.clock.sample_rate(), &track)
            .map_err(|()| EnqueueTrackError::BadSampleRate));

        self.output.play_queue.push_back(track);

        Ok(())
    }

    fn track_skip_to_end(&mut self, req: TrackSkipToEndRequest) -> Result<(), TrackSkipToEndError> {
        unimplemented!();
    }

    fn handle_command(&mut self, req_wr: RequestWrapper) {
        let mut binder = CoreBinder { core: self };
        binder.handle_command(req_wr)
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

struct CoreBinder<'a> {
    core: &'a mut Core,
}

impl<'a> CoreBinder<'a> {
    fn handle_command(&mut self, req_wr: RequestWrapper) {
        let RequestWrapper {
            response_queue: response_queue,
            req_type: req_type,
            req_buf: req_buf,
        } = req_wr;
        let response = match req_type {
            RequestType::EnqueueTrack => {
                self.enqueue_track(&req_buf)
            },
            RequestType::TrackSkipToEnd => {
                self.track_skip_to_end(&req_buf)
            },
        };
        response_queue.send(response).unwrap();
    }

    fn enqueue_track(&mut self, req: &[u8]) -> Vec<u8> {
        let res = bincode::serde::deserialize(req)
            .map_err(|_| BinderError::RemoteSerdeError)
            .and_then(|req| {
                self.core.enqueue_track(req)
                    .map_err(BinderError::CallError)
            });

        bincode::serde::serialize(&res, SIZE_LIMIT).unwrap()
    }

    fn track_skip_to_end(&mut self, req: &[u8]) -> Vec<u8> {
         let res: Result<(), BinderError<TrackSkipToEndError>> =
            bincode::serde::deserialize::<TrackSkipToEndRequest>(req)
                .map_err(|_| BinderError::RemoteSerdeError)
                .and_then(|req| {
                    Err(BinderError::StubImplementation)
                });

        bincode::serde::serialize(&res, SIZE_LIMIT).unwrap()
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
    cur_serial: u32,
    clock: OggClock,
    playing_offline: bool,
    buffer: VecDeque<OggPageBuf>,
    play_queue: VecDeque<OggTrackBuf>,
    offline_track: OggTrackBuf,
}

impl OutputManager {
    fn fill_buffer(&mut self) {
        let mut track = match self.play_queue.pop_front() {
            Some(track) => track,
            None => self.offline_track.clone(),
        };

        // not sure why we as_mut instead of just using &mut track
        update_serial(self.cur_serial, track.as_mut());
        self.cur_serial = self.cur_serial.wrapping_add(1);
        update_positions(0, track.as_mut());

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

