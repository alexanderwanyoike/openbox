use crate::DecodeError;
use crate::waveform::Waveform;
use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
    sync::Arc,
};

const PREVIEW_RATE: u32 = 44100;

#[derive(Debug, Clone)]
pub struct Clip {
    pub(crate) samples: Arc<[i16]>,
    pub sample_rate: u32,
    pub waveform: Waveform,
}

impl Clip {
    pub fn from_pcm(samples: Vec<i16>, sample_rate: u32) -> Result<Self, String> {
        if sample_rate == 0 || samples.is_empty() || !samples.len().is_multiple_of(2) {
            return Err("Preview audio must contain stereo frames at a valid sample rate".into());
        }
        let waveform = Waveform::from_pcm(&samples, sample_rate);
        Ok(Self {
            samples: samples.into(),
            sample_rate,
            waveform,
        })
    }

    pub fn decode(path: &Path) -> Result<Self, DecodeError> {
        match path.metadata() {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(DecodeError::MissingFile(path.into()));
            }
            Err(error) => {
                return Err(DecodeError::UnreadableFile {
                    path: path.into(),
                    details: error.to_string(),
                });
            }
            Ok(metadata) if !metadata.is_file() => {
                return Err(DecodeError::UnreadableFile {
                    path: path.into(),
                    details: "This path is not a file".into(),
                });
            }
            Ok(_) => {}
        }
        let unreadable = |details: String| DecodeError::UnreadableFile {
            path: path.into(),
            details,
        };
        let mut child = Command::new("ffmpeg")
            .args(["-nostdin", "-v", "error", "-i"])
            .arg(path)
            .args([
                "-map", "0:a:0", "-vn", "-f", "s16le", "-ac", "2", "-ar", "44100", "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| DecodeError::DecoderUnavailable(e.to_string()))?;
        let mut stderr = child.stderr.take().unwrap();
        let diagnostics = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stderr.read_to_end(&mut bytes);
            bytes
        });
        // Only two selected songs are decoded. Bound memory for unusually long recordings.
        let maximum = u64::from(PREVIEW_RATE) * 2 * 2 * 30 * 60;
        let mut bytes = Vec::new();
        let read = child
            .stdout
            .take()
            .unwrap()
            .take(maximum + 1)
            .read_to_end(&mut bytes);
        if read.is_err() || bytes.len() as u64 > maximum {
            let _ = child.kill();
        }
        let status = child.wait().map_err(|e| unreadable(e.to_string()))?;
        let diagnostics = diagnostics.join().unwrap_or_default();
        read.map_err(|e| unreadable(e.to_string()))?;
        if bytes.len() as u64 > maximum {
            return Err(DecodeError::TooLong);
        }
        if !status.success() {
            return Err(unreadable(
                String::from_utf8_lossy(&diagnostics).trim().into(),
            ));
        }
        let samples = bytes
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
            .collect();
        Self::from_pcm(samples, PREVIEW_RATE).map_err(DecodeError::InvalidAudio)
    }

    pub fn duration(&self) -> f64 {
        self.samples.len() as f64 / 2.0 / f64::from(self.sample_rate)
    }

    pub(crate) fn sample(&self, frame: usize, channel: usize) -> f32 {
        self.samples.get(frame * 2 + channel).copied().unwrap_or(0) as f32 / 32768.0
    }
}
