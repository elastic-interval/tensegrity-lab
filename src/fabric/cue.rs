//! A cue for the wobble table's three steppers: a loop of DMX frames, each
//! three values, one for each stepper's drum, how far it has drawn its
//! cable loop along the deck's edge, 128 its middle. Played at `DMX_RATE`
//! frames a second, round and round, each drum going from one position to
//! the next. It is what the electronics play.
//!
//! A cue is made, and changed, in the deck's own three ways of moving: side
//! to side, fore and aft, and twisting about its middle, each turned into
//! what the drums do (`Cue::drums`). All three drums drawn the same way
//! twist the deck; drawn so that they add up to nothing, they sway it.
//!
//! Cues are evolved: from simple seeds, swaying, twisting and circling at a
//! spread of speeds, by mutation and crossing, and by sliding one way of
//! moving's timing against the others.

use crate::fabric::quake::drum_along;
use rand::Rng;
use std::f32::consts::TAU;

/// DMX frames a second.
pub const DMX_RATE: f32 = 44.0;
/// A drum's middle, as a DMX value.
pub const ZERO: u8 = 128;
/// How far a drum draws its cable for each DMX value: 20 mm either way.
pub const PER_VALUE: f32 = 0.02 / 127.0;
/// How fast a drum can draw its cable, metres a second: 10 mm round, four
/// turns a second.
pub const TOP_SPEED: f32 = 0.25;
/// The shortest and longest loops evolved, in frames.
const FRAMES: (usize, usize) = (16, 176);
/// How far a drum can go in a frame, in DMX values.
const FARTHEST: f32 = TOP_SPEED / DMX_RATE / PER_VALUE;
/// How far a seed moves the deck, in DMX values: 8 mm.
const AMPLITUDE: f32 = 50.0;
/// The deck's ways of moving, as weights of side, fore and twist: a twist
/// swings a piece more than a sway does, so a seed twists not as far.
pub const SIDE: [f32; 3] = [1.0, 0.0, 0.0];
pub const FORE: [f32; 3] = [0.0, 1.0, 0.0];
pub const TWIST: [f32; 3] = [0.0, 0.0, 0.6];
const STILL: [f32; 3] = [0.0; 3];
const WAYS: [&str; 3] = ["side", "fore", "twist"];

/// Side, fore and twist, in DMX values from the middle.
type Ways = [f32; 3];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cue {
    pub frames: Vec<[u8; 3]>,
}

fn scaled(ways: Ways, by: f32) -> Ways {
    ways.map(|way| way * by)
}

fn sum(a: Ways, b: Ways) -> Ways {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

impl Cue {
    /// What the drums do, from the middle, for the deck to move `ways`:
    /// each drum draws along the deck's edge, so sways by how much the sway
    /// goes its way, and twists by the twist.
    pub fn drums(ways: Ways) -> [f32; 3] {
        [0, 1, 2].map(|drum| {
            let along = drum_along(drum);
            ways[0] * along.x + ways[1] * along.z + ways[2]
        })
    }

    /// How the deck moves when the drums are at `drums`, from the middle.
    fn ways(drums: [f32; 3]) -> Ways {
        let mut ways = [0.0, 0.0, drums.iter().sum::<f32>() / 3.0];
        for (drum, &at) in drums.iter().enumerate() {
            let along = drum_along(drum);
            ways[0] += 2.0 / 3.0 * at * along.x;
            ways[1] += 2.0 / 3.0 * at * along.z;
        }
        ways
    }

    /// A cue from how the deck moves, frame by frame.
    fn from_ways(frames: impl IntoIterator<Item = Ways>) -> Cue {
        let value = |at: f32| (ZERO as f32 + at).round().clamp(0.0, 255.0) as u8;
        Cue { frames: frames.into_iter().map(|ways| Cue::drums(ways).map(value)).collect() }
    }

    /// How the deck moves, frame by frame.
    fn way_frames(&self) -> Vec<Ways> {
        self.frames.iter().map(|frame| Cue::ways(frame.map(|value| value as f32 - ZERO as f32))).collect()
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

    /// What the drums are told, tersely: the loop's length in frames, and
    /// how far the deck goes each way it goes, in millimetres, the twist at
    /// the drums: `34f side 8 twist 5`.
    pub fn describe(&self) -> String {
        let ways = self.way_frames();
        let mut words = vec![format!("{}f", self.frames.len())];
        for (way, name) in WAYS.iter().enumerate() {
            let (low, high) = ways.iter().fold((f32::MAX, f32::MIN), |(low, high), at| (low.min(at[way]), high.max(at[way])));
            let reach = (high - low) / 2.0 * PER_VALUE * 1000.0;
            if reach >= 1.0 {
                words.push(format!("{name} {reach:.0}"));
            }
        }
        words.join(" ")
    }

    /// Its DMX frames as written: each frame the three drums' values.
    pub fn written(&self) -> String {
        let frames: Vec<String> = self.frames.iter().map(|[a, b, c]| format!("{a}/{b}/{c}")).collect();
        frames.join(" ")
    }

    /// A cue from its DMX frames as written.
    pub fn parse(text: &str) -> Option<Cue> {
        let frame = |word: &str| -> Option<[u8; 3]> {
            let values: Vec<u8> = word.split('/').map(str::parse).collect::<Result<_, _>>().ok()?;
            values.try_into().ok()
        };
        let frames: Vec<[u8; 3]> = text.split_whitespace().map(frame).collect::<Option<_>>()?;
        (frames.len() >= FRAMES.0).then_some(Cue { frames })
    }

    /// Moving `a` and, a quarter cycle on, `b`, `hertz` times a second: with
    /// `b` still it rocks one way, side and fore together it circles.
    pub fn swaying(a: Ways, b: Ways, hertz: f32) -> Cue {
        let (frames, cycles) = Self::span(hertz);
        Cue::from_ways((0..frames).map(|i| {
            let phase = TAU * cycles as f32 * i as f32 / frames as f32;
            sum(scaled(a, AMPLITUDE * phase.sin()), scaled(b, AMPLITUDE * phase.cos()))
        }))
    }

    /// Rocking `way`, `hertz` times a second.
    pub fn rocking(way: Ways, hertz: f32) -> Cue {
        Cue::swaying(way, STILL, hertz)
    }

    /// Rocking `way` for `cycles` cycles at `hertz`, then holding still for
    /// `pause` seconds, so the piece rings on by itself.
    pub fn pulsing(way: Ways, hertz: f32, cycles: usize, pause: f32) -> Cue {
        let driving = (cycles as f32 * DMX_RATE / hertz).round() as usize;
        let holding = (pause * DMX_RATE).round() as usize;
        Cue::from_ways((0..driving + holding).map(|i| {
            let phase = (TAU * cycles as f32 * i.min(driving) as f32 / driving as f32).sin();
            scaled(way, if i < driving { AMPLITUDE * phase } else { 0.0 })
        }))
    }

    /// A loop of whole cycles at `hertz`, at least 32 frames long.
    fn span(hertz: f32) -> (usize, usize) {
        let cycles = (32.0 * hertz / DMX_RATE).ceil().max(1.0) as usize;
        ((cycles as f32 * DMX_RATE / hertz).round() as usize, cycles)
    }

    /// Where evolving starts: rocking side to side, fore and aft, along a
    /// diagonal and twisting, at a spread of speeds; circling; swaying and
    /// twisting by turns; and a few cycles of rocking, then holding still
    /// while the piece rings on.
    pub fn seeds() -> Vec<Cue> {
        const DIAGONAL: Ways = [0.7, 0.7, 0.0];
        let side = [0.6, 0.8, 1.0, 1.3, 1.6, 2.0, 2.3, 2.6, 2.9].map(|hertz| Cue::rocking(SIDE, hertz));
        let fore = [0.8, 1.3, 2.0, 2.6].map(|hertz| Cue::rocking(FORE, hertz));
        let diagonal = [1.0, 1.6, 2.3].map(|hertz| Cue::rocking(DIAGONAL, hertz));
        let twist = [0.8, 1.3, 1.6, 2.0, 2.6, 3.2].map(|hertz| Cue::rocking(TWIST, hertz));
        let circling = [1.3, 2.1, 2.6].map(|hertz| Cue::swaying(SIDE, FORE, hertz));
        let wagging = [1.3, 2.0].map(|hertz| Cue::swaying(SIDE, TWIST, hertz));
        let pulses = [(SIDE, 1.3, 2), (SIDE, 2.0, 3), (FORE, 1.3, 2), (TWIST, 2.0, 3)]
            .map(|(way, hertz, cycles)| Cue::pulsing(way, hertz, cycles, 2.0));
        [&side[..], &fore, &diagonal, &twist, &circling, &wagging, &pulses].concat()
    }

    /// This loop's ways resampled to `frames` frames.
    fn resampled(&self, frames: usize) -> Vec<Ways> {
        let seconds = self.seconds();
        (0..frames)
            .map(|i| Cue::ways(self.value(seconds * i as f32 / frames as f32).map(|value| value - ZERO as f32)))
            .collect()
    }

    /// Whether the drums can follow it, from each frame to the next.
    pub fn followable(&self) -> bool {
        let frames = self.frames.len();
        (0..frames).all(|i| {
            let (from, to) = (self.frames[i], self.frames[(i + 1) % frames]);
            (0..3).all(|drum| (to[drum] as f32 - from[drum] as f32).abs() <= FARTHEST)
        })
    }

    /// A child the drums can follow (`mutation`), or failing that, the cue
    /// itself.
    pub fn mutated(&self, other: Option<&Cue>, random: &mut impl Rng) -> Cue {
        (0..10).map(|_| self.mutation(other, random)).find(Cue::followable).unwrap_or_else(|| self.clone())
    }

    /// A child, in the deck's ways of moving: a bump in one of them
    /// somewhere in the loop; the loop stretched, slower or quicker; one
    /// way, or all, made wider or narrower; one way's timing slid against
    /// the others'; the sway turned to another direction; or from `other`,
    /// a part of its loop blended in and out over a few frames, or one of
    /// its ways in place of this one's.
    fn mutation(&self, other: Option<&Cue>, random: &mut impl Rng) -> Cue {
        const BLEND: f32 = 4.0;
        let mut ways = self.way_frames();
        let frames = ways.len();
        let way = random.gen_range(0..3);
        match random.gen_range(0..6) {
            1 => {
                let longer = random.gen_range(1..=3) * if random.gen_bool(0.5) { 1 } else { -1 };
                ways = self.resampled((frames as i32 + longer).clamp(FRAMES.0 as i32, FRAMES.1 as i32) as usize);
            }
            2 => {
                let wider = random.gen_range(0.75..1.3);
                let all = random.gen_bool(0.3);
                for each in 0..3 {
                    if all || each == way {
                        let centre = ways.iter().map(|at| at[each]).sum::<f32>() / frames as f32;
                        ways.iter_mut().for_each(|at| at[each] = centre + (at[each] - centre) * wider);
                    }
                }
            }
            3 => {
                let by = random.gen_range(1..=frames / 2);
                let slid: Vec<f32> = (0..frames).map(|i| ways[(i + by) % frames][way]).collect();
                ways.iter_mut().zip(slid).for_each(|(at, slid)| at[way] = slid);
            }
            4 => {
                let (sin, cos) = random.gen_range(-TAU / 8.0..TAU / 8.0).sin_cos();
                ways.iter_mut().for_each(|at| (at[0], at[1]) = (at[0] * cos - at[1] * sin, at[0] * sin + at[1] * cos));
            }
            5 if other.is_some() => {
                let other = other.unwrap().resampled(frames);
                if random.gen_bool(0.5) {
                    ways.iter_mut().zip(&other).for_each(|(at, theirs)| at[way] = theirs[way]);
                } else {
                    let (from, length) = (random.gen_range(0..frames), random.gen_range(frames / 4..=frames / 2));
                    for (i, at) in ways.iter_mut().enumerate() {
                        let inside = ((i + frames - from) % frames) as f32;
                        let edge = inside.min(length as f32 - inside);
                        let mix = if inside < length as f32 { (edge / BLEND).min(1.0) } else { 0.0 };
                        *at = sum(scaled(*at, 1.0 - mix), scaled(other[i], mix));
                    }
                }
            }
            _ => {
                let (middle, width) = (random.gen_range(0.0..frames as f32), random.gen_range(2.0..(frames as f32 / 4.0).max(2.5)));
                let height = random.gen_range(-24.0..24.0);
                for (i, at) in ways.iter_mut().enumerate() {
                    let away = (i as f32 - middle).rem_euclid(frames as f32);
                    let away = away.min(frames as f32 - away);
                    at[way] += height * (-away * away / (2.0 * width * width)).exp();
                }
            }
        }
        Cue::from_ways(ways)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cues_sway_twist_and_circle() {
        let side = Cue::rocking(SIDE, 2.0);
        assert_eq!(side.frames[0], [ZERO; 3]);
        assert_eq!(side.describe(), "44f side 8");
        let twist = Cue::rocking(TWIST, 2.0);
        let quarter = twist.frames[5];
        assert!(quarter[0] == quarter[1] && quarter[1] == quarter[2] && quarter[0] > ZERO, "twisting, all drums go alike");
        assert_eq!(twist.describe(), "44f twist 5");
        assert_eq!(Cue::swaying(SIDE, FORE, 2.0).describe(), "44f side 8 fore 8");
        let ways = Cue::ways(Cue::drums([10.0, -20.0, 5.0]));
        assert!(ways.iter().zip([10.0, -20.0, 5.0]).all(|(a, b)| (a - b).abs() < 1e-4), "{ways:?}");
        assert!(Cue::seeds().iter().all(|seed| seed.followable() && seed.frames.len() <= FRAMES.1));
        let pulse = Cue::pulsing(FORE, 2.0, 3, 2.0);
        assert_eq!(pulse.frames[pulse.frames.len() - 1], [ZERO; 3], "it holds still in the middle");
        assert_eq!(Cue::parse(&side.written()), Some(side.clone()));
        let between = side.value(0.5 / DMX_RATE);
        let (first, second) = (side.frames[0], side.frames[1]);
        assert!((0..3).all(|drum| (between[drum] - (first[drum] as f32 + second[drum] as f32) / 2.0).abs() < 1e-4));
    }
}
