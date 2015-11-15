use std::net::TcpStream;
use std::io::{self, Write};

use rustc_serialize::base64::{ToBase64, MIME};
use ogg::OggPage;

// TODO: Move mountpoint information into some serializable thing
// https://gist.github.com/ePirat/adc3b8ba00d85b7e3870#specifying-mountpoint-information
pub struct IceCastWriterOptions {
    pub endpoint: &'static str,
    pub mount: &'static str, // should start with a `/` eg. `/mymount.ogg`
    pub user_pass: Option<&'static str>, // username:password
    pub name: Option<&'static str>,
    pub description: Option<&'static str>,
    pub url: Option<&'static str>,
    pub genre: Option<&'static str>,
}

impl Default for IceCastWriterOptions {
    fn default() -> IceCastWriterOptions {
        IceCastWriterOptions {
            endpoint: "127.0.0.1:8000",
            mount: "/mountpoint.ogg",
            user_pass: None,
            name: None,
            description: None,
            url: None,
            genre: None,
        }
    }
}

pub struct IceCastWriter {
    stream: TcpStream,
    options: IceCastWriterOptions,
}

impl IceCastWriter {
    pub fn new(options: IceCastWriterOptions) -> io::Result<IceCastWriter> {
        // IceCast drops connection if mountpoint does not begin with `/`
        assert!(options.mount.as_bytes()[0] == b'/');

        let stream = match TcpStream::connect(options.endpoint) {
            Ok(stream) => stream,
            Err(err) => return Err(err)
        };

        let mut writer = IceCastWriter {
            stream: stream,
            options: options
        };

        try!(writer.send_header());

        Ok(writer)
    }

    fn send_header(&mut self) -> io::Result<()> {
        try!(write!(self.stream, "SOURCE {} HTTP/1.0\r\n", self.options.mount));

        if let Some(user_pass) = self.options.user_pass {
            let mut config = MIME;
            config.line_length = None;
            try!(write!(self.stream, "Authorization: Basic {}\r\n", user_pass.as_bytes().to_base64(config)));
        }

        try!(write!(self.stream, "Host: {}\r\n", self.options.endpoint)); // TODO: Verify this
        try!(write!(self.stream, "Accept: */*"));
        try!(write!(self.stream, "User-Agent: ireul\r\n"));
        try!(write!(self.stream, "Ice-Public: 1\r\n"));

        if let Some(name) = self.options.name {
            try!(write!(self.stream, "Ice-Name: {}\r\n", name));
        }

        if let Some(description) = self.options.description {
            try!(write!(self.stream, "Ice-Description: {}\r\n", description));
        }

        if let Some(url) = self.options.url {
            try!(write!(self.stream, "Ice-Url: {}\r\n", url));
        }

        if let Some(genre) = self.options.genre {
            try!(write!(self.stream, "Ice-Genre: {}\r\n", genre));
        }

        // Do not reorder Content-Type! Somehow IceCast treated it as audio/mpeg when moved up
        try!(write!(self.stream, "Content-Type: audio/ogg\r\n"));

        // IceCast responds with some headers and a HTTP OK, but they're not parsed yet
        self.stream.write_all(b"\r\n")
    }

    pub fn send_ogg_page(&mut self, page: &OggPage) -> io::Result<()> {
        self.stream.write_all(page.as_u8_slice())
    }
}

#[cfg(test)]
mod test {
    extern crate ogg;

    use std::fs::File;
    use std::io::{self, Read};

    use ogg::OggTrackBuf;
    use ogg_clock::OggClock;

    use super::{IceCastWriter, IceCastWriterOptions};

    #[test]
    #[ignore]
    pub fn push_ogg_to_icecast() {
        let mut file = io::BufReader::new(File::open("howbigisthis_repaired.ogg").unwrap());
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        let track = OggTrackBuf::new(buffer).unwrap();

        let mut writer = IceCastWriter::new(IceCastWriterOptions {
            endpoint: "lollipop.hiphop:8000",
            mount: "/howbigisthis.ogg",
            user_pass: Some("user:pass"),
            name: Some("lollipop.hipop radio"),
            description: Some("Hiphop to the lollipop bop"),
            ..Default::default()
        }).unwrap();

        let clock = OggClock::new(48000);
        for page in track.pages() {
            writer.send_ogg_page(&page).unwrap();
            clock.wait(&page).unwrap();
        }
    }
}
