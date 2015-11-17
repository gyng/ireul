use std::net::TcpStream;
use std::io::{self, Write};

use url;
use rustc_serialize::base64::{ToBase64, MIME};

use ogg::OggPage;

// TODO: Move mountpoint information into some serializable thing
// https://gist.github.com/ePirat/adc3b8ba00d85b7e3870#specifying-mountpoint-information
#[derive(Debug)]
pub struct IceCastWriterOptions {
    host: String,
    port: u16,
    mount: String, // should start with a `/` eg. `/mymount.ogg`
    user: Option<String>,
    password: Option<String>,
    name: Option<String>,
    description: Option<String>,
    url: Option<String>,
    genre: Option<String>,
}

impl IceCastWriterOptions {
    pub fn from_url(url: &url::Url) -> Result<IceCastWriterOptions, &'static str> {
        if url.scheme != "http" {
            return Err("scheme must be http");
        }

        let mut opts = IceCastWriterOptions::default();
        if let Some(domain) = url.domain() {
            opts.set_host(domain);
        }
        if let Some(port) = url.port() {
            opts.set_port(port);
        }
        if let Some(ref path) = url.serialize_path() {
            opts.set_mount(path);
        }
        if let Some(user) = url.username() {
            opts.set_user(user);
        }
        if let Some(password) = url.password() {
            opts.set_password(password);
        }
        Ok(opts)
    }

    pub fn set_host(&mut self, host: &str) -> &mut Self {
        self.host = host.to_string();
        self
    }

    pub fn set_port(&mut self, port: u16) -> &mut Self {
        self.port = port;
        self
    }

    fn endpoint(&self) -> (&str, u16) {
        (&self.host, self.port)
    }

    fn endpoint_name(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn set_mount(&mut self, mount: &str) -> &mut Self {
        self.mount = mount.to_string();
        self
    }

    pub fn set_user(&mut self, user: &str) -> &mut Self {
        self.user = Some(user.to_string());
        self
    }

    pub fn set_password(&mut self, password: &str) -> &mut Self {
        self.password = Some(password.to_string());
        self
    }

    fn get_userpass(&self) -> Option<String> {
        let mut buf = String::new();
        let mut emit = false;
        if let Some(ref user) = self.user {
            emit = true;
            buf.push_str(user);
        }
        if let Some(ref password) = self.password {
            buf.push_str(":");
            buf.push_str(password);
            emit = true;
        }
        if emit {
            Some(buf)
        } else {
            None
        }
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
            host: "127.0.0.1".to_string(),
            port: 8000,
            mount: "/mountpoint.ogg".to_string(),
            user: None,
            password: None,
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

        let stream = match TcpStream::connect(options.endpoint()) {
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

        if let Some(ref user_pass) = self.options.get_userpass() {
            let mut config = MIME;
            config.line_length = None;
            try!(write!(self.stream, "Authorization: Basic {}\r\n", user_pass.as_bytes().to_base64(config)));
        }


        try!(write!(self.stream, "Host: {}\r\n", self.options.endpoint_name())); // TODO: Verify this
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
