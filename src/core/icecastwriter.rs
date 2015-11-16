use std::net::TcpStream;
use std::io::{self, Write};

use rustc_serialize::base64::{ToBase64, MIME};
use ogg::OggPage;

// TODO: Move mountpoint information into some serializable thing
// https://gist.github.com/ePirat/adc3b8ba00d85b7e3870#specifying-mountpoint-information
pub struct IceCastWriterOptions {
    endpoint: String,
    mount: String, // should start with a `/` eg. `/mymount.ogg`
    user_pass: Option<String>, // username:password
    name: Option<String>,
    description: Option<String>,
    url: Option<String>,
    genre: Option<String>,
}

impl IceCastWriterOptions {
    pub fn set_endpoint(&mut self, endpoint: &str) -> &mut Self {
        self.endpoint = endpoint.to_string();
        self
    }

    pub fn set_mount(&mut self, mount: &str) -> &mut Self {
        self.mount = mount.to_string();
        self
    }

    pub fn set_user_pass(&mut self, user_pass: &str) -> &mut Self {
        self.user_pass = Some(user_pass.to_string());
        self
    }

    pub fn set_name(&mut self, name: &str) -> &mut Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn set_description(&mut self, description: &str) -> &mut Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn set_url(&mut self, url: &str) -> &mut Self {
        self.url = Some(url.to_string());
        self
    }

    pub fn set_genre(&mut self, genre: &str) -> &mut Self {
        self.genre = Some(genre.to_string());
        self
    }
}

impl Default for IceCastWriterOptions {
    fn default() -> IceCastWriterOptions {
        IceCastWriterOptions {
            endpoint: "127.0.0.1:8000".to_string(),
            mount: "/mountpoint.ogg".to_string(),
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

        // let endpoint: &str = &options.endpoint;
        let stream = match TcpStream::connect(&options.endpoint as &str) {
            Ok(stream) => stream,
            Err(err) => return Err(err)
        };

        let mut writer = IceCastWriter {
            stream: stream,
            options: options,
        };

        try!(writer.send_header());

        Ok(writer)
    }

    fn send_header(&mut self) -> io::Result<()> {
        try!(write!(self.stream, "SOURCE {} HTTP/1.0\r\n", self.options.mount));

        if let Some(ref user_pass) = self.options.user_pass {
            let mut config = MIME;
            config.line_length = None;
            try!(write!(self.stream, "Authorization: Basic {}\r\n", user_pass.as_bytes().to_base64(config)));
        }

        try!(write!(self.stream, "Host: {}\r\n", self.options.endpoint)); // TODO: Verify this
        try!(write!(self.stream, "Accept: */*"));
        try!(write!(self.stream, "User-Agent: ireul\r\n"));
        try!(write!(self.stream, "Ice-Public: 1\r\n"));

        if let Some(ref name) = self.options.name {
            try!(write!(self.stream, "Ice-Name: {}\r\n", name));
        }

        if let Some(ref description) = self.options.description {
            try!(write!(self.stream, "Ice-Description: {}\r\n", description));
        }

        if let Some(ref url) = self.options.url {
            try!(write!(self.stream, "Ice-Url: {}\r\n", url));
        }

        if let Some(ref genre) = self.options.genre {
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

        let mut icecastopts = IceCastWriterOptions::default();
        icecastopts
            .set_endpoint("lollipop.hiphop:8000")
            .set_mount("/howbigisthis.ogg")
            .set_user_pass("user:pass")
            .set_name("lollipop.hiphop radio")
            .set_description("Hiphop to the lollipop bop");

        let mut writer = IceCastWriter::new(icecastopts).unwrap();
        let clock = OggClock::new(48000);
        for page in track.pages() {
            writer.send_ogg_page(&page).unwrap();
            clock.wait(&page).unwrap();
        }
    }
}
