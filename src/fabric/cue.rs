//! A cue for the wobble table's stepper: a loop of DMX frames, each the
//! crank's position, 0 to 255 round its turn with 128 its zero, the crank
//! pointing forward along the piece's mirror plane. Played at `DMX_RATE`
//! frames a second, round and round, the stepper going from each position to
//! the next the short way round, so a cue can rock the crank, turn it round
//! and round, or anything between. It is what the electronics play.
//!
//! Cues are bred: from simple seeds, rocking and turning at a spread of
//! speeds, by mutation and crossing.

use rand::Rng;
use std::f32::consts::TAU;

/// DMX frames a second.
pub const DMX_RATE: f32 = 44.0;
/// The crank's zero, as a DMX value.
pub const ZERO: u8 = 128;
/// The shortest and longest loops bred, in frames.
const FRAMES: (usize, usize) = (16, 176);
/// The stepper's top speed, turns a second.
pub const TOP_TURNS: f32 = 4.0;
/// How far the stepper can go in a frame, in DMX values.
const FARTHEST: f32 = 256.0 * TOP_TURNS / DMX_RATE;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cue {
    pub frames: Vec<u8>,
}

/// From DMX value `from` to `to` the short way round, in values.
fn step(from: u8, to: u8) -> f32 {
    to.wrapping_sub(from) as i8 as f32
}

/// `value` moved by `by` values, round the turn.
fn moved(value: u8, by: f32) -> u8 {
    (value as f32 + by).round().rem_euclid(256.0) as u8
}

impl Cue {
    /// The crank's angle from its zero for DMX value `value`.
    pub fn angle(value: f32) -> f32 {
        (value - ZERO as f32) / 256.0 * TAU
    }

    /// How long the loop lasts.
    pub fn seconds(&self) -> f32 {
        self.frames.len() as f32 / DMX_RATE
    }

    /// The DMX value at `seconds` into the loop, between frames on the
    /// short way from one to the next, round the turn.
    pub fn value(&self, seconds: f32) -> f32 {
        let at = (seconds * DMX_RATE).rem_euclid(self.frames.len() as f32);
        let index = at.floor() as usize % self.frames.len();
        let from = self.frames[index];
        let to = self.frames[(index + 1) % self.frames.len()];
        (from as f32 + step(from, to) * at.fract()).rem_euclid(256.0)
    }

    /// How many times round the loop turns the crank.
    pub fn turns(&self) -> i32 {
        let steps: f32 = (0..self.frames.len()).map(|i| step(self.frames[i], self.frames[(i + 1) % self.frames.len()])).sum();
        (steps / 256.0).round() as i32
    }

    /// What the stepper is told, tersely: the loop's length in frames, how
    /// many times round it turns if it does (`↻1`), where it starts, and
    /// then each position it turns back at with the frames it takes to get
    /// there, the last back where it started: `34f 130 149/4 107/8 130/22`.
    /// Turning back by less than `WAVER` is not counted.
    pub fn describe(&self) -> String {
        const WAVER: f32 = 3.0;
        let frames = self.frames.len();
        let steps: Vec<f32> = (0..frames).map(|i| step(self.frames[i], self.frames[(i + 1) % frames])).collect();
        let mut legs: Vec<(f32, usize)> = Vec::new();
        for &step in &steps {
            match legs.last_mut() {
                Some((travel, count)) if step == 0.0 || travel.signum() == step.signum() || *travel == 0.0 => {
                    *travel += step;
                    *count += 1;
                }
                _ => legs.push((step, 1)),
            }
        }
        let mut merged: Vec<(f32, usize)> = Vec::new();
        for (travel, count) in legs {
            match merged.last_mut() {
                Some(last) if travel.abs() < WAVER || last.0.signum() == travel.signum() => {
                    last.0 += travel;
                    last.1 += count;
                }
                _ => merged.push((travel, count)),
            }
        }
        let mut at = self.frames[0] as f32;
        let mut words = vec![format!("{frames}f")];
        if self.turns() != 0 {
            words.push(format!("↻{}", self.turns()));
        }
        words.push(self.frames[0].to_string());
        if merged.len() > 1 {
            for (travel, count) in merged {
                at = (at + travel).rem_euclid(256.0);
                words.push(format!("{}/{count}", at.round() as u8));
            }
        }
        words.join(" ")
    }

    /// A cue from its DMX frames, written as numbers apart.
    pub fn parse(text: &str) -> Option<Cue> {
        let frames: Vec<u8> = text.split_whitespace().map(str::parse).collect::<Result<_, _>>().ok()?;
        (frames.len() >= FRAMES.0).then_some(Cue { frames })
    }

    /// Rocking `degrees` each way, `hertz` times a second.
    pub fn rocking(hertz: f32, degrees: f32) -> Cue {
        let (frames, cycles) = Self::span(hertz);
        let reach = degrees / 360.0 * 256.0;
        Cue {
            frames: (0..frames)
                .map(|i| moved(ZERO, reach * (TAU * cycles as f32 * i as f32 / frames as f32).sin()))
                .collect(),
        }
    }

    /// Turning round `hertz` times a second.
    pub fn turning(hertz: f32) -> Cue {
        let (frames, cycles) = Self::span(hertz);
        Cue { frames: (0..frames).map(|i| moved(ZERO, 256.0 * cycles as f32 * i as f32 / frames as f32)).collect() }
    }

    /// A loop of whole cycles at `hertz`, at least 32 frames long.
    fn span(hertz: f32) -> (usize, usize) {
        let cycles = (32.0 * hertz / DMX_RATE).ceil().max(1.0) as usize;
        ((cycles as f32 * DMX_RATE / hertz).round() as usize, cycles)
    }

    /// Where breeding starts: rocking at a spread of speeds, and turning.
    pub fn seeds() -> Vec<Cue> {
        let rocking = [0.6, 0.8, 1.0, 1.3, 1.6, 2.0, 2.3, 2.6, 2.9].map(|hertz| Cue::rocking(hertz, 30.0));
        let turning = [1.3, 2.1, 2.6].map(Cue::turning);
        rocking.into_iter().chain(turning).collect()
    }

    /// The value at `index` of this loop resampled to `frames` frames.
    fn resampled(&self, frames: usize) -> Cue {
        let seconds = self.seconds();
        Cue {
            frames: (0..frames)
                .map(|i| self.value(seconds * i as f32 / frames as f32).round().rem_euclid(256.0) as u8)
                .collect(),
        }
    }

    /// Whether the stepper can follow it, from each frame to the next.
    pub fn followable(&self) -> bool {
        let frames = self.frames.len();
        (0..frames).all(|i| step(self.frames[i], self.frames[(i + 1) % frames]).abs() <= FARTHEST)
    }

    /// A child the stepper can follow (`mutation`), or failing that, the
    /// cue itself.
    pub fn mutated(&self, other: Option<&Cue>, random: &mut impl Rng) -> Cue {
        (0..10).map(|_| self.mutation(other, random)).find(Cue::followable).unwrap_or_else(|| self.clone())
    }

    /// A child: a bump somewhere in the loop, or the loop stretched (slower
    /// or quicker), or its rocking made wider or narrower, or part of
    /// `other` crossed in, blended in and out over a few frames.
    fn mutation(&self, other: Option<&Cue>, random: &mut impl Rng) -> Cue {
        const BLEND: f32 = 4.0;
        let frames = self.frames.len();
        match random.gen_range(0..4) {
            1 => {
                let longer = random.gen_range(1..=3) * if random.gen_bool(0.5) { 1 } else { -1 };
                self.resampled((frames as i32 + longer).clamp(FRAMES.0 as i32, FRAMES.1 as i32) as usize)
            }
            2 if self.turns() == 0 => {
                let wider = random.gen_range(0.75..1.3);
                Cue { frames: self.frames.iter().map(|&value| moved(ZERO, step(ZERO, value) * wider)).collect() }
            }
            3 if other.is_some() => {
                let other = other.unwrap().resampled(frames);
                let (from, length) = (random.gen_range(0..frames), random.gen_range(frames / 4..=frames / 2));
                Cue {
                    frames: (0..frames)
                        .map(|i| {
                            let inside = ((i + frames - from) % frames) as f32;
                            let edge = inside.min(length as f32 - inside);
                            let mix = if inside < length as f32 { (edge / BLEND).min(1.0) } else { 0.0 };
                            moved(self.frames[i], step(self.frames[i], other.frames[i]) * mix)
                        })
                        .collect(),
                }
            }
            _ => {
                let (middle, width) = (random.gen_range(0.0..frames as f32), random.gen_range(2.0..(frames as f32 / 4.0).max(2.5)));
                let height = random.gen_range(-24.0..24.0);
                Cue {
                    frames: (0..frames)
                        .map(|i| {
                            let away = (i as f32 - middle).rem_euclid(frames as f32);
                            let away = away.min(frames as f32 - away);
                            moved(self.frames[i], height * (-away * away / (2.0 * width * width)).exp())
                        })
                        .collect(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cues_rock_and_turn() {
        let rocking = Cue::rocking(2.0, 30.0);
        assert_eq!(rocking.turns(), 0);
        assert_eq!(rocking.frames[0], ZERO);
        let turning = Cue::turning(2.0);
        assert!(turning.turns() > 0);
        assert!((1.0 / (turning.seconds() / turning.turns() as f32) - 2.0).abs() < 0.1);
        assert_eq!(rocking.describe(), "44f 128 149/6 107/11 149/11 107/11 128/5");
        assert!(turning.describe().contains('↻'), "{}", turning.describe());
        let between = turning.value(0.5 / DMX_RATE);
        assert!(between > turning.frames[0] as f32 && between < turning.frames[1] as f32);
    }
}
