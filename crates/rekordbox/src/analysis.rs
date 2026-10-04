use baken_export::anlz::section::{AnlzFile, Section, section};
use openbox_domain::{Beat, TrackAnalysis};
use std::path::Path;

pub(super) fn encode(grid: &TrackAnalysis, duration_ms: f64) -> Result<Section, String> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&0u32.to_be_bytes());
    payload.extend_from_slice(&0x0008_0000u32.to_be_bytes());
    payload.extend_from_slice(&(grid.beats.len() as u32).to_be_bytes());
    let mut previous = None;
    for beat in &grid.beats {
        let time = beat.time_seconds * 1000.0;
        if !time.is_finite()
            || time < 0.0
            || time >= duration_ms
            || !beat.bpm.is_finite()
            || !(1.0..=655.35).contains(&beat.bpm)
            || !(1..=4).contains(&beat.number)
            || previous.is_some_and(|p| time <= p)
        {
            return Err(
                "Beat grid has invalid or unordered beats. Analyze the track again.".into(),
            );
        }
        payload.extend_from_slice(&beat.number.to_be_bytes());
        payload.extend_from_slice(&((beat.bpm * 100.0).round() as u16).to_be_bytes());
        payload.extend_from_slice(&(time.round() as u32).to_be_bytes());
        previous = Some(time);
    }
    Ok(section(b"PQTZ", 0x18, &payload))
}

pub(super) fn read(path: &Path) -> Option<TrackAnalysis> {
    let bytes = std::fs::read(path).ok()?;
    let file = AnlzFile::parse(&bytes).ok()?;
    let section = file.find(b"PQTZ")?;
    let data = section.bytes.get(24..)?;
    let count = u32::from_be_bytes(section.bytes.get(20..24)?.try_into().ok()?) as usize;
    if count > data.len() / 8 {
        return None;
    }
    let mut beats = Vec::new();
    for entry in data[..count * 8].chunks_exact(8) {
        let number = u16::from_be_bytes(entry[0..2].try_into().ok()?);
        let bpm = f64::from(u16::from_be_bytes(entry[2..4].try_into().ok()?)) / 100.0;
        let time_seconds = f64::from(u32::from_be_bytes(entry[4..8].try_into().ok()?)) / 1000.0;
        if !(1..=4).contains(&number)
            || bpm <= 0.0
            || beats
                .last()
                .is_some_and(|b: &Beat| time_seconds <= b.time_seconds)
        {
            return None;
        }
        beats.push(Beat {
            time_seconds,
            bpm,
            number,
        });
    }
    Some(TrackAnalysis {
        version: "Rekordbox USB".into(),
        beats,
    })
}
