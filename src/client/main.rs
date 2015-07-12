#![feature(plugin)]
#![plugin(phf_macros)]

extern crate ogg;
extern crate ogg_clock;
extern crate phf;
extern crate phf_macros;

use std::ffi::OsString;
use std::env; // ::args_os;
use std::process;

mod enqueue;
mod entrypoint;

use entrypoint::EntryPoint;

static ENTRY_POINT_MAP: phf::Map<&'static str, &'static EntryPoint> = phf_map! {
    "enqueue" => &enqueue::ENTRY_POINT,
};

fn print_usage(args: &[OsString]) {
    for (key, val) in ENTRY_POINT_MAP.entries() {
        (val.print_usage)(args);
    }
}

fn main() {
    let args: Vec<OsString> = env::args_os().collect();
    let app_name = args[0].clone();
    let sub_command = args[1].clone().into_string().ok().unwrap();

    let help_args = args.clone();
    if let Some(entry_pt) = ENTRY_POINT_MAP.get(&sub_command[..]) {
        match (entry_pt.main)(args) {
            Ok(()) => (),
            Err(entrypoint::Error::Unspecified(msg)) => {
                println!("subcommand failed: {}", msg);
                process::exit(1);
            },
            Err(entrypoint::Error::InvalidArguments) => {
                (entry_pt.print_usage)(&help_args);
                process::exit(1);
            },
        }
        return;
    }

    print_usage(&args);
    process::exit(1);
}