use std::io;

use byteorder::{BigEndian, WriteBytesExt};

use ::proto::{self, Serialize};

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct Handle(pub u64);

pub const STATUS_FIELD_COUNT: u32 = 7;

pub struct Status {
    pub handle: Handle,

    pub artist: String,
    pub album: String,
    pub title: String,

    pub sample_rate: u64,
    pub sample_count: u64,
    pub sample_position: u64,

    pub metadata: Vec<(String, String)>,
}

pub const QUEUE_FIELD_COUNT: u32 = 1;

pub struct Queue {
    // history: Vec<Status>,
    // We'll just include the currently playing song
    // in here.
    upcoming: Vec<Status>,
}

impl Serialize for Handle {
    fn write(&self, buf: &mut io::Cursor<Vec<u8>>) -> io::Result<()> {
        try!(Serialize::write(&self.0, buf));
        Ok(())
    }
}

impl Serialize for Status {
    fn write(&self, buf: &mut io::Cursor<Vec<u8>>) -> io::Result<()> {
        try!(buf.write_u16::<BigEndian>(proto::TYPE_STRUCT));
        try!(buf.write_u32::<BigEndian>(STATUS_FIELD_COUNT));

        try!(Serialize::write("handle", buf));
        try!(Serialize::write(&self.handle, buf));

        try!(Serialize::write("artist", buf));
        try!(Serialize::write(&*self.artist, buf));

        try!(Serialize::write("album", buf));
        try!(Serialize::write(&*self.album, buf));

        try!(Serialize::write("title", buf));
        try!(Serialize::write(&*self.title, buf));

        try!(Serialize::write("sample_rate", buf));
        try!(Serialize::write(&self.sample_rate, buf));

        try!(Serialize::write("sample_count", buf));
        try!(Serialize::write(&self.sample_count, buf));

        try!(Serialize::write("sample_position", buf));
        try!(Serialize::write(&self.sample_position, buf));

        Ok(())
    }
}

impl Serialize for Queue {
    fn write(&self, buf: &mut io::Cursor<Vec<u8>>) -> io::Result<()> {
        try!(buf.write_u16::<BigEndian>(proto::TYPE_STRUCT));
        try!(buf.write_u32::<BigEndian>(QUEUE_FIELD_COUNT));

        try!(Serialize::write("upcoming", buf));
        try!(Serialize::write(&self.upcoming[..], buf));

        Ok(())
    }
}
