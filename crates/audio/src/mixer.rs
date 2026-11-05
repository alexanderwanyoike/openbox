use crate::{BeatGrid, Clip, SyncError};

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
    pub pitch_rate: f64,
    pub peak: f32,
    pub cue: f64,
    pub sync_leader: Option<DeckId>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Snapshot {
    pub decks: [DeckSnapshot; 2],
    pub peak: f32,
    pub clipped: bool,
}

struct Deck {
    grid: Option<BeatGrid>,
    sync_leader: Option<DeckId>,
    clip: Option<Clip>,
    cursor: f64,
    playing: bool,
    rate: f64,
    adjustment: f64,
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
            grid: None,
            sync_leader: None,
            clip: None,
            cursor: 0.0,
            playing: false,
            rate: 1.0,
            adjustment: 1.0,
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
        for deck in &mut self.decks {
            if deck.sync_leader == Some(id) {
                deck.sync_leader = None;
            }
        }
        let deck = &mut self.decks[id.index()];
        deck.clip = Some(clip);
        deck.grid = None;
        deck.sync_leader = None;
        deck.cursor = 0.0;
        deck.playing = false;
        deck.rate = 1.0;
        deck.adjustment = 1.0;
        deck.peak = 0.0;
        deck.cue = 0.0;
        deck.low = [0.0; 2];
        deck.upper = [0.0; 2];
    }

    pub fn control(&mut self, id: DeckId, control: DeckControl) {
        let manual = matches!(
            control,
            DeckControl::Seek(_) | DeckControl::Rate(_) | DeckControl::Cue | DeckControl::SetCue
        );
        if manual {
            self.decks[id.index()].sync_leader = None;
            self.decks[id.index()].adjustment = 1.0;
        }
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
        for follower in [DeckId::A, DeckId::B] {
            let leader = self.decks[follower.index()].sync_leader;
            if leader.is_some() && (follower == id || leader == Some(id)) {
                if matches!(control, DeckControl::Rate(_)) {
                    self.follow_tempo(follower);
                } else if matches!(
                    control,
                    DeckControl::Seek(_) | DeckControl::Cue | DeckControl::PlayPause
                ) {
                    self.align_phase(follower);
                }
            }
        }
    }

    pub fn set_beat_grid(&mut self, id: DeckId, grid: BeatGrid) {
        self.clear_beat_grid(id);
        self.decks[id.index()].grid = Some(grid);
    }
    pub fn clear_beat_grid(&mut self, id: DeckId) {
        for (index, deck) in self.decks.iter_mut().enumerate() {
            if index == id.index() || deck.sync_leader == Some(id) {
                deck.sync_leader = None;
                deck.adjustment = 1.0;
            }
        }
        self.decks[id.index()].grid = None;
    }
    pub fn release_sync(&mut self, id: DeckId) {
        self.decks[id.index()].sync_leader = None;
        self.decks[id.index()].adjustment = 1.0;
    }
    pub fn sync_to(&mut self, follower: DeckId, leader: DeckId) -> Result<(), SyncError> {
        if follower == leader {
            return Err(SyncError::SameDeck);
        }
        let target = &self.decks[follower.index()];
        let source = &self.decks[leader.index()];
        let (Some(grid), Some(master_grid), Some(clip), Some(_)) =
            (&target.grid, &source.grid, &target.clip, &source.clip)
        else {
            return Err(SyncError::MissingGrid);
        };
        let source_time = source.cursor / f64::from(source.clip.as_ref().unwrap().sample_rate);
        let target_time = target.cursor / f64::from(clip.sample_rate);
        let rate = master_grid.local_bpm(source_time) * source.rate / grid.local_bpm(target_time);
        if !(0.92..=1.08).contains(&rate) {
            return Err(SyncError::PitchRange);
        }
        let phase = master_grid
            .phase(source.cursor / f64::from(source.clip.as_ref().unwrap().sample_rate))
            .0;
        let position = grid
            .align(
                target.cursor / f64::from(clip.sample_rate),
                phase,
                clip.duration(),
            )
            .ok_or(SyncError::InvalidGrid)?;
        self.decks[leader.index()].sync_leader = None;
        let target = &mut self.decks[follower.index()];
        target.sync_leader = Some(leader);
        target.rate = rate;
        target.adjustment = 1.0;
        target.cursor = position * f64::from(target.clip.as_ref().unwrap().sample_rate);
        target.low = [0.0; 2];
        target.upper = [0.0; 2];
        Ok(())
    }
    fn follow_tempo(&mut self, follower: DeckId) {
        let Some(leader) = self.decks[follower.index()].sync_leader else {
            return;
        };
        let source = &self.decks[leader.index()];
        let target = &self.decks[follower.index()];
        let rate =
            source
                .grid
                .as_ref()
                .unwrap()
                .local_bpm(source.cursor / f64::from(source.clip.as_ref().unwrap().sample_rate))
                * source.rate
                / target.grid.as_ref().unwrap().local_bpm(
                    target.cursor / f64::from(target.clip.as_ref().unwrap().sample_rate),
                );
        if (0.92..=1.08).contains(&rate) {
            self.decks[follower.index()].rate = rate;
        } else {
            self.decks[follower.index()].sync_leader = None;
        }
    }
    fn align_phase(&mut self, follower: DeckId) {
        let Some(leader) = self.decks[follower.index()].sync_leader else {
            return;
        };
        let _ = self.sync_to(follower, leader);
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
                    rate: d.rate * d.adjustment,
                    pitch_rate: d.rate,
                    peak: d.peak,
                    sync_leader: d.sync_leader,
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
        for follower in [DeckId::A, DeckId::B] {
            self.follow_tempo(follower);
            let target = &self.decks[follower.index()];
            let adjustment = if let Some(leader) = target.sync_leader {
                let source = &self.decks[leader.index()];
                if target.playing && source.playing {
                    let phase = |deck: &Deck| {
                        deck.grid
                            .as_ref()
                            .unwrap()
                            .phase(deck.cursor / f64::from(deck.clip.as_ref().unwrap().sample_rate))
                            .0
                    };
                    let error = (phase(source) - phase(target) + 0.5).rem_euclid(1.0) - 0.5;
                    (1.0 + error * 0.5).clamp(0.98, 1.02)
                } else {
                    1.0
                }
            } else {
                1.0
            };
            self.decks[follower.index()].adjustment = adjustment;
        }
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
                deck.cursor += f64::from(clip.sample_rate) / f64::from(sample_rate)
                    * deck.rate
                    * deck.adjustment;
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
