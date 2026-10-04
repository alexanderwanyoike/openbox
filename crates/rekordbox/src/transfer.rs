use super::{
    DATABASE, DeviceLibrary, ExportPhase, ExportProgress, SourceLink,
    database::Database,
    open, safe_path,
    write::{DeviceWrite, commit},
};
use baken_export::{
    anlz::{generate, hash::AnlzSlots, rewrite},
    build::{self, DeviceTrack},
    collection, pdb,
};
use openbox_domain::{Folder, Library, Playlist, Track};
use openbox_storage as storage;
use std::{
    collections::{HashMap, HashSet},
    io::{BufRead, BufReader, Read},
    path::Path,
    process::{Command, Stdio},
};

pub(super) enum Transfer<'a> {
    Playlists(&'a [u64]),
    Tracks {
        tracks: &'a [u64],
        playlist: u64,
        replace: Option<u64>,
    },
}

pub(super) fn run(
    library: &Library,
    operation: Transfer<'_>,
    root: &Path,
    expected: Option<&[u8]>,
    progress: &mut dyn FnMut(ExportProgress),
) -> Result<DeviceLibrary, String> {
    progress(ExportProgress {
        percent: 0.0,
        phase: ExportPhase::Checking,
        track: String::new(),
        completed: 0,
        total: 0,
    });
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let _lock = DeviceWrite::acquire(&root)?;
    let previous = if safe_path(&root, DATABASE)?.exists() {
        Some(open(&root)?)
    } else {
        if safe_path(&root, "PIONEER")?.exists() {
            return Err("This USB has no supported traditional Rekordbox library".into());
        }
        None
    };
    let previous_bytes = previous.as_ref().map(|d| d.database.bytes.as_slice());
    if expected.is_some() && expected != previous_bytes {
        return Err("USB library changed. Refresh it before saving.".into());
    }
    let base = match &previous {
        Some(device) => device.database.clone(),
        None => Database::read(pdb::write(&pdb::Export {
            device_name: "Openbox".into(),
            export_date: build::today(),
            ..Default::default()
        }))?,
    };
    let target = base.library(&root)?;
    let requested = operation.requested(library)?;
    let total = requested.tracks().len();
    let mut report = |percent, phase, completed| {
        progress(ExportProgress {
            percent,
            phase,
            track: String::new(),
            completed,
            total,
        })
    };
    report(3.0, ExportPhase::Checking, 0);
    let sources = previous
        .as_ref()
        .map(|d| d.sources.clone())
        .unwrap_or_default();
    let staged = StagedTransfer::prepare(&requested, &target, sources, &root, progress)?;
    let updated = operation.merge(
        &base,
        &target,
        staged.tracks.clone(),
        &requested,
        &staged.mapping,
    )?;
    let output = base.with_tracks(staged.added.clone(), &updated, &staged.artwork)?;
    let mut report = |percent, phase| {
        progress(ExportProgress {
            percent,
            phase,
            track: String::new(),
            completed: total,
            total,
        })
    };
    report(95.0, ExportPhase::Publishing);
    let mut published = staged.publish(&root, &mut |fraction| {
        report(95.0 + 3.0 * fraction, ExportPhase::Publishing)
    })?;
    report(99.0, ExportPhase::Saving);
    if let Err(error) = commit(&root, &output, previous_bytes, &updated, &staged.sources) {
        let unchanged = match std::fs::read(safe_path(&root, DATABASE)?) {
            Ok(bytes) => previous_bytes == Some(bytes.as_slice()),
            Err(e) => previous.is_none() && e.kind() == std::io::ErrorKind::NotFound,
        };
        // An uncertain commit must retain its audio in case the new database reached the device.
        published.keep = !unchanged;
        return Err(error);
    }
    published.keep = true;
    let device = open(&root)?;
    report(100.0, ExportPhase::Complete);
    Ok(device)
}

impl Transfer<'_> {
    fn requested(&self, library: &Library) -> Result<Library, String> {
        match self {
            Self::Playlists(playlists) => selected(library, playlists),
            Self::Tracks { tracks, .. } => {
                if tracks.is_empty() {
                    return Err("Choose a song from the library".into());
                }
                let selected = tracks
                    .iter()
                    .map(|id| {
                        library
                            .track(*id)
                            .cloned()
                            .ok_or("Library song no longer exists".to_string())
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Library::from_parts(selected, Vec::new(), Vec::new(), 0)
            }
        }
    }

    fn merge(
        &self,
        base: &Database,
        target: &Library,
        tracks: Vec<Track>,
        requested: &Library,
        mapping: &HashMap<u64, u64>,
    ) -> Result<Library, String> {
        let mut playlists = target.playlists().to_vec();
        let mut folders = target.folders().to_vec();
        let mut next = base.next_playlist_id()?;
        match self {
            Self::Playlists(_) => {
                let mut parents = HashMap::new();
                for playlist in requested.playlists() {
                    let mut parent = None;
                    for folder in requested.ancestors(playlist.parent) {
                        if playlists.iter().any(|p| {
                            p.parent == parent && p.name.eq_ignore_ascii_case(&folder.name)
                        }) {
                            return Err("A USB playlist already uses this folder name".into());
                        }
                        let id = if let Some(id) = parents.get(&folder.id) {
                            *id
                        } else if let Some(existing) = folders.iter().find(|f| {
                            f.parent == parent && f.name.eq_ignore_ascii_case(&folder.name)
                        }) {
                            existing.id
                        } else {
                            let id = next;
                            next += 1;
                            let sort_order = folders
                                .iter()
                                .filter(|f| f.parent == parent)
                                .map(|f| f.sort_order)
                                .chain(
                                    playlists
                                        .iter()
                                        .filter(|p| p.parent == parent)
                                        .map(|p| p.sort_order),
                                )
                                .max()
                                .map_or(0, |n| n + 1);
                            folders.push(Folder {
                                id,
                                name: folder.name.clone(),
                                parent,
                                sort_order,
                            });
                            id
                        };
                        parents.insert(folder.id, id);
                        parent = Some(id);
                    }
                    let existing = playlists.iter().find(|p| {
                        p.parent == parent && p.name.eq_ignore_ascii_case(&playlist.name)
                    });
                    let (id, sort_order) = if let Some(existing) = existing {
                        (existing.id, existing.sort_order)
                    } else {
                        let id = next;
                        next += 1;
                        let order = folders
                            .iter()
                            .filter(|f| f.parent == parent)
                            .map(|f| f.sort_order)
                            .chain(
                                playlists
                                    .iter()
                                    .filter(|p| p.parent == parent)
                                    .map(|p| p.sort_order),
                            )
                            .max()
                            .map_or(0, |n| n + 1);
                        (id, order)
                    };
                    if folders
                        .iter()
                        .any(|f| f.parent == parent && f.name.eq_ignore_ascii_case(&playlist.name))
                    {
                        return Err("A USB folder already uses this playlist name".into());
                    }
                    playlists.retain(|p| p.id != id);
                    playlists.push(Playlist {
                        id,
                        name: playlist.name.clone(),
                        parent,
                        sort_order,
                        track_ids: playlist.track_ids.iter().map(|id| mapping[id]).collect(),
                    });
                }
            }
            Self::Tracks {
                tracks,
                playlist,
                replace,
            } => {
                let target = playlists
                    .iter_mut()
                    .find(|p| p.id == *playlist)
                    .ok_or("USB playlist no longer exists")?;
                let added: Vec<_> = tracks.iter().map(|id| mapping[id]).collect();
                if let Some(old) = replace {
                    let position = target
                        .track_ids
                        .iter()
                        .position(|id| id == old)
                        .ok_or("The song to replace is no longer in this playlist")?;
                    target.track_ids[position] = added[0];
                } else {
                    for id in added {
                        if !target.track_ids.contains(&id) {
                            target.track_ids.push(id);
                        }
                    }
                }
            }
        }
        Library::from_parts(tracks, playlists, folders, next - 1)
    }
}

struct StagedTransfer {
    folder: tempfile::TempDir,
    files: Vec<String>,
    tracks: Vec<Track>,
    mapping: HashMap<u64, u64>,
    sources: Vec<SourceLink>,
    added: pdb::Export,
    artwork: Vec<(u32, String)>,
}

impl StagedTransfer {
    fn prepare(
        requested: &Library,
        target: &Library,
        mut sources: Vec<SourceLink>,
        root: &Path,
        progress: &mut dyn FnMut(ExportProgress),
    ) -> Result<Self, String> {
        let folder = tempfile::Builder::new()
            .prefix(".openbox-stage-")
            .tempdir_in(root)
            .map_err(|e| e.to_string())?;
        let generation = folder
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .replace('.', "");
        let mut tracks = target.tracks().to_vec();
        let mut next = tracks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        let mut mapping = HashMap::new();
        let mut added_tracks = Vec::new();
        let mut files = Vec::new();
        let mut slots = AnlzSlots::default();
        let mut assigned_ids = Vec::new();
        let mut artwork = Vec::new();
        for (index, track) in requested.tracks().iter().enumerate() {
            let mut report = |fraction: f32, phase| {
                progress(ExportProgress {
                    percent: 3.0
                        + 92.0 * (index as f32 + fraction) / requested.tracks().len().max(1) as f32,
                    phase,
                    track: track.title.clone(),
                    completed: index + usize::from(fraction >= 1.0),
                    total: requested.tracks().len(),
                })
            };
            report(0.0, ExportPhase::Checking);
            let path = track
                .path
                .canonicalize()
                .map_err(|e| format!("Cannot read {}: {e}", track.title))?;
            let source = source_fingerprint(track)?;
            let reusable = sources
                .iter()
                .find(|link| link.path == path && link.source == source)
                .and_then(|link| target.track(link.track_id).map(|track| (link, track)))
                .filter(|(link, track)| {
                    storage::fingerprint(&track.path).is_ok_and(|hash| hash == link.audio)
                });
            if let Some((link, _)) = reusable {
                mapping.insert(track.id, link.track_id);
                report(1.0, ExportPhase::Checking);
                continue;
            }
            if next > u32::MAX as u64 - 20 {
                return Err("Device track IDs exhausted".into());
            }
            let song = prepare_song(
                track,
                next,
                &generation,
                root,
                folder.path(),
                &mut slots,
                &mut report,
            )?;
            if source_fingerprint(track)? != source {
                return Err("A source song changed during transfer. Try again.".into());
            }
            sources.retain(|link| link.path != path);
            sources.push(SourceLink {
                path,
                source,
                audio: storage::fingerprint(&song.staged_audio)?,
                track_id: next,
            });
            mapping.insert(track.id, next);
            assigned_ids.push(next);
            next += 1;
            files.extend(song.files);
            if let Some(path) = song.artwork {
                artwork.push((added_tracks.len() as u32 + 1, path));
            }
            tracks.push(song.track);
            added_tracks.push(song.backend);
            report(1.0, ExportPhase::Waveforms);
        }
        let mut added = build::build(
            &collection::Library::default(),
            &added_tracks,
            &[],
            "Openbox",
            &build::today(),
        );
        for (row, id) in added.tracks.iter_mut().zip(assigned_ids) {
            row.id = id as u32;
        }
        for (id, _) in &artwork {
            added.tracks[*id as usize - 1].artwork_id = *id;
        }
        Ok(Self {
            folder,
            files,
            tracks,
            mapping,
            sources,
            added,
            artwork,
        })
    }

    fn publish(
        &self,
        root: &Path,
        progress: &mut dyn FnMut(f32),
    ) -> Result<PublishedFiles, String> {
        let mut published = PublishedFiles::default();
        for relative in &self.files {
            let destination = safe_path(root, relative)?;
            std::fs::create_dir_all(destination.parent().unwrap()).map_err(|e| e.to_string())?;
            let source = safe_path(self.folder.path(), relative)?;
            std::fs::File::open(&source)
                .and_then(|f| f.sync_all())
                .map_err(|e| e.to_string())?;
            tempfile::TempPath::try_from_path(source)
                .map_err(|e| e.to_string())?
                .persist_noclobber(&destination)
                .map_err(|e| e.to_string())?;
            published.files.push(destination);
            progress(published.files.len() as f32 / self.files.len().max(1) as f32);
        }
        Ok(published)
    }
}

struct PreparedSong {
    track: Track,
    backend: DeviceTrack,
    files: Vec<String>,
    staged_audio: std::path::PathBuf,
    artwork: Option<String>,
}

fn prepare_song(
    source: &Track,
    id: u64,
    generation: &str,
    root: &Path,
    staging: &Path,
    slots: &mut AnlzSlots,
    progress: &mut dyn FnMut(f32, ExportPhase),
) -> Result<PreparedSong, String> {
    let mut analyzed = source.clone();
    if analyzed.details.analysis.is_none() {
        let result = openbox_analysis::analyze(&analyzed.path)?;
        if result.bpm.is_some() {
            analyzed.details.bpm = result.bpm;
        }
        if !result.key.is_empty() {
            analyzed.details.key = result.key;
        }
        analyzed.details.analysis = Some(result.grid);
    }
    let source = &analyzed;
    let relative = format!(
        "Contents/Openbox/{generation}/{}-{}.mp3",
        source.id,
        filename(&source.title)
    );
    let output = safe_path(staging, &relative)?;
    std::fs::create_dir_all(output.parent().unwrap()).map_err(|e| e.to_string())?;
    transcode(source, &output, &mut |fraction| {
        progress(0.65 * fraction, ExportPhase::Encoding)
    })?;
    openbox_metadata::write_mp3(source, &output)?;
    progress(0.65, ExportPhase::Waveforms);
    let measured = generate::measure(&output).map_err(|e| e.to_string())?;
    progress(0.85, ExportPhase::Waveforms);
    let mut track = source.clone();
    track.id = id;
    track.path = safe_path(root, &relative)?;
    track.details.duration_seconds = (measured.duration_ms() / 1000.0).round() as u32;
    if let Some(grid) = &mut track.details.analysis {
        // CBR without Xing retains LAME's 576 encoder + 529 decoder priming samples.
        for beat in &mut grid.beats {
            beat.time_seconds += 1105.0 / 44100.0;
        }
    }
    let backend = backend_track(&track);
    let usb_path = format!("/{relative}");
    let (directory, index) = loop {
        let location = slots.assign(&usb_path);
        if !safe_path(root, &format!("{}/ANLZ{:04}.DAT", location.0, location.1))?.exists() {
            break location;
        }
    };
    let frames = rewrite::mp3_audio(&output)
        .map_err(|e| e.to_string())?
        .frames;
    let [mut dat, ext, _] = generate::build_files(&backend, &usb_path, &measured, Some(frames));
    if let Some(grid) = &track.details.analysis {
        *dat.find_mut(b"PQTZ")
            .ok_or("Generated analysis has no beat-grid section")? =
            super::analysis::encode(grid, measured.duration_ms())?;
    }
    let mut files = vec![relative];
    let artwork = if let Some(cover) = &source.details.artwork {
        let [small, large] = openbox_metadata::device_covers(cover)?;
        let relative = format!("PIONEER/Artwork/{generation}/art{id}.jpg");
        let enlarged = format!("PIONEER/Artwork/{generation}/art{id}_m.jpg");
        storage::write_atomic(&safe_path(staging, &relative)?, &small)?;
        storage::write_atomic(&safe_path(staging, &enlarged)?, &large)?;
        track.details.artwork = Some(safe_path(root, &enlarged)?);
        files.extend([relative.clone(), enlarged]);
        Some(format!("/{relative}"))
    } else {
        None
    };
    for (extension, analysis) in [("DAT", dat), ("EXT", ext)] {
        let path = format!("{directory}/ANLZ{index:04}.{extension}");
        storage::write_atomic(&safe_path(staging, &path)?, &analysis.to_bytes())?;
        files.push(path);
    }
    let device = DeviceTrack {
        track: backend,
        usb_path,
        anlz_dir: directory,
        anlz_index: index,
        file_size: output.metadata().map_err(|e| e.to_string())?.len(),
        sample_depth: 16,
        file_type: pdb::rows::FILE_TYPE_MP3,
        bitrate: 320,
        sample_rate: 44100,
    };
    Ok(PreparedSong {
        track,
        backend: device,
        files,
        staged_audio: output,
        artwork,
    })
}

#[derive(Default)]
struct PublishedFiles {
    files: Vec<std::path::PathBuf>,
    keep: bool,
}
impl Drop for PublishedFiles {
    fn drop(&mut self) {
        if !self.keep {
            // Only files created by this transfer are eligible for rollback cleanup.
            for file in &self.files {
                let _ = std::fs::remove_file(file);
            }
        }
    }
}

fn selected(library: &Library, playlists: &[u64]) -> Result<Library, String> {
    let mut snapshot = library.clone();
    for id in playlists {
        if library.playlist(*id).is_none() {
            return Err("Playlist no longer exists".into());
        }
    }
    for playlist in library.playlists() {
        if !playlists.contains(&playlist.id) {
            snapshot.delete_playlist(playlist.id)?;
        }
    }
    let needed: HashSet<u64> = snapshot
        .playlists()
        .iter()
        .flat_map(|p| p.track_ids.iter().copied())
        .collect();
    for track in library.tracks() {
        if !needed.contains(&track.id) {
            snapshot.remove_track(track.id);
        }
    }
    if snapshot.playlists().is_empty() {
        return Err("Choose a playlist before exporting".into());
    }
    Ok(snapshot)
}

fn filename(title: &str) -> String {
    let name: String = title
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(36)
        .collect();
    if name.is_empty() {
        "track".into()
    } else {
        name
    }
}

fn source_fingerprint(track: &openbox_domain::Track) -> Result<String, String> {
    let audio = storage::fingerprint(&track.path)?;
    let metadata = serde_json::to_vec(&(&track.title, &track.artist, &track.details))
        .map_err(|e| e.to_string())?;
    let artwork = track
        .details
        .artwork
        .as_ref()
        .map(|p| storage::fingerprint(p))
        .transpose()?
        .unwrap_or_default();
    Ok(format!(
        "{audio}:{}:{artwork}",
        storage::fingerprint_bytes(&metadata)
    ))
}

fn transcode(
    track: &openbox_domain::Track,
    output: &Path,
    progress: &mut dyn FnMut(f32),
) -> Result<(), String> {
    progress(0.0);
    let mut child = Command::new("ffmpeg")
        .args([
            "-nostdin",
            "-v",
            "error",
            "-progress",
            "pipe:1",
            "-nostats",
            "-i",
        ])
        .arg(&track.path)
        .args([
            "-map",
            "0:a:0",
            "-vn",
            "-map_metadata",
            "0",
            "-c:a",
            "libmp3lame",
            "-b:a",
            "320k",
            "-ar",
            "44100",
            "-ac",
            "2",
            "-write_xing",
            "0",
            "-metadata",
        ])
        .arg(format!("title={}", track.title))
        .arg("-metadata")
        .arg(format!("artist={}", track.artist))
        .arg(output)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not run ffmpeg: {e}"))?;
    let mut stderr = child.stderr.take().unwrap();
    // Drain diagnostics concurrently so a failed encoder cannot block on a full pipe.
    let diagnostics = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes);
        bytes
    });
    let duration = f64::from(track.details.duration_seconds).max(1.0) * 1_000_000.0;
    let mut last = 0.0_f32;
    for line in BufReader::new(child.stdout.take().unwrap())
        .lines()
        .map_while(Result::ok)
    {
        if let Some(time) = line
            .strip_prefix("out_time_us=")
            .and_then(|s| s.parse::<f64>().ok())
        {
            let fraction = (time / duration).clamp(0.0, 1.0) as f32;
            last = last.max(fraction);
            progress(last);
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let diagnostics = diagnostics.join().unwrap_or_default();
    if !status.success() {
        return Err(format!(
            "{} could not be exported: {}",
            track.title,
            String::from_utf8_lossy(&diagnostics).trim()
        ));
    }
    progress(1.0);
    Ok(())
}

fn backend_track(track: &openbox_domain::Track) -> collection::Track {
    collection::Track {
        id: track.id,
        name: track.title.clone(),
        artist: track.artist.clone(),
        album: track.details.album.clone(),
        genre: track.details.genre.clone(),
        composer: track.details.composer.clone(),
        comments: track.details.comment.clone(),
        year: track.details.year,
        track_number: track.details.track_number,
        disc_number: track.details.disc_number,
        tonality: track.details.key.clone(),
        average_bpm: track.details.bpm.unwrap_or(0.0),
        total_time: track.details.duration_seconds,
        location: track.path.to_string_lossy().into(),
        kind: "MP3 File".into(),
        bit_rate: 320,
        sample_rate: 44100,
        ..Default::default()
    }
}
