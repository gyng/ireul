extern crate time;
extern crate ogg;

use ogg::OggPage;
use time::{Duration, SteadyTime};


pub struct AudioClock {
    // in Hz. e.g. 44100, 48000.
    sample_rate: u32,

    start_time: SteadyTime,
}

impl AudioClock {
    pub fn new_with_start(sample_rate: u32, start_time: SteadyTime) -> AudioClock {
        AudioClock {
            sample_rate: sample_rate,
            start_time: start_time,
        }
    }

    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn wait_delay(&self, now: SteadyTime, position: u64) -> Duration {
        let sample_rate = self.sample_rate as u64;
        let milli_offset = (1000 * position / sample_rate) as i64;
        let current_pos = Duration::milliseconds(milli_offset);
        let sleep_duration = self.start_time - now + current_pos;
        ::std::cmp::max(sleep_duration, Duration::zero())
    }
}

pub struct OggClock {
    clock: AudioClock,
    last_pos: u64,
    base_pos: u64,
}

impl OggClock {
    pub fn new(sample_rate: u32) -> OggClock {
        OggClock::new_with_start(sample_rate, SteadyTime::now())
    }

    pub fn new_with_start(sample_rate: u32, start_time: SteadyTime) -> OggClock {
        OggClock {
            clock: AudioClock {
                sample_rate: sample_rate,
                start_time: start_time,
            },
            last_pos: 0,
            base_pos: 0,
        }
    }

    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.clock.sample_rate()
    }

    pub fn wait_duration(&mut self, page: &OggPage) -> Duration {
        let new_pos = page.position();
        if self.base_pos + new_pos < self.last_pos {
            self.base_pos = self.last_pos;
        }

        let abs_pos = self.base_pos + new_pos;
        self.last_pos = abs_pos;
        self.clock.wait_delay(SteadyTime::now(), abs_pos)
    }

    pub fn wait(&mut self, page: &OggPage) -> Result<(), ()> {
        let new_pos = page.position();
        if self.base_pos + new_pos < self.last_pos {
            self.base_pos = self.last_pos;
        }

        let abs_pos = self.base_pos + new_pos;
        self.last_pos = abs_pos;
        let sleep_dur = self.clock.wait_delay(SteadyTime::now(), abs_pos);

        if Duration::zero() < sleep_dur {
            ::std::thread::sleep_ms(sleep_dur.num_milliseconds() as u32);
        }

        Ok(())
    }
}
