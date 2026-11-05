use std::{fmt, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncError {
    MissingGrid,
    InvalidGrid,
    PitchRange,
    SameDeck,
}
impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MissingGrid => "Analyze both loaded tracks before syncing",
            Self::InvalidGrid => "The beat grid is invalid",
            Self::PitchRange => "Matching tempo requires pitch outside the supported ±8% range",
            Self::SameDeck => "A deck cannot sync to itself",
        })
    }
}
impl std::error::Error for SyncError {}

#[derive(Debug, Clone)]
pub struct BeatGrid {
    pub(crate) bpm: f64,
    beats: Arc<[f64]>,
}
impl BeatGrid {
    pub fn new(bpm: f64, beats: Vec<f64>) -> Result<Self, SyncError> {
        if !bpm.is_finite()
            || bpm <= 0.0
            || beats.len() < 2
            || beats.iter().any(|t| !t.is_finite() || *t < 0.0)
            || beats.windows(2).any(|p| p[0] >= p[1])
        {
            return Err(SyncError::InvalidGrid);
        }
        Ok(Self {
            bpm,
            beats: beats.into(),
        })
    }
    pub(crate) fn local_bpm(&self, time: f64) -> f64 {
        let index = self
            .beats
            .partition_point(|&at| at <= time)
            .saturating_sub(1);
        let count = 8.min(self.beats.len() - 1);
        let start = index.saturating_sub(4).min(self.beats.len() - 1 - count);
        60.0 * count as f64 / (self.beats[start + count] - self.beats[start])
    }

    pub(crate) fn phase(&self, time: f64) -> (f64, f64) {
        let index = self
            .beats
            .partition_point(|&at| at <= time)
            .saturating_sub(1);
        let at = self.beats[index];
        let period = self
            .beats
            .get(index + 1)
            .map_or(60.0 / self.bpm, |next| next - at);
        (((time - at) / period).rem_euclid(1.0), period)
    }
    pub(crate) fn align(&self, time: f64, phase: f64, duration: f64) -> Option<f64> {
        let (current, period) = self.phase(time);
        (-1..=1)
            .map(|beat| time + (phase - current + f64::from(beat)) * period)
            .filter(|time| (0.0..duration).contains(time))
            .min_by(|a, b| (a - time).abs().total_cmp(&(b - time).abs()))
    }
}
