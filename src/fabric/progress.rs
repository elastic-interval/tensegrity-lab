#[derive(Clone, Default, Debug, PartialEq)]
pub struct Progress {
    limit: usize,
    count: usize,
}

impl Progress {
    pub fn start(&mut self, countdown: usize) {
        self.count = 0;
        self.limit = countdown;
    }

    pub fn step(&mut self) -> bool {
        // true if it takes the final step
        let count = self.count + 1;
        if count > self.limit {
            return false;
        }
        self.count = count;
        self.count == self.limit // final step?
    }

    pub fn is_busy(&self) -> bool {
        self.count < self.limit
    }

    pub fn nuance(&self) -> f32 {
        if self.limit == 0 {
            1.0
        } else {
            (self.count as f32) / (self.limit as f32)
        }
    }
}
