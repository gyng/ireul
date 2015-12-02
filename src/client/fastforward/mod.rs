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
    FastForwardRequest,
    FastForwardResult,
    FastForwardError,
};

use ::entrypoint::EntryPoint;
use ::entrypoint::Error as EntryPointError;

pub static ENTRY_POINT: EntryPoint = EntryPoint {
    main: main,
    print_usage: print_usage,
};


#[derive(Debug)]
struct ProgramArgs {
    app_name: OsString,
}

impl ProgramArgs {
    pub fn new(args: Vec<OsString>) -> Result<ProgramArgs, EntryPointError> {
        if args.len() < 2 {
            return Err(EntryPointError::InvalidArguments);
        }

        let app_name = args[0].clone();
        assert_eq!(&args[1], "fast-forward");
        Ok(ProgramArgs {
            app_name: app_name,
        })
    }
}


pub fn main(args: Vec<OsString>) -> Result<(), EntryPointError> {
    let app_name = args[0].clone();
    let args = try!(ProgramArgs::new(args));

    let mut conn = TcpStream::connect("127.0.0.1:3001").unwrap();
    try!(conn.write_u8(0));
    try!(conn.write_u32::<BigEndian>(RequestType::FastForward.to_op_code()));
    try!(conn.write_u32::<BigEndian>(4));
    try!(conn.write_u32::<BigEndian>(0));

    let res: FastForwardResult = match try!(conn.read_u8()) {
        0 => Ok(()),
        _ => {
            let errno = try!(conn.read_u32::<BigEndian>());
            let errstr_len = try!(conn.read_u32::<BigEndian>());
            let mut errstr = String::new();
            {
                let mut limit_reader = Read::by_ref(&mut conn).take(errstr_len as u64);
                try!(limit_reader.read_to_string(&mut errstr));
            }
            Err(FastForwardError)
            // Err(FastForwardError::from_u32(errno).unwrap())
        }
    };
    println!("got response: {:?}", res);
    try!(conn.write_u8(0));
    try!(conn.write_u32::<BigEndian>(0));
    Ok(())
}

pub fn print_usage(args: &[OsString]) {
    println!("{} fast-forward", args[0].clone().into_string().ok().unwrap());
    println!("");
    println!("    Skips the currently-playing track");
    println!("");
}
