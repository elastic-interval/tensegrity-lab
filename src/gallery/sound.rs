//! The gallery's music: each piece its own groove, played by its own
//! movement, as its sensors feel it (`sensor.rs`), not by the table that
//! sets it going. Each time the first sensor's joint turns in its swing is
//! a downbeat, a kick and, every so many, the next note of the piece's own
//! arpeggio, over chords changing slowly, all in one key; the second's, a
//! snare; the third's, a hi-hat. Each piece is heard by its own platform,
//! from where it stands, fading quickly as the visitor walks away.
//! Synthesised as it plays, on its own thread, each beat a moment after it
//! is felt, so that all keep their time.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::f32::consts::TAU;
use std::sync::{Arc, Mutex};

/// How a piece sounds. Its tone: the octave of its lowest note (MIDI, as a
/// multiple of 12), a frequency-modulation tone (the modulator's ratio to
/// the note, and how bright it starts), how it comes in and dies away,
/// seconds, played every so many downbeats, stepping through its chord
/// tones in `pattern` (past the fourth, an octave up), and how loud. Its
/// drums: how loud the kick, the snare and the hi-hat, none at zero, and
/// how deep the kick, hertz.
struct Instrument {
    octave: u8,
    ratio: f32,
    brightness: f32,
    attack: f32,
    decay: f32,
    every: usize,
    pattern: &'static [usize],
    loudness: f32,
    kick: f32,
    kick_pitch: f32,
    snare: f32,
    hat: f32,
}

/// The gallery's pieces in turn: Open Claw electric-piano stabs over kick
/// and snare; Headless Hug the deep one, a heavy kick and a short sub bass
/// on every beat; Quadruped crisp hats, a light kick and a sparse bell;
/// Halo by Crane a quick marimba running up and down over a snare, no kick;
/// Noodle a slow, low pluck over a soft kick and hats.
const INSTRUMENTS: [Instrument; crate::gallery::PIECES] = [
    Instrument {
        octave: 4,
        ratio: 1.0,
        brightness: 1.2,
        attack: 0.005,
        decay: 0.35,
        every: 2,
        pattern: &[0, 2, 1, 3],
        loudness: 0.35,
        kick: 0.7,
        kick_pitch: 52.0,
        snare: 0.45,
        hat: 0.2,
    },
    Instrument {
        octave: 2,
        ratio: 1.0,
        brightness: 0.5,
        attack: 0.005,
        decay: 0.45,
        every: 1,
        pattern: &[0, 0, 2, 0, 1, 0],
        loudness: 0.7,
        kick: 1.0,
        kick_pitch: 42.0,
        snare: 0.0,
        hat: 0.15,
    },
    Instrument {
        octave: 6,
        ratio: 3.5,
        brightness: 1.2,
        attack: 0.002,
        decay: 0.6,
        every: 4,
        pattern: &[0, 2, 1, 3],
        loudness: 0.15,
        kick: 0.5,
        kick_pitch: 60.0,
        snare: 0.3,
        hat: 0.35,
    },
    Instrument {
        octave: 5,
        ratio: 4.0,
        brightness: 0.8,
        attack: 0.002,
        decay: 0.25,
        every: 1,
        pattern: &[0, 1, 2, 3, 2, 1],
        loudness: 0.25,
        kick: 0.0,
        kick_pitch: 50.0,
        snare: 0.35,
        hat: 0.1,
    },
    Instrument {
        octave: 3,
        ratio: 2.0,
        brightness: 0.9,
        attack: 0.01,
        decay: 0.8,
        every: 2,
        pattern: &[0, 3, 2, 1],
        loudness: 0.35,
        kick: 0.6,
        kick_pitch: 48.0,
        snare: 0.0,
        hat: 0.25,
    },
];

/// The chords, as the pitch classes of their tones (A is 9), and how long
/// each lasts: A minor add 9, F major 7, C add 9, G6.
const CHORDS: [[u8; 4]; 4] = [[9, 0, 4, 11], [5, 9, 0, 4], [0, 4, 7, 2], [7, 11, 2, 4]];
const CHORD_SECONDS: f32 = 24.0;
/// A piece is heard loudest by its platform: at this distance from its
/// middle, half as loud as beside it, a fifth at twice as far.
const NEAR: f32 = 3.0;
/// How much of a piece goes into the reverberation beside it; the room's
/// reverberation falls off more slowly, so far pieces are heard mostly in it.
const WET: f32 = 0.3;
/// The whole, before it is softly limited.
const MASTER: f32 = 0.35;
/// How quickly gains follow what they are told, seconds.
const GLIDE: f32 = 0.05;
/// How long after it is felt a beat sounds, seconds: long enough for every
/// beat felt between two frames to sound in its own time.
const LATENCY: f32 = 0.06;
/// How the piece's time is kept to the music's: followed this much at a
/// time, and taken afresh when it is off by more than `RESYNC` seconds.
const FOLLOW: f32 = 0.05;
const RESYNC: f32 = 0.25;

#[derive(Clone, Copy)]
enum Kind {
    Tone,
    Kick,
    Snare,
    Hat,
}

/// A sound sounding: what it is, how long since struck, its frequency, how
/// loud, and its phases (a tone's carrier and modulator, a drum's body and
/// its filtered noise).
struct Note {
    kind: Kind,
    age: f32,
    frequency: f32,
    loudness: f32,
    carrier: f32,
    modulator: f32,
}

/// A piece as the music knows it.
struct Voice {
    instrument: &'static Instrument,
    /// The music's time less the piece's, and the beats felt, still to
    /// sound: when, in the music's time, which sensor, and how hard.
    offset: Option<f32>,
    pending: Vec<(f32, usize, f32)>,
    downbeats: usize,
    step: usize,
    /// To the left and right, and into the reverberation: as told, and as
    /// heard now, gliding there.
    told: [f32; 3],
    gains: [f32; 3],
    notes: Vec<Note>,
    noise: u32,
}

/// What the main loop and the sound's own thread share.
struct Shared {
    voices: Vec<Voice>,
    seconds: f32,
    reverb: Reverb,
}

pub struct Sound {
    shared: Arc<Mutex<Shared>>,
    _stream: cpal::Stream,
}

impl Sound {
    /// The music, playing through the default output, silent until told
    /// where the pieces are; none if there is no output to play on.
    pub fn new() -> Option<Sound> {
        let device = cpal::default_host().default_output_device()?;
        let config = device.default_output_config().ok()?;
        if config.sample_format() != cpal::SampleFormat::F32 {
            eprintln!("No sound: the output doesn't take floating point samples");
            return None;
        }
        let rate = config.sample_rate().0 as f32;
        let channels = config.channels() as usize;
        let voices = INSTRUMENTS
            .iter()
            .enumerate()
            .map(|(index, instrument)| Voice {
                instrument,
                offset: None,
                pending: Vec::new(),
                downbeats: 0,
                step: 0,
                told: [0.0; 3],
                gains: [0.0; 3],
                notes: Vec::new(),
                noise: 0x9E37_79B9 ^ (index as u32 + 1),
            })
            .collect();
        let shared = Arc::new(Mutex::new(Shared { voices, seconds: 0.0, reverb: Reverb::new(rate) }));
        let playing = Arc::clone(&shared);
        let stream = device
            .build_output_stream(
                &config.into(),
                move |data: &mut [f32], _| playing.lock().unwrap().fill(data, channels, rate),
                |error| eprintln!("Sound: {error}"),
                None,
            )
            .ok()?;
        stream.play().ok()?;
        Some(Sound { shared, _stream: stream })
    }

    /// The piece at `station`, its own time `now`, seconds: the turns its
    /// sensors `felt` since last told (which sensor, when, how hard); how
    /// far away it is, and how far to the right of where the visitor looks,
    /// from -1 to 1.
    pub fn tell(&self, station: usize, now: f32, felt: &[(usize, f32, f32)], distance: f32, right: f32) {
        let mut shared = self.shared.lock().unwrap();
        let offset = shared.seconds - now;
        let voice = &mut shared.voices[station];
        let offset = match voice.offset {
            Some(kept) if (kept - offset).abs() < RESYNC => kept + (offset - kept) * FOLLOW,
            _ => offset,
        };
        voice.offset = Some(offset);
        voice.pending.extend(felt.iter().map(|&(sensor, at, hard)| (at + offset + LATENCY, sensor, hard)));
        let near = 1.0 / (1.0 + (distance / NEAR).powi(2));
        let roomy = 1.0 / (1.0 + distance / NEAR);
        let angle = (right.clamp(-1.0, 1.0) + 1.0) * TAU / 8.0;
        voice.told = [near * angle.cos(), near * angle.sin(), WET * roomy];
    }
}

impl Shared {
    /// The next samples, `channels` to a frame, the first two left and
    /// right.
    fn fill(&mut self, data: &mut [f32], channels: usize, rate: f32) {
        let dt = 1.0 / rate;
        let glide = dt / GLIDE;
        for frame in data.chunks_mut(channels) {
            self.seconds += dt;
            let chord = CHORDS[(self.seconds / CHORD_SECONDS) as usize % CHORDS.len()];
            let (mut left, mut right, mut wet) = (0.0, 0.0, 0.0);
            for voice in &mut self.voices {
                for (gain, told) in voice.gains.iter_mut().zip(voice.told) {
                    *gain += (told - *gain) * glide;
                }
                voice.strike(self.seconds, chord);
                let sound = voice.sound(dt);
                left += sound * voice.gains[0];
                right += sound * voice.gains[1];
                wet += sound * voice.gains[2];
            }
            let (reverb_left, reverb_right) = self.reverb.process(wet);
            let out = [(left + reverb_left) * MASTER, (right + reverb_right) * MASTER].map(f32::tanh);
            for (channel, sample) in frame.iter_mut().enumerate() {
                *sample = out[channel.min(1)];
            }
        }
    }
}

impl Voice {
    /// Striking each beat due by `now`: the first sensor's a kick and,
    /// every so many, the next note of its pattern in `chord`; the
    /// second's a snare; the third's a hi-hat.
    fn strike(&mut self, now: f32, chord: [u8; 4]) {
        if self.pending.iter().all(|&(at, ..)| at > now) {
            return;
        }
        let due: Vec<(usize, f32)> = self.pending.iter().filter(|&&(at, ..)| at <= now).map(|&(_, sensor, hard)| (sensor, hard)).collect();
        self.pending.retain(|&(at, ..)| at > now);
        let instrument = self.instrument;
        for (sensor, hard) in due {
            let mut strike = |kind: Kind, frequency: f32, loudness: f32| {
                if loudness > 0.0 {
                    self.notes.push(Note { kind, age: 0.0, frequency, loudness: loudness * hard, carrier: 0.0, modulator: 0.0 });
                }
            };
            match sensor {
                0 => {
                    strike(Kind::Kick, instrument.kick_pitch, instrument.kick);
                    self.downbeats += 1;
                    if self.downbeats % instrument.every == 0 {
                        let degree = instrument.pattern[self.step % instrument.pattern.len()];
                        self.step += 1;
                        let mut tones = chord;
                        tones.sort();
                        let midi = instrument.octave * 12 + tones[degree % 4] + 12 * (degree / 4) as u8;
                        let frequency = 440.0 * 2f32.powf((midi as f32 - 69.0) / 12.0);
                        self.notes.push(Note { kind: Kind::Tone, age: 0.0, frequency, loudness: instrument.loudness * hard, carrier: 0.0, modulator: 0.0 });
                    }
                }
                1 => strike(Kind::Snare, 190.0, instrument.snare),
                _ => strike(Kind::Hat, 0.0, instrument.hat),
            }
        }
    }

    /// White noise, from -1 to 1.
    fn noise(&mut self) -> f32 {
        self.noise ^= self.noise << 13;
        self.noise ^= self.noise >> 17;
        self.noise ^= self.noise << 5;
        self.noise as f32 / u32::MAX as f32 * 2.0 - 1.0
    }

    /// Its sounds sounding, `dt` on, the dead ones let go.
    fn sound(&mut self, dt: f32) -> f32 {
        let Instrument { ratio, brightness, attack, decay, .. } = *self.instrument;
        let noise = self.noise();
        let mut sum = 0.0;
        for note in &mut self.notes {
            note.age += dt;
            let age = note.age;
            sum += note.loudness
                * match note.kind {
                    Kind::Tone => {
                        let envelope = (age / attack).min(1.0) * (-age / decay).exp();
                        let index = brightness * (-age / (0.3 * decay)).exp();
                        let out = envelope * (note.carrier + index * note.modulator.sin()).sin();
                        note.carrier = (note.carrier + TAU * note.frequency * dt) % TAU;
                        note.modulator = (note.modulator + TAU * note.frequency * ratio * dt) % TAU;
                        out
                    }
                    // A sine falling quickly in pitch, and a click.
                    Kind::Kick => {
                        let pitch = note.frequency * (1.0 + 2.5 * (-age / 0.04).exp());
                        note.carrier = (note.carrier + TAU * pitch * dt) % TAU;
                        note.carrier.sin() * (-age / 0.3).exp() + 0.2 * noise * (-age / 0.003).exp()
                    }
                    // A short tone under a burst of brightened noise.
                    Kind::Snare => {
                        note.carrier = (note.carrier + TAU * note.frequency * dt) % TAU;
                        let bright = noise - note.modulator;
                        note.modulator = noise;
                        0.5 * note.carrier.sin() * (-age / 0.06).exp() + 0.5 * bright * (-age / 0.14).exp()
                    }
                    // Noise with its lows taken out, very short.
                    Kind::Hat => {
                        note.carrier += (noise - note.carrier) * 0.3;
                        (noise - note.carrier) * (-age / 0.035).exp()
                    }
                };
        }
        self.notes.retain(|note| note.age < 6.0 * decay.max(0.3));
        sum
    }
}

/// A plain reverberation: four damped combs and two all-passes either
/// side, of slightly different lengths, so it is wide.
struct Reverb {
    sides: [([Comb; 4], [AllPass; 2]); 2],
}

impl Reverb {
    fn new(rate: f32) -> Reverb {
        let scaled = |samples: usize| (samples as f32 * rate / 44100.0) as usize;
        let side = |spread: usize| {
            (
                [1557, 1617, 1491, 1422].map(|length| Comb::new(scaled(length + spread))),
                [556, 441].map(|length| AllPass::new(scaled(length + spread))),
            )
        };
        Reverb { sides: [side(0), side(23)] }
    }

    fn process(&mut self, input: f32) -> (f32, f32) {
        let [left, right] = self.sides.each_mut().map(|(combs, passes)| {
            let mut out: f32 = combs.iter_mut().map(|comb| comb.process(input)).sum::<f32>() * 0.25;
            for pass in passes.iter_mut() {
                out = pass.process(out);
            }
            out
        });
        (left, right)
    }
}

struct Comb {
    buffer: Vec<f32>,
    index: usize,
    damped: f32,
}

impl Comb {
    const FEEDBACK: f32 = 0.86;
    const DAMPING: f32 = 0.3;

    fn new(length: usize) -> Comb {
        Comb { buffer: vec![0.0; length.max(1)], index: 0, damped: 0.0 }
    }

    fn process(&mut self, input: f32) -> f32 {
        let out = self.buffer[self.index];
        self.damped = out * (1.0 - Self::DAMPING) + self.damped * Self::DAMPING;
        self.buffer[self.index] = input + self.damped * Self::FEEDBACK;
        self.index = (self.index + 1) % self.buffer.len();
        out
    }
}

struct AllPass {
    buffer: Vec<f32>,
    index: usize,
}

impl AllPass {
    fn new(length: usize) -> AllPass {
        AllPass { buffer: vec![0.0; length.max(1)], index: 0 }
    }

    fn process(&mut self, input: f32) -> f32 {
        let delayed = self.buffer[self.index];
        let out = delayed - input;
        self.buffer[self.index] = input + delayed * 0.5;
        self.index = (self.index + 1) % self.buffer.len();
        out
    }
}
