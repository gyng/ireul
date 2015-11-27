use std::mem;
use std::collections::{VecDeque, HashMap, HashSet};

use rand::{self, Rng, ChaChaRng};

use ogg::{OggTrack, OggTrackBuf, OggPage, OggPageBuf};


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

        let mut new_handle = 0;
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

        self.items.push_back(Track {
            handle: handle,
            data: track.to_owned(),
        });
        Ok(handle)
    }

    pub fn pop_track(&mut self) -> Option<OggTrackBuf> {
        match self.items.pop_front() {
            Some(Track { handle, data }) => {
                self.halloc.dispose(handle).unwrap();
                Some(data)
            },
            None => None,
        }
    }
}

pub enum PlayQueueError {
    Full,
}
