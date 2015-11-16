use std::io::{self, Read, Write, BufRead, Seek, SeekFrom};
use std::fs::{self, File};
use std::ffi::OsString;
use std::net::TcpStream;

use bincode::serde as bincode;
use byteorder::{self, ReadBytesExt, WriteBytesExt, BigEndian};

use ogg::{OggTrackBuf, OggPageCheckError};
use ireul_interface::proxy::{
    SIZE_LIMIT,
    RequestType,
    EnqueueTrackRequest,
    EnqueueTrackError
};

use ::entrypoint::EntryPoint;
use ::entrypoint::Error as EntryPointError;

pub static ENTRY_POINT: EntryPoint = EntryPoint {
    main: main,
    print_usage: print_usage,
};

impl From<byteorder::Error> for EntryPointError {
    fn from(e: byteorder::Error) -> EntryPointError {
        EntryPointError::Unspecified(format!("{}", e))
    }
}

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
    try!(file.read_to_end(&mut buffer));
    let track = OggTrackBuf::new(buffer).unwrap();

    let mut pages = 0;
    let mut samples = 0;
    for page in track.pages() {
        pages += 1;
        samples = page.position();
    }

    println!("loaded {} samples in {} pages", samples, pages);

    let mut conn = TcpStream::connect("127.0.0.1:3001").unwrap();
    try!(conn.write_u8(0));
    try!(conn.write_u32::<BigEndian>(RequestType::EnqueueTrack.to_op_code()));

    let req = EnqueueTrackRequest { track: track };
    let buf = bincode::serialize(&req, SIZE_LIMIT).unwrap();
    try!(conn.write_u32::<BigEndian>(buf.len() as u32));
    try!(conn.write_all(&buf));

    let mut resp_buf = Vec::new();
    let resp_len = try!(conn.read_u32::<BigEndian>());
    {
        let mut limit_reader = Read::by_ref(&mut conn).take(resp_len as u64);
        try!(limit_reader.read_to_end(&mut resp_buf));
    }

    println!("got response: {:?}", resp_buf);
    try!(conn.write_u8(0));
    try!(conn.write_u32::<BigEndian>(0));

    Ok(())
}

pub fn print_usage(args: &[OsString]) {
    println!("{} enqueue <ogg-file>", args[0].clone().into_string().ok().unwrap());
    println!("");
    println!("    Validates and enqueues the target file");
    println!("");
}
