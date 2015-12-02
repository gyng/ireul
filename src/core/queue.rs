use std::mem;
use std::collections::{VecDeque, HashMap, HashSet};

use rand::{self, Rng, ChaChaRng};

use ogg::{OggTrack, OggTrackBuf};
use ogg::vorbis::{Comments, VorbisHeader};


#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct Handle(pub u64);

struct HandleAllocator<R> {
    rng: R,
    limit: usize,
    allocated: HashSet<u64>,
}

impl<R> HandleAllocator<R> where R: Rng {
    pub fn new(rng: R, limit: usize) -> HandleAllocator<R> {
        HandleAllocator {
            rng: rng,
            limit: limit,
            allocated: HashSet::new(),
        }
    }

    pub fn generate(&mut self) -> Result<Handle, ()> {
        if self.limit <= self.allocated.len() {
            return Err(());
        }

        let new_handle;
        loop {
            let foo = self.rng.next_u64();
            if !self.allocated.contains(&foo) {
                new_handle = foo;
                break;
            }
        }

        println!("EMITTING HANDLE {:#x}", new_handle);
        self.allocated.insert(new_handle);
        Ok(Handle(new_handle))
    }

    pub fn dispose(&mut self, handle: Handle) -> Result<(), Handle> {
        match self.allocated.remove(&handle.0) {
            true => Ok(()),
            false => Err(handle),
        }
    }
}

struct Track {
    handle: Handle,
    data: OggTrackBuf,
    comments: Option<Comments>,
}

pub struct PlayQueue {
    halloc: HandleAllocator<ChaChaRng>,
    items: VecDeque<Track>,
}

impl PlayQueue {
    pub fn new(limit: usize) -> PlayQueue {
        let rng: ChaChaRng = rand::thread_rng().gen();
        PlayQueue {
            halloc: HandleAllocator::new(rng, limit),
            items: VecDeque::new(),
        }
    }

    pub fn reorder(&mut self, handle_ord: &[Handle]) {
        let old_items: VecDeque<Track> = mem::replace(&mut self.items, VecDeque::new());

        let mut map: HashMap<Handle, Track> = old_items.into_iter().map(|pq| (pq.handle, pq)).collect();

        for handle in handle_ord.iter() {
            match map.remove(handle) {
                Some(item) => {
                    self.items.push_back(item);
                },
                None => (),
            }
        }
    }

    pub fn remove_by_handle(&mut self, handle: Handle) {
        let old_items: VecDeque<Track> = mem::replace(&mut self.items, VecDeque::new());
        for item in old_items.into_iter() {
            if item.handle != handle {
                self.items.push_back(item);
            }
        }
    }

    pub fn add_track(&mut self, track: &OggTrack) -> Result<Handle, PlayQueueError> {
        let handle = try!(self.halloc.generate()
                .map_err(|()| PlayQueueError::Full));

        let comments = match VorbisHeader::find_comments(track.pages()) {
            Ok(header) => header.comments(),
            Err(_) => None
        };

        self.items.push_back(Track {
            handle: handle,
            data: track.to_owned(),
            comments: comments
        });
        Ok(handle)
    }

    pub fn pop_track(&mut self) -> Option<OggTrackBuf> {
        match self.items.pop_front() {
            Some(Track { handle, data, comments }) => {
                self.halloc.dispose(handle).unwrap();
                Some(data)
            },
            None => None,
        }
    }

    pub fn get_queue_comments_by_fields(&mut self, fields: &[&str]) -> Vec<Vec<(String, String)>> {
        self.items.iter().map(|track| {
            track.comments.as_ref().unwrap().comments.iter().filter_map(|comment|
                match fields.iter().any(|&field| *field == comment.0) {
                    true => Some(comment.clone()),
                    false => None
                }
            ).collect()
        }).collect()
    }
}

pub enum PlayQueueError {
    Full,
}

#[cfg(test)]
mod test {
    use ogg::OggTrack;
    use super::PlayQueue;

    static SAMPLE_OGG: &'static [u8] = include_bytes!("../../ogg/testdata/Hydrate-Kenny_Beltrey.ogg");

    #[test]
    fn test_get_queue_comments_by_fields() {
        let mut queue = PlayQueue::new(2);
        let track1 = OggTrack::new(SAMPLE_OGG).unwrap();
        let track2 = OggTrack::new(SAMPLE_OGG).unwrap();

        queue.add_track(&track1).ok();
        queue.add_track(&track2).ok();

        let expected = vec!(
            vec!(
                ("TITLE".to_string(), "Hydrate - Kenny Beltrey".to_string()),
                ("ARTIST".to_string(), "Kenny Beltrey".to_string())
            ),
            vec!(
                ("TITLE".to_string(), "Hydrate - Kenny Beltrey".to_string()),
                ("ARTIST".to_string(), "Kenny Beltrey".to_string())
            )
        );

        let got = queue.get_queue_comments_by_fields(&["ARTIST", "TITLE", "NOTAFIELD"]);
        assert_eq!(expected, got);
    }
}
