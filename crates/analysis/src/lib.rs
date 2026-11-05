mod grid;
mod key;

use aubio::{OnsetMode, Tempo};
use openbox_domain::{Beat, TrackAnalysis};
use std::{io::Read, path::Path, process::Stdio, sync::Mutex};

const RATE: u32 = 22050;
const HOP: usize = 64;
static KEYFINDER_LOCK: Mutex<()> = Mutex::new(());

pub const VERSION: &str = "bliss-aubio-0.2.4/libkeyfinder-2.2.8/mixxx-grid-1";

pub struct Analysis {
    pub bpm: Option<f64>,
    pub key: String,
    pub grid: TrackAnalysis,
}

pub fn analyze(path: &Path) -> Result<Analysis, String> {
    // FFTW planner state is global, including inside otherwise independent keyfinder instances.
    let _guard = KEYFINDER_LOCK.lock().map_err(|e| e.to_string())?;
    let mut key = key::Key::new()?;
    let mut tempo = Tempo::new(OnsetMode::SpecFlux, 1024, HOP, RATE).map_err(|e| e.to_string())?;
    let mut child = openbox_media_tools::Tools::system()
        .ffmpeg()
        .args(["-nostdin", "-v", "error", "-i"])
        .arg(path)
        .args([
            "-map", "0:a:0", "-vn", "-ac", "1", "-ar", "22050", "-f", "f32le", "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut errors = child.stderr.take().unwrap();
    let diagnostics = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = errors.read_to_end(&mut bytes);
        bytes
    });
    let mut pipe = child.stdout.take().unwrap();
    let mut buffer = [0u8; HOP * 4];
    let mut beats: Vec<Beat> = Vec::new();
    let mut bpms = Vec::new();
    let mut sample_count = 0u64;
    let result: Result<(), String> = (|| {
        loop {
            let mut count = 0;
            while count < buffer.len() {
                let read = pipe.read(&mut buffer[count..]).map_err(|e| e.to_string())?;
                if read == 0 {
                    break;
                }
                count += read;
            }
            if count == 0 {
                break;
            }
            if !count.is_multiple_of(4) {
                return Err("Decoder returned incomplete audio samples".into());
            }
            let mut samples = [0.0; HOP];
            for (sample, bytes) in samples.iter_mut().zip(buffer[..count].chunks_exact(4)) {
                *sample = f32::from_le_bytes(bytes.try_into().unwrap());
            }
            key.push(&samples[..count / 4])?;
            sample_count += (count / 4) as u64;
            if tempo.do_result(&samples[..]).map_err(|e| e.to_string())? > 0.0 {
                let time = f64::from(tempo.get_last_s());
                let bpm = f64::from(tempo.get_bpm());
                if bpm.is_finite()
                    && (20.0..=400.0).contains(&bpm)
                    && time.is_finite()
                    && time >= 0.0
                    && beats.last().is_none_or(|last| time > last.time_seconds)
                {
                    beats.push(Beat {
                        time_seconds: time,
                        bpm,
                        number: (beats.len() % 4 + 1) as u16,
                    });
                    if bpm.is_finite() && (20.0..=400.0).contains(&bpm) {
                        bpms.push(bpm);
                    }
                }
            }
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = child.kill();
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let diagnostics = diagnostics.join().unwrap_or_default();
    result?;
    if !status.success() {
        return Err(format!(
            "Cannot decode audio: {}",
            String::from_utf8_lossy(&diagnostics).trim()
        ));
    }
    bpms.sort_by(f64::total_cmp);
    let mut bpm = (beats.len() >= 4)
        .then(|| bpms.get(bpms.len() / 2).copied())
        .flatten();
    if bpm.is_none() {
        beats.clear();
    } else if let Some((stable_bpm, stable_beats)) =
        grid::stabilize(&beats, sample_count as f64 / f64::from(RATE))
    {
        bpm = Some(stable_bpm);
        beats = stable_beats;
    }
    Ok(Analysis {
        bpm,
        key: key.finish()?,
        grid: TrackAnalysis {
            version: VERSION.into(),
            beats,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_tempo_key_and_beats_from_audio_without_tags() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("rhythm.wav");
        openbox_test_support::rhythm(&path);
        let Analysis { bpm, key, grid } = analyze(&path).unwrap();
        let beats: Vec<_> = grid.beats.iter().map(|b| b.time_seconds).collect();
        assert!((bpm.unwrap() - 120.0).abs() < 0.5, "{bpm:?}");
        assert_eq!(key, "C");
        assert!(beats.len() > 40);
        assert!(beats.windows(2).all(|b| b[1] > b[0]));
        for beat in beats.iter().skip(10) {
            assert!((beat * 2.0 - (beat * 2.0).round()).abs() < 0.15, "{beat}");
        }
    }

    #[test]
    fn a_constant_tempo_recording_produces_an_even_grid_instead_of_detector_jitter() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("constant-tempo.wav");
        openbox_test_support::rhythm(&path);
        let analysis = analyze(&path).unwrap();
        let bpm = analysis.bpm.unwrap();
        assert!((bpm - 120.0).abs() < 0.01, "{bpm}");
        let interval = 60.0 / bpm;
        assert!(analysis.grid.beats.len() > 60);
        for pair in analysis.grid.beats.windows(2) {
            let distance = pair[1].time_seconds - pair[0].time_seconds;
            assert!(
                (distance - interval).abs() < 0.000_001,
                "CDJ grid interval {distance} does not match the track tempo {bpm}"
            );
        }
        assert!(analysis.grid.beats[0].time_seconds < interval);
    }

    #[test]
    fn silence_has_no_invented_key_tempo_or_grid_and_corrupt_audio_fails() {
        let directory = tempfile::tempdir().unwrap();
        let silent = directory.path().join("silence.wav");
        let status = openbox_media_tools::Tools::system()
            .ffmpeg()
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "anullsrc=r=22050:cl=mono",
                "-t",
                "10",
            ])
            .arg(&silent)
            .status()
            .unwrap();
        assert!(status.success());
        let result = analyze(&silent).unwrap();
        assert!(result.bpm.is_none());
        assert!(result.key.is_empty());
        assert!(result.grid.beats.is_empty());
        let broken = directory.path().join("broken.mp3");
        std::fs::write(&broken, "not audio").unwrap();
        assert!(analyze(&broken).is_err());
    }
}
