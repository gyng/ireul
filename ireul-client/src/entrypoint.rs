use std::io;
use std::ffi::OsString;

use byteorder;
use ogg::OggPageCheckError;

pub enum Error {
    InvalidArguments,
    Unspecified(String),
}

pub trait EntryPoint: Sync {
    fn main(&self, args: Vec<OsString>) -> Result<(), Error>;

    fn print_usage(&self, args: &[OsString]);
}

impl From<byteorder::Error> for Error {
    fn from(e: byteorder::Error) -> Error {
        Error::Unspecified(format!("{}", e))
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Unspecified(format!("{}", e))
    }
}

impl From<OggPageCheckError> for Error {
    fn from(e: OggPageCheckError) -> Error {
        Error::Unspecified(format!("{:?}", e))
    }
}
