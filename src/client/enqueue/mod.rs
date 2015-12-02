use std::io::{self, Read, Write, BufRead, Seek, SeekFrom};
use std::fs::{self, File};
use std::ffi::OsString;
use std::net::TcpStream;

use byteorder::{self, ReadBytesExt, WriteBytesExt, BigEndian};

use ogg::{OggTrackBuf, OggPageCheckError};
use ireul_interface::proxy::{
    RequestType,
    EnqueueTrackRequest,
    EnqueueTrackResult,
    EnqueueTrackError,
};

use ::entrypoint::{self, Error as EntryPointError};

pub struct EntryPoint;

unsafe impl Sync for EntryPoint {}

impl ::entrypoint::EntryPoint for EntryPoint {
    fn main(&self, args: Vec<OsString>) -> Result<(), EntryPointError> {
        main(args)
    }

    fn print_usage(&self, args: &[OsString]) {
        print_usage(args)
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
    let slice = track.as_u8_slice();
    try!(conn.write_u32::<BigEndian>(slice.len() as u32));
    try!(conn.write_all(&slice));

    let frame_length = try!(conn.read_u32::<BigEndian>());
    let mut resp_buf = Vec::new();
    {
        let mut limit_reader = Read::by_ref(&mut conn).take(frame_length as u64);
        try!(limit_reader.read_to_end(&mut resp_buf));
    }

    let mut frame = io::Cursor::new(&resp_buf[..]);
    let res: EnqueueTrackResult = match try!(frame.read_u8()) {
        0 => {
            let handle = try!(frame.read_u64::<BigEndian>());
            println!("got handle {:#x}", handle);
            Ok(handle)
        },
        _ => {
            let errno = try!(frame.read_u32::<BigEndian>());
            let errstr_len = try!(frame.read_u32::<BigEndian>());
            let mut errstr = String::new();
            {
                let mut limit_reader = Read::by_ref(&mut frame).take(errstr_len as u64);
                try!(limit_reader.read_to_string(&mut errstr));
            }
            Err(EnqueueTrackError::from_u32(errno).unwrap())
        }
    };
    println!("got response: {:?}", res);

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
