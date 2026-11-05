use std::{path::PathBuf, process::Command};

pub struct Tools {
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
}

impl Tools {
    pub fn system() -> Self {
        let directories = std::env::var_os("PATH")
            .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
            .unwrap_or_default();
        #[cfg(target_os = "macos")]
        let directories = directories.into_iter().chain([
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
        ]);
        Self::in_directories(directories)
    }

    pub fn in_directories(directories: impl IntoIterator<Item = PathBuf>) -> Self {
        let paths = std::env::join_paths(directories).ok();
        let find = |name| {
            which::which_in(name, paths.as_ref(), ".").unwrap_or_else(|_| PathBuf::from(name))
        };
        Self {
            ffmpeg: find("ffmpeg"),
            ffprobe: find("ffprobe"),
        }
    }

    pub fn ffmpeg(&self) -> Command {
        command(&self.ffmpeg)
    }

    pub fn ffprobe(&self) -> Command {
        command(&self.ffprobe)
    }
}

#[cfg(windows)]
fn command(program: &std::path::Path) -> Command {
    use std::os::windows::process::CommandExt;
    let mut command = Command::new(program);
    // Audio decoding runs in the background without opening a console per track.
    command.creation_flags(0x08000000);
    command
}

#[cfg(not(windows))]
fn command(program: &std::path::Path) -> Command {
    Command::new(program)
}
