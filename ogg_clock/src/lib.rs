extern crate time;
extern crate ogg;

use ogg::OggPage;
use time::{Duration, SteadyTime};


pub struct AudioClock {
    // in Hz. e.g. 44100, 48000.
    sample_rate: u64,

    start_time: SteadyTime,
}

impl AudioClock {
    pub fn new_with_start(sample_rate: u64, start_time: SteadyTime) -> AudioClock {
        AudioClock {
            sample_rate: sample_rate,
            start_time: start_time,
        }
    }

    pub fn wait_delay(&self, now: SteadyTime, position: u64) -> Duration {
        println!("({}, {}, {})", 1000, position, self.sample_rate);
        let milli_offset = (1000 * position / self.sample_rate) as i64;
        let current_pos = Duration::milliseconds(milli_offset);     
        let sleep_duration = self.start_time - now + current_pos;
        ::std::cmp::max(sleep_duration, Duration::zero())   
    }
}

pub struct OggClock(AudioClock);

impl OggClock {
    pub fn new(sample_rate: u64) -> OggClock {
        OggClock::new_with_start(sample_rate, SteadyTime::now())
    }

    pub fn new_with_start(sample_rate: u64, start_time: SteadyTime) -> OggClock {
        OggClock(AudioClock {
            sample_rate: sample_rate,
            start_time: start_time,
        })
    }

    pub fn wait(&self, page: &OggPage) -> Result<(), ()> {
        let sleep_dur = self.0.wait_delay(SteadyTime::now(), page.position());
        if Duration::zero() < sleep_dur {
            println!("pos = {}", sleep_dur);
            ::std::thread::sleep_ms(sleep_dur.num_milliseconds() as u32);
        }
        Ok(())
    }
}
