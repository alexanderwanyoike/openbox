use std::{fmt, path::PathBuf};

#[derive(Debug, Clone)]
pub enum DecodeError {
    MissingFile(PathBuf),
    UnreadableFile { path: PathBuf, details: String },
    DecoderUnavailable(String),
    TooLong,
    InvalidAudio(String),
}
impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let filename = |path: &PathBuf| {
            path.file_name()
                .unwrap_or(path.as_os_str())
                .to_string_lossy()
                .into_owned()
        };
        match self {
            Self::MissingFile(path) => write!(
                f,
                "“{}” could not be found. Reconnect its drive or locate the file, then load it again.",
                filename(path)
            ),
            Self::UnreadableFile { path, .. } => write!(
                f,
                "“{}” could not be read. Check file permissions and whether the file plays in another application.",
                filename(path)
            ),
            Self::DecoderUnavailable(_) => f.write_str(
                "Audio preview needs FFmpeg. Check that FFmpeg is installed and available.",
            ),
            Self::TooLong => f.write_str("Preview supports songs up to 30 minutes long."),
            Self::InvalidAudio(_) => {
                f.write_str("This file does not contain playable stereo audio.")
            }
        }
    }
}
impl std::error::Error for DecodeError {}
