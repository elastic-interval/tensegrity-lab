use crate::units::Seconds;
use std::time::Duration;

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Progress {
    limit: Duration,
    current: Duration,
}

impl Progress {
    pub fn start(&mut self, seconds: Seconds) {
        self.current = Duration::ZERO;
        self.limit = Duration::from_secs_f32(seconds.0);
    }

    pub fn step(&mut self, elapsed: Duration) -> bool {
        // true if it takes the final step
        let next = self.current + elapsed;
        if next >= self.limit {
            self.current = self.limit;
            return true;
        }
        self.current = next;
        self.current >= self.limit // final step?
    }

    pub fn is_busy(&self) -> bool {
        self.current < self.limit
    }

    pub fn nuance(&self) -> f32 {
        if self.limit.is_zero() { // immediate so nuance is already complete
            1.0
        } else {
            self.current.as_secs_f32() / self.limit.as_secs_f32()
        }
    }

    /// Returns a countdown from 10 to 0 based on progress
    pub fn countdown(&self) -> i32 {
        ((1.0 - self.nuance()) * 10.0).ceil().max(0.0) as i32
    }
}
