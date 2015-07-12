extern crate mio;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::io;
use std::num::Wrapping;


use mio::unix::{UnixStream, UnixListener};
use mio::{EventLoop, EventLoopConfig, Token, EventSet, PollOpt};

fn main() {
	Handler::new("/tmp/ireul-core").unwrap().start().unwrap();
}

const CONTROL_SOCKET: Token = Token(1);

enum CoreNotify {}

enum Timeout {}

struct Client {
	conn: UnixStream,
}

impl Client {
	fn new(conn: UnixStream) -> Client {
		Client { conn: conn }
	}

	fn ready(&mut self, eloop: &mut EventLoop<Handler>, eset: EventSet) -> Result<(), ()> {
		Err(())
	}
}

struct OggTrack;

/// Connects to IceCast and holds references to streamable content.
struct OutputManager {
	play_queue: VecDeque<OggTrack>,
	offline_track: OggTrack,
}

struct Handler {
	control: UnixListener,
	output: OutputManager,
	next_token: Wrapping<usize>,
	tokens_in_use: HashSet<Token>,
	clients: HashMap<Token, Client>,
}

impl Handler {
	pub fn new<P: AsRef<Path>>(addr: P) -> io::Result<Handler> {
		let addr: &Path = addr.as_ref();
		let control_socket = try!(UnixListener::bind(addr));

		let mut tokens_in_use = HashSet::new();
		tokens_in_use.insert(Token(0));
		tokens_in_use.insert(CONTROL_SOCKET);

		Ok(Handler {
			control: control_socket,
			output: OutputManager {
				play_queue: VecDeque::new(),
				offline_track: OggTrack,
			},
			next_token: Wrapping(0),
			tokens_in_use: tokens_in_use,
			clients: HashMap::new(),
		})
	}

	fn next_token(&mut self) -> Token {
		loop {
			let current = Token(self.next_token.0);
			self.next_token = self.next_token + Wrapping(1);
			if !self.tokens_in_use.contains(&current) {
				return current;
			}
		}
	}

	pub fn start(&mut self) -> io::Result<()> {
		let mut event_loop = try!(EventLoop::configured(EventLoopConfig {
			io_poll_timeout_ms: 60000,
			timer_tick_ms: 10000,
			.. EventLoopConfig::default()
		}));
		event_loop.register(&self.control, CONTROL_SOCKET).unwrap();
		event_loop.reregister(&self.control, CONTROL_SOCKET, EventSet::readable(), PollOpt::empty()).unwrap();
		Ok(try!(event_loop.run(self)))
	}
}

impl ::mio::Handler for Handler {
	type Timeout = Timeout;
	type Message = CoreNotify;

	fn notify(&mut self, eloop: &mut EventLoop<Handler>, msg: CoreNotify) {
		//
	}
	
	fn ready(&mut self, eloop: &mut EventLoop<Handler>, token: Token, eset: EventSet) {
		if token == CONTROL_SOCKET {
			if let Ok(Some(csock)) = self.control.accept() {
				let client_token = self.next_token();
				println!("allocated {:?} for {:?}", client_token, csock);
				eloop.register(&csock, client_token).unwrap();
				self.clients.insert(client_token, Client::new(csock));
			}
			return;
		}

		let mut eliminate_client: bool = false;
		if let Some(client) = self.clients.get_mut(&token) {
			if let Err(err) = client.ready(eloop, eset) {
				println!("client erorr'd: {:?}", err);
				eliminate_client = true;
			}
		}
		if eliminate_client {
			self.clients.remove(&token);
		}
	}

	fn timeout(&mut self, eloop: &mut EventLoop<Handler>, token: Timeout) {
		//
	}
}