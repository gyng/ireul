use std::ffi::OsString;
use std::process;

use ::entrypoint::{self as ep, EntryPoint as EntryPointTrait};

mod add;
mod show;

pub struct EntryPoint;

unsafe impl Sync for EntryPoint {}

impl ep::EntryPoint for EntryPoint {
    fn main(&self, args: Vec<OsString>) -> Result<(), ep::Error> {
        main(args)
    }

    fn print_usage(&self, args: &[OsString]) {
        print_usage(args)
    }
}

static ENTRY_POINT_MAP: &'static [(&'static str, &'static ep::EntryPoint)] = &[
    ("add", &add::EntryPoint),
    ("show", &show::EntryPoint),
];


fn get_entry_point(name: &str) -> Option<&'static ep::EntryPoint> {
    for &(key, val) in ENTRY_POINT_MAP.iter() {
        if key == name {
            return Some(val);
        }
    }
    None
}

fn main(args: Vec<OsString>) -> Result<(), ep::Error> {
    assert_eq!(&args[1], "queue");
    let sub_command = args[2].clone().into_string().ok().unwrap();

    if let Some(entry_pt) = get_entry_point(&sub_command) {
        return entry_pt.main(args);
    }

    print_usage(&args);
    process::exit(1);
}

fn print_usage(args: &[OsString]) {
    for &(_key, val) in ENTRY_POINT_MAP.iter() {
        val.print_usage(args);
    }
}
