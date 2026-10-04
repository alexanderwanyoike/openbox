use openbox_domain::Track;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Default)]
pub struct ImportReport {
    pub tracks: Vec<Track>,
    pub skipped: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ImportProgress {
    pub completed: usize,
    pub total: usize,
    pub track: String,
}

pub fn scan(paths: &[PathBuf], artwork: &Path) -> Result<ImportReport, String> {
    scan_with_progress(paths, artwork, |_| {})
}

pub fn scan_with_progress(
    paths: &[PathBuf],
    artwork: &Path,
    mut progress: impl FnMut(ImportProgress),
) -> Result<ImportReport, String> {
    let mut files = BTreeSet::new();
    for path in paths {
        collect(path, &mut files)?;
    }
    let mut report = ImportReport::default();
    let total = files.len();
    for (index, file) in files.into_iter().enumerate() {
        progress(ImportProgress {
            completed: index,
            total,
            track: file
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
        });
        match read_track(&file, artwork) {
            Ok(track) => report.tracks.push(track),
            Err(error) => report.skipped.push(format!("{}: {error}", file.display())),
        }
    }
    progress(ImportProgress {
        completed: total,
        total,
        track: String::new(),
    });
    Ok(report)
}

fn collect(path: &Path, files: &mut BTreeSet<PathBuf>) -> Result<(), String> {
    let metadata = path.symlink_metadata().map_err(|e| e.to_string())?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    if metadata.is_dir() {
        for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
            collect(&entry.map_err(|e| e.to_string())?.path(), files)?;
        }
    } else if metadata.is_file() && supported(path) {
        files.insert(path.canonicalize().map_err(|e| e.to_string())?);
    }
    Ok(())
}

fn supported(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "mp3" | "wav" | "aif" | "aiff" | "flac" | "m4a" | "aac" | "ogg" | "opus"
    )
}

pub fn read_track(path: &Path, artwork: &Path) -> Result<Track, String> {
    let mut track = openbox_metadata::read(path, artwork)?;
    let result = openbox_analysis::analyze(path)?;
    if result.bpm.is_some() {
        track.details.bpm = result.bpm;
    }
    if !result.key.is_empty() {
        track.details.key = result.key;
    }
    track.details.analysis = Some(result.grid);
    Ok(track)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_import_reads_real_audio_and_skips_duplicates_and_corrupt_files() {
        let music = tempfile::tempdir().unwrap();
        let nested = music.path().join("nested");
        std::fs::create_dir(&nested).unwrap();
        let song = nested.join("Artist - Title.wav");
        openbox_test_support::audio(&song);
        std::fs::write(music.path().join("notes.txt"), "notes").unwrap();
        std::fs::write(music.path().join("broken.mp3"), "not audio").unwrap();
        let report = scan(
            &[music.path().into(), song.clone()],
            &music.path().join("artwork"),
        )
        .unwrap();
        assert_eq!(report.tracks.len(), 1);
        assert_eq!(report.tracks[0].title, "Title");
        assert_eq!(report.tracks[0].artist, "Artist");
        assert_eq!(report.tracks[0].details.duration_seconds, 2);
        assert_eq!(report.tracks[0].path, song.canonicalize().unwrap());
        assert_eq!(report.skipped.len(), 1);
    }

    #[test]
    fn importing_music_runs_analysis_and_keeps_the_source_intact() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Music.wav");
        openbox_test_support::rhythm(&path);
        let before = std::fs::read(&path).unwrap();
        let report = scan(std::slice::from_ref(&path), &dir.path().join("artwork")).unwrap();
        let track = &report.tracks[0];
        assert!((track.details.bpm.unwrap() - 120.0).abs() < 1.0);
        assert_eq!(track.details.key, "C");
        assert!(track.details.analysis.as_ref().unwrap().beats.len() > 40);
        assert_eq!(std::fs::read(path).unwrap(), before);
    }
}
