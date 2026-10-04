use crate::Clip;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckId {
    A,
    B,
}

impl DeckId {
    pub fn index(self) -> usize {
        match self {
            Self::A => 0,
            Self::B => 1,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DeckControl {
    PlayPause,
    Cue,
    SetCue,
    Seek(f64),
    Rate(f64),
    Gain(f32),
    Fader(f32),
    Eq(Band, f32),
}

#[derive(Debug, Clone, Copy)]
pub enum Band {
    Low,
    Mid,
    High,
}

impl Band {
    pub fn index(self) -> usize {
        match self {
            Self::Low => 0,
            Self::Mid => 1,
            Self::High => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DeckSnapshot {
    pub position: f64,
    pub duration: f64,
    pub playing: bool,
    pub rate: f64,
    pub peak: f32,
    pub cue: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Snapshot {
    pub decks: [DeckSnapshot; 2],
    pub peak: f32,
    pub clipped: bool,
}

struct Deck {
    clip: Option<Clip>,
    cursor: f64,
    playing: bool,
    rate: f64,
    gain: f32,
    fader: f32,
    peak: f32,
    cue: f64,
    eq: [f32; 3],
    low: [f32; 2],
    upper: [f32; 2],
}

impl Default for Deck {
    fn default() -> Self {
        Self {
            clip: None,
            cursor: 0.0,
            playing: false,
            rate: 1.0,
            gain: 0.8,
            fader: 1.0,
            peak: 0.0,
            cue: 0.0,
            eq: [1.0; 3],
            low: [0.0; 2],
            upper: [0.0; 2],
        }
    }
}

pub struct Mixer {
    decks: [Deck; 2],
    crossfade: f32,
    master: f32,
    peak: f32,
    clipped: bool,
}

impl Default for Mixer {
    fn default() -> Self {
        Self {
            decks: [Deck::default(), Deck::default()],
            crossfade: 0.5,
            master: 0.8,
            peak: 0.0,
            clipped: false,
        }
    }
}

impl Mixer {
    pub fn load(&mut self, id: DeckId, clip: Clip) {
        let deck = &mut self.decks[id.index()];
        deck.clip = Some(clip);
        deck.cursor = 0.0;
        deck.playing = false;
        deck.rate = 1.0;
        deck.peak = 0.0;
        deck.cue = 0.0;
        deck.low = [0.0; 2];
        deck.upper = [0.0; 2];
    }

    pub fn control(&mut self, id: DeckId, control: DeckControl) {
        let deck = &mut self.decks[id.index()];
        match control {
            DeckControl::PlayPause if deck.clip.is_some() => {
                if deck
                    .clip
                    .as_ref()
                    .is_some_and(|c| deck.cursor >= (c.samples.len() / 2) as f64)
                {
                    deck.cursor = 0.0;
                }
                deck.playing = !deck.playing;
            }
            DeckControl::Seek(seconds) if seconds.is_finite() => {
                if let Some(clip) = &deck.clip {
                    deck.cursor = seconds.clamp(0.0, clip.duration()) * f64::from(clip.sample_rate);
                }
                deck.low = [0.0; 2];
                deck.upper = [0.0; 2];
            }
            DeckControl::Cue => {
                deck.cursor = deck.cue;
                deck.playing = false;
                deck.low = [0.0; 2];
                deck.upper = [0.0; 2];
            }
            DeckControl::SetCue => {
                deck.cue = deck.cursor;
                deck.playing = false;
            }
            DeckControl::Rate(rate) if rate.is_finite() => deck.rate = rate.clamp(0.92, 1.08),
            DeckControl::Gain(gain) if gain.is_finite() => deck.gain = gain.clamp(0.0, 1.0),
            DeckControl::Fader(value) if value.is_finite() => deck.fader = value.clamp(0.0, 1.0),
            DeckControl::Eq(band, db) if db.is_finite() => {
                deck.eq[band.index()] = 10.0_f32.powf(db.clamp(-24.0, 6.0) / 20.0)
            }
            _ => {}
        }
    }

    pub fn set_crossfade(&mut self, value: f32) {
        if value.is_finite() {
            self.crossfade = value.clamp(0.0, 1.0);
        }
    }

    pub fn set_master(&mut self, value: f32) {
        if value.is_finite() {
            self.master = value.clamp(0.0, 1.0);
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            decks: std::array::from_fn(|i| {
                let d = &self.decks[i];
                DeckSnapshot {
                    position: d
                        .clip
                        .as_ref()
                        .map_or(0.0, |c| d.cursor / f64::from(c.sample_rate)),
                    duration: d.clip.as_ref().map_or(0.0, Clip::duration),
                    playing: d.playing,
                    rate: d.rate,
                    peak: d.peak,
                    cue: d
                        .clip
                        .as_ref()
                        .map_or(0.0, |c| d.cue / f64::from(c.sample_rate)),
                }
            }),
            peak: self.peak,
            clipped: self.clipped,
        }
    }

    pub fn render(&mut self, output: &mut [f32], sample_rate: u32, channels: usize) {
        output.fill(0.0);
        if sample_rate == 0 || channels == 0 {
            return;
        }
        let angle = self.crossfade * std::f32::consts::FRAC_PI_2;
        let weights = [angle.cos(), angle.sin()];
        let low_coefficient = 1.0 - (-std::f32::consts::TAU * 250.0 / sample_rate as f32).exp();
        let upper_coefficient = 1.0 - (-std::f32::consts::TAU * 4000.0 / sample_rate as f32).exp();
        self.peak *= 0.9;
        self.clipped = false;
        for deck in &mut self.decks {
            deck.peak *= 0.9;
        }
        for frame in output.chunks_exact_mut(channels) {
            let mut stereo = [0.0; 2];
            for (i, deck) in self.decks.iter_mut().enumerate() {
                let Some(clip) = &deck.clip else { continue };
                if !deck.playing {
                    continue;
                }
                let frames = clip.samples.len() / 2;
                if deck.cursor >= frames as f64 {
                    deck.playing = false;
                    deck.cursor = frames as f64;
                    continue;
                }
                let index = deck.cursor as usize;
                let blend = (deck.cursor - index as f64) as f32;
                for (channel, mixed) in stereo.iter_mut().enumerate() {
                    let a = clip.sample(index, channel);
                    let b = clip.sample((index + 1).min(frames - 1), channel);
                    let sample = a + (b - a) * blend;
                    deck.low[channel] += low_coefficient * (sample - deck.low[channel]);
                    deck.upper[channel] += upper_coefficient * (sample - deck.upper[channel]);
                    let value = (deck.low[channel] * deck.eq[0]
                        + (deck.upper[channel] - deck.low[channel]) * deck.eq[1]
                        + (sample - deck.upper[channel]) * deck.eq[2])
                        * deck.gain;
                    deck.peak = deck.peak.max(value.abs());
                    *mixed += value * deck.fader * weights[i] * self.master;
                }
                deck.cursor += f64::from(clip.sample_rate) / f64::from(sample_rate) * deck.rate;
                if deck.cursor >= frames as f64 {
                    deck.cursor = frames as f64;
                    deck.playing = false;
                }
            }
            for (channel, sample) in frame.iter_mut().enumerate() {
                let value = if channels == 1 {
                    (stereo[0] + stereo[1]) * 0.5
                } else if channel < 2 {
                    stereo[channel]
                } else {
                    0.0
                };
                self.clipped |= value.abs() > 1.0;
                self.peak = self.peak.max(value.abs().min(1.0));
                *sample = value.clamp(-1.0, 1.0);
            }
        }
    }
}
