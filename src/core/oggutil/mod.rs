use ogg::OggPageBuf;
use ogg_clock::OggClock;


#[derive(Clone)]
pub struct OggTrack {
    pub pages: Vec<OggPageBuf>
}
