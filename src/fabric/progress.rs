use crate::Seconds;

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Progress {
    limit_microseconds: f64,
    current: f64,
}

impl Progress {
    pub fn start(&mut self, seconds: Seconds) {
        self.current = 0.0;
        self.limit_microseconds = (seconds.0 * 1_000_000.0) as f64;
    }

    pub fn step(&mut self, elapsed: f32) -> bool {
        // true if it takes the final step
        let next = self.current + elapsed as f64;
        if next >= self.limit_microseconds {
            self.current = self.limit_microseconds;
            return true;
        }
        self.current = next;
        self.current >= self.limit_microseconds // final step?
    }

    pub fn is_busy(&self) -> bool {
        self.current < self.limit_microseconds
    }

    pub fn nuance(&self) -> f32 {
        if self.limit_microseconds <= 0.0 { // immediate so nuance is already complete
            1.0
        } else {
            (self.current / self.limit_microseconds) as f32
        }
    }
}
