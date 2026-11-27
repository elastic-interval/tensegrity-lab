//! A cue for the wobble table's three motors: a smooth swing, the same loop
//! over and over. It is made in the deck's own three ways of moving, side to
//! side, fore and aft, and twisting about its middle: one period for the
//! loop, and for each way how far and when in the swing. All three motors
//! winding the same way twist the deck; winding so that they add up to
//! nothing, they sway it (`Cue::drums`). Only smooth swings: the deck on its
//! ties and its elastic ring is the resonator, and a motor's quick moves are
//! swallowed by the ring anyway. A cue may also swing a few times and then
//! hold still, so the piece rings on by itself.
//!
//! What the electronics play is made from those numbers: a loop of DMX
//! frames, `DMX_RATE` a second, each three values, one for each motor's
//! drum, how far it has wound the ring, 128 its middle.
//!
//! Cues are evolved: from seeds rocking each way, circling and pulsing at a
//! spread of speeds, by changing the period, a way's reach or timing, the
//! direction of the sway, the pulsing, and by taking a way from another cue.

use crate::fabric::quake::drum_along;
use rand::Rng;
use std::f32::consts::TAU;

/// DMX frames a second.
pub const DMX_RATE: f32 = 44.0;
/// A drum's middle, as a DMX value.
pub const ZERO: u8 = 128;
/// How far a drum winds the ring for each DMX value: 20 mm either way.
pub const PER_VALUE: f32 = 0.02 / 127.0;
/// How fast a drum can wind, metres a second: 10 mm round, four turns a
/// second.
pub const TOP_SPEED: f32 = 0.25;
/// The shortest and longest swings evolved, seconds.
const PERIODS: (f32, f32) = (0.36, 4.0);
/// How far a seed moves the deck, in DMX values: 8 mm.
const AMPLITUDE: f32 = 50.0;
/// The deck's ways of moving, as weights of side, fore and twist: a twist
/// swings a piece more than a sway does, so a seed twists not as far.
pub const SIDE: [f32; 3] = [1.0, 0.0, 0.0];
pub const FORE: [f32; 3] = [0.0, 1.0, 0.0];
pub const TWIST: [f32; 3] = [0.0, 0.0, 0.6];
const WAYS: [&str; 3] = ["side", "fore", "twist"];

/// Side, fore and twist.
type Ways = [f32; 3];

#[derive(Clone, Debug, PartialEq)]
pub struct Cue {
    /// How long one swing takes, seconds, a whole number of frames.
    pub period: f32,
    /// For side, fore and twist: how far the deck goes each way, in DMX
    /// values from the middle, and when, as a part of the swing.
    pub reach: Ways,
    pub timing: Ways,
    /// Swung this many times and then held still this many seconds, the
    /// piece ringing on; or swung on and on.
    pub pulse: Option<(usize, f32)>,
    /// What the electronics play: each frame the three drums' values.
    pub frames: Vec<[u8; 3]>,
}

impl Cue {
    /// A cue swinging every `period` seconds, each way as far as `reach`,
    /// at `timing`, perhaps pulsing.
    pub fn new(period: f32, reach: Ways, timing: Ways, pulse: Option<(usize, f32)>) -> Cue {
        let swing = (period.clamp(PERIODS.0, PERIODS.1) * DMX_RATE).round().max(1.0) as usize;
        let period = swing as f32 / DMX_RATE;
        // Kept to the steps it is written in, so it reads back the same.
        let reach = reach.map(|each| (each * 1000.0).round() / 1000.0);
        let timing = timing.map(|at| (at.rem_euclid(1.0) * 10000.0).round() / 10000.0);
        let (driving, holding) = match pulse {
            Some((cycles, hold)) => (cycles.max(1) * swing, (hold.max(0.0) * DMX_RATE).round() as usize),
            None => (swing, 0),
        };
        let value = |at: f32| (ZERO as f32 + at).round().clamp(0.0, 255.0) as u8;
        let frames = (0..driving + holding)
            .map(|i| {
                let ways: Ways = if i < driving {
                    let along = i as f32 / swing as f32;
                    [0, 1, 2].map(|way| reach[way] * (TAU * (along + timing[way])).sin())
                } else {
                    [0.0; 3]
                };
                Cue::drums(ways).map(value)
            })
            .collect();
        Cue { period, reach, timing, pulse, frames }
    }

    /// What the drums do, from the middle, for the deck to move `ways`: each
    /// drum pushes the deck along its edge, so sways it by how much the
    /// sway goes its way, and twists it by the twist.
    pub fn drums(ways: Ways) -> [f32; 3] {
        [0, 1, 2].map(|drum| {
            let along = drum_along(drum);
            ways[0] * along.x + ways[1] * along.z + ways[2]
        })
    }

    /// How long the loop lasts.
    pub fn seconds(&self) -> f32 {
        self.frames.len() as f32 / DMX_RATE
    }

    /// Each drum's DMX value at `seconds` into the loop, between frames.
    pub fn value(&self, seconds: f32) -> [f32; 3] {
        let at = (seconds * DMX_RATE).rem_euclid(self.frames.len() as f32);
        let index = at.floor() as usize % self.frames.len();
        let (from, to) = (self.frames[index], self.frames[(index + 1) % self.frames.len()]);
        [0, 1, 2].map(|drum| from[drum] as f32 + (to[drum] as f32 - from[drum] as f32) * at.fract())
    }

    /// What it is, tersely, for people: the period, how far it goes each
    /// way, millimetres either way at the drums, a way out of step with the
    /// first by how much of a swing, and its pulsing: `1.25s side 8 twist 5
    /// +0.25 x3+2s`.
    pub fn describe(&self) -> String {
        let mut words = vec![format!("{:.2}s", self.period)];
        let first = (0..3).find(|&way| self.reach[way].abs() * PER_VALUE >= 0.0005);
        for (way, name) in WAYS.iter().enumerate() {
            let millimetres = self.reach[way] * PER_VALUE * 1000.0;
            if millimetres.abs() < 0.5 {
                continue;
            }
            words.push(format!("{name} {millimetres:.0}"));
            let late = (self.timing[way] - first.map_or(0.0, |first| self.timing[first])).rem_euclid(1.0);
            if late > 0.02 && late < 0.98 {
                words.push(format!("+{late:.2}"));
            }
        }
        if let Some((cycles, hold)) = self.pulse {
            words.push(format!("x{cycles}+{hold:.0}s"));
        }
        words.join(" ")
    }

    /// What it is, in its own numbers, as written and read back (`parse`):
    /// the period, each way it goes, millimetres either way at the drums
    /// and when, and its pulsing: `1.250s side 7.8740@0.0000 x3+2.0s`.
    pub fn numbers(&self) -> String {
        let mut words = vec![format!("{:.3}s", self.period)];
        for (way, name) in WAYS.iter().enumerate() {
            if self.reach[way].abs() > 0.0 {
                words.push(format!("{name} {:.4}@{:.4}", self.reach[way] * PER_VALUE * 1000.0, self.timing[way]));
            }
        }
        if let Some((cycles, hold)) = self.pulse {
            words.push(format!("x{cycles}+{hold:.1}s"));
        }
        words.join(" ")
    }

    /// A cue from what `numbers` wrote.
    pub fn parse(text: &str) -> Option<Cue> {
        let mut words = text.split_whitespace();
        let period: f32 = words.next()?.strip_suffix('s')?.parse().ok()?;
        let (mut reach, mut timing, mut pulse) = ([0.0; 3], [0.0; 3], None);
        while let Some(word) = words.next() {
            if let Some(way) = WAYS.iter().position(|name| *name == word) {
                let (millimetres, at) = words.next()?.split_once('@')?;
                reach[way] = millimetres.parse::<f32>().ok()? / 1000.0 / PER_VALUE;
                timing[way] = at.parse().ok()?;
            } else {
                let (cycles, hold) = word.strip_prefix('x')?.split_once('+')?;
                pulse = Some((cycles.parse().ok()?, hold.strip_suffix('s')?.parse().ok()?));
            }
        }
        Some(Cue::new(period, reach, timing, pulse))
    }

    /// Its DMX frames as written: each frame the three drums' values.
    pub fn written(&self) -> String {
        let frames: Vec<String> = self.frames.iter().map(|[a, b, c]| format!("{a}/{b}/{c}")).collect();
        frames.join(" ")
    }

    /// Moving `a` and, a quarter swing on, `b`, `hertz` times a second: with
    /// `b` still it rocks one way, side and fore together it circles.
    pub fn swaying(a: Ways, b: Ways, hertz: f32) -> Cue {
        let mut reach = [0.0; 3];
        let mut timing = [0.0; 3];
        for way in 0..3 {
            if a[way] != 0.0 {
                (reach[way], timing[way]) = (a[way] * AMPLITUDE, 0.0);
            } else if b[way] != 0.0 {
                (reach[way], timing[way]) = (b[way] * AMPLITUDE, 0.25);
            }
        }
        Cue::new(1.0 / hertz, reach, timing, None)
    }

    /// Rocking `way`, `hertz` times a second.
    pub fn rocking(way: Ways, hertz: f32) -> Cue {
        Cue::swaying(way, [0.0; 3], hertz)
    }

    /// Rocking `way` for `cycles` swings at `hertz`, then holding still for
    /// `pause` seconds, so the piece rings on by itself.
    pub fn pulsing(way: Ways, hertz: f32, cycles: usize, pause: f32) -> Cue {
        let rocking = Cue::rocking(way, hertz);
        Cue::new(rocking.period, rocking.reach, rocking.timing, Some((cycles, pause)))
    }

    /// Where evolving starts: rocking side to side, fore and aft, along a
    /// diagonal and twisting, at a spread of speeds; circling; swaying and
    /// twisting by turns; and a few swings of rocking, then holding still
    /// while the piece rings on.
    pub fn seeds() -> Vec<Cue> {
        const DIAGONAL: Ways = [0.7, 0.7, 0.0];
        let side = [0.6, 0.8, 1.0, 1.3, 1.6, 2.0, 2.3, 2.6].map(|hertz| Cue::rocking(SIDE, hertz));
        let fore = [0.8, 1.3, 2.0, 2.6].map(|hertz| Cue::rocking(FORE, hertz));
        let diagonal = [1.0, 1.6, 2.3].map(|hertz| Cue::rocking(DIAGONAL, hertz));
        let twist = [0.8, 1.3, 1.6, 2.0, 2.6].map(|hertz| Cue::rocking(TWIST, hertz));
        let circling = [1.3, 2.1, 2.6].map(|hertz| Cue::swaying(SIDE, FORE, hertz));
        let wagging = [1.3, 2.0].map(|hertz| Cue::swaying(SIDE, TWIST, hertz));
        let pulses = [(SIDE, 1.3, 2), (SIDE, 2.0, 3), (FORE, 1.3, 2), (TWIST, 2.0, 3)]
            .map(|(way, hertz, cycles)| Cue::pulsing(way, hertz, cycles, 2.0));
        [&side[..], &fore, &diagonal, &twist, &circling, &wagging, &pulses].concat()
    }

    /// Whether the drums can follow it: none must wind faster than
    /// `TOP_SPEED`.
    pub fn followable(&self) -> bool {
        let fastest = TOP_SPEED / DMX_RATE / PER_VALUE;
        let frames = self.frames.len();
        let mut steps = (0..frames).map(|i| (self.frames[i], self.frames[(i + 1) % frames]));
        steps.all(|(from, to)| (0..3).all(|drum| (to[drum] as f32 - from[drum] as f32).abs() <= fastest))
    }

    /// A child the drums can follow (`mutation`), or failing that, the cue
    /// itself.
    pub fn mutated(&self, other: Option<&Cue>, random: &mut impl Rng) -> Cue {
        (0..10).map(|_| self.mutation(other, random)).find(Cue::followable).unwrap_or_else(|| self.clone())
    }

    /// A child: its swing slower or quicker; one way, or all, reaching
    /// farther or less far, or begun from nothing; one way's timing slid
    /// against the others'; the sway turned to another direction; its
    /// pulsing changed, begun or ended; or from `other`, one of its ways,
    /// or its period.
    fn mutation(&self, other: Option<&Cue>, random: &mut impl Rng) -> Cue {
        let (mut period, mut reach, mut timing, mut pulse) = (self.period, self.reach, self.timing, self.pulse);
        let way = random.gen_range(0..3);
        match random.gen_range(0..6) {
            0 => period *= random.gen_range(0.85..1.18),
            1 => {
                let wider = random.gen_range(0.75..1.3);
                if random.gen_bool(0.3) {
                    reach = reach.map(|each| each * wider);
                } else if reach[way] == 0.0 {
                    reach[way] = random.gen_range(-AMPLITUDE..AMPLITUDE) / 2.0;
                } else {
                    reach[way] *= wider;
                }
            }
            2 => timing[way] += random.gen_range(-0.25..0.25),
            3 => {
                let turn = random.gen_range(-TAU / 8.0..TAU / 8.0);
                let at = |way: usize| (reach[way] * (TAU * timing[way]).cos(), reach[way] * (TAU * timing[way]).sin());
                let ((side_c, side_s), (fore_c, fore_s)) = (at(0), at(1));
                let (sin, cos) = turn.sin_cos();
                let turned = [(side_c * cos - fore_c * sin, side_s * cos - fore_s * sin), (side_c * sin + fore_c * cos, side_s * sin + fore_s * cos)];
                for (way, (c, s)) in turned.into_iter().enumerate() {
                    reach[way] = c.hypot(s);
                    timing[way] = s.atan2(c) / TAU;
                }
            }
            4 => {
                pulse = match pulse {
                    None => Some((random.gen_range(2..=4), random.gen_range(1.0..3.0))),
                    Some(_) if random.gen_bool(0.3) => None,
                    Some((cycles, hold)) => Some(((cycles as i32 + random.gen_range(-1..=1)).max(1) as usize, (hold * random.gen_range(0.75..1.3)).min(4.0))),
                };
            }
            _ => match other {
                Some(other) if random.gen_bool(0.7) => (reach[way], timing[way]) = (other.reach[way], other.timing[way]),
                Some(other) => period = other.period,
                None => timing[way] += random.gen_range(-0.25..0.25),
            },
        }
        Cue::new(period, reach, timing, pulse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cues_sway_twist_and_circle() {
        let side = Cue::rocking(SIDE, 2.0);
        assert_eq!(side.frames[0], [ZERO; 3]);
        assert_eq!(side.frames.len(), 22);
        let twist = Cue::rocking(TWIST, 2.0);
        let quarter = twist.frames[5];
        assert!(quarter[0] == quarter[1] && quarter[1] == quarter[2] && quarter[0] > ZERO, "twisting, all drums go alike");
        assert!(Cue::seeds().iter().all(Cue::followable));
        let pulse = Cue::pulsing(FORE, 2.0, 3, 2.0);
        assert_eq!(pulse.frames.len(), 3 * 22 + 88);
        assert_eq!(pulse.frames[pulse.frames.len() - 1], [ZERO; 3], "it holds still in the middle");
        let circling = Cue::swaying(SIDE, FORE, 1.3);
        let read = Cue::parse(&circling.numbers()).unwrap();
        assert_eq!(read.frames, circling.frames, "{} read back as {}", circling.numbers(), read.numbers());
        assert_eq!(Cue::parse(&pulse.numbers()).unwrap().frames, pulse.frames);
        assert_eq!(circling.describe(), "0.77s side 8 fore 8 +0.25");
        assert_eq!(pulse.describe(), "0.50s fore 8 x3+2s");
        let between = side.value(0.5 / DMX_RATE);
        let (first, second) = (side.frames[0], side.frames[1]);
        assert!((0..3).all(|drum| (between[drum] - (first[drum] as f32 + second[drum] as f32) / 2.0).abs() < 1e-4));
    }
}
