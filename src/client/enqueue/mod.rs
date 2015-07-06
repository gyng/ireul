use std::io::{self, Read, BufRead, Seek, SeekFrom};
use std::fs::{self, File};
use std::ffi::OsString;

use ogg::{OggPage, OggPageCheckError};

use ::entrypoint::EntryPoint;
use ::entrypoint::Error as EntryPointError;

pub static ENTRY_POINT: EntryPoint = EntryPoint {
	main: main,
	print_usage: print_usage,
};

impl From<io::Error> for EntryPointError {
	fn from(e: io::Error) -> EntryPointError {
		EntryPointError::Unspecified(format!("{}", e))
	}
}

impl From<OggPageCheckError> for EntryPointError {
	fn from(e: OggPageCheckError) -> EntryPointError {
		EntryPointError::Unspecified(format!("{:?}", e))
	}
}


#[derive(Debug)]
struct ProgramArgs {
	app_name: OsString,
	target_file: OsString,
}

impl ProgramArgs {
	pub fn new(args: Vec<OsString>) -> Result<ProgramArgs, EntryPointError> {
		if args.len() < 3 {
			return Err(EntryPointError::InvalidArguments);
		}

		let app_name = args[0].clone();
		assert_eq!(&args[1], "enqueue");
		let target_file = args[2].clone();

		Ok(ProgramArgs {
			app_name: app_name,
			target_file: target_file,
		})
	}
}


pub fn main(args: Vec<OsString>) -> Result<(), EntryPointError> {
	let app_name = args[0].clone();
	let args = try!(ProgramArgs::new(args));

	let mut file = io::BufReader::new(try!(File::open(&args.target_file)));
	let mut buffer = Vec::new();
	let _ = try!(file.read_to_end(&mut buffer));

	let mut pages = 0;
	let mut offset = 0;
	let mut samples = 0;
	while offset < buffer.len() {
		let page = try!(OggPage::new(&buffer[offset..]));
		offset += page.as_u8_slice().len();
		pages += 1;
		samples = page.position();
	}
	try!(file.seek(SeekFrom::Start(0)));
	
	println!("loaded ~~{} samples in {} pages", samples, pages);
	Ok(())
}

pub fn print_usage(args: &[OsString]) {
	println!("{} enqueue <ogg-file>", args[0].clone().into_string().ok().unwrap());
	println!("");
	println!("    Validates and enqueues the target file");
	println!("");
}