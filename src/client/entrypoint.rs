use std::ffi::OsString;

pub enum Error {
	InvalidArguments,
	Unspecified(String),
}

pub struct EntryPoint {
	pub main: fn(Vec<OsString>) -> Result<(), Error>,
	pub print_usage: fn(&[OsString])
}

