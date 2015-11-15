use serde;

use ogg::OggPageBuf;

pub struct OggTrack {
    pub pages: Vec<OggPageBuf>
}

// impl serde::Serialize for OggTrack {
//     fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
//         where S: serde::Serializer,
//     {
//         let mut bigbuf = Vec::new();
//         for page in self.pages.iter() {
//             bigbuf.extend(page.as_u8_slice());
//         }
//         serializer.visit_byte_buf(bigbuf)
//     }
// }

// impl serde::Deserialize for OggTrack {
//     fn deserialize<D>(deserializer: &mut D) -> Result<OggTrack, D::Error>
//         where D: serde::Deserializer,
//     {
//         deserializer.visit(OggTrackVisitor)
//     }
// }
