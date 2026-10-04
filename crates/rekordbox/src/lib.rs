mod analysis;
mod database;
mod transfer;
mod write;
use database::Database;
use openbox_domain::Library;
use openbox_storage as storage;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use transfer::Transfer;
use write::DeviceWrite;

const MANIFEST: &str = ".openbox/device.json";
const DATABASE: &str = "PIONEER/rekordbox/export.pdb";

#[derive(Debug, Serialize, Deserialize)]
struct DeviceCache {
    library: Library,
    #[serde(default)]
    database_fingerprint: String,
    #[serde(default)]
    sources: Vec<SourceLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SourceLink {
    path: PathBuf,
    source: String,
    audio: String,
    track_id: u64,
}

#[derive(Debug, Clone)]
pub struct DeviceLibrary {
    pub root: PathBuf,
    pub library: Library,
    database: Database,
    sources: Vec<SourceLink>,
}

impl DeviceLibrary {
    pub fn playlist_label(&self, id: u64) -> String {
        self.library.playlist_label(id)
    }
}

pub fn export(library: &Library, playlists: &[u64], root: &Path) -> Result<DeviceLibrary, String> {
    export_with_progress(library, playlists, root, |_| {})
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportPhase {
    Checking,
    Encoding,
    Waveforms,
    Publishing,
    Saving,
    Complete,
}

#[derive(Debug, Clone)]
pub struct ExportProgress {
    pub percent: f32,
    pub phase: ExportPhase,
    pub track: String,
    pub completed: usize,
    pub total: usize,
}

pub fn export_with_progress(
    library: &Library,
    playlists: &[u64],
    root: &Path,
    mut progress: impl FnMut(ExportProgress),
) -> Result<DeviceLibrary, String> {
    transfer::run(
        library,
        Transfer::Playlists(playlists),
        root,
        None,
        &mut progress,
    )
}

pub fn put_tracks(
    device: &DeviceLibrary,
    library: &Library,
    tracks: &[u64],
    playlist: u64,
    replace: Option<u64>,
) -> Result<DeviceLibrary, String> {
    put_tracks_with_progress(device, library, tracks, playlist, replace, |_| {})
}

pub fn put_tracks_with_progress(
    device: &DeviceLibrary,
    library: &Library,
    tracks: &[u64],
    playlist: u64,
    replace: Option<u64>,
    mut progress: impl FnMut(ExportProgress),
) -> Result<DeviceLibrary, String> {
    let target = device
        .library
        .playlist(playlist)
        .ok_or("USB playlist no longer exists")?;
    if replace.is_some_and(|id| !target.track_ids.contains(&id)) {
        return Err("The song to replace is no longer in this playlist".into());
    }
    if replace.is_some() && tracks.len() != 1 {
        return Err("Choose one replacement song".into());
    }
    transfer::run(
        library,
        Transfer::Tracks {
            tracks,
            playlist,
            replace,
        },
        &device.root,
        Some(&device.database.bytes),
        &mut progress,
    )
}

pub fn open(root: &Path) -> Result<DeviceLibrary, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let database = Database::read(
        std::fs::read(safe_path(&root, DATABASE)?)
            .map_err(|e| format!("Cannot read Rekordbox USB library: {e}"))?,
    )?;
    let library = database.library(&root)?;
    let sources = std::fs::read(safe_path(&root, MANIFEST)?)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<DeviceCache>(&bytes).ok())
        .filter(|cache| cache.database_fingerprint == storage::fingerprint_bytes(&database.bytes))
        .map(|cache| cache.sources)
        .unwrap_or_default();
    Ok(DeviceLibrary {
        root,
        library,
        database,
        sources,
    })
}

pub fn save_playlists(device: &DeviceLibrary) -> Result<(), String> {
    let _lock = DeviceWrite::acquire(&device.root)?;
    let pdb = device.database.with_playlists(&device.library)?;
    write::commit(
        &device.root,
        &pdb,
        Some(&device.database.bytes),
        &device.library,
        &device.sources,
    )
}

fn safe_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative = Path::new(relative.trim_start_matches('/'));
    let mut target = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err("Invalid device file path".into());
        };
        target.push(name);
        if target
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
        {
            return Err("Device directories must not contain symbolic links".into());
        }
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use binrw::BinRead;
    use rekordcrate::{
        anlz::ANLZ,
        pdb::{Header, PageType, Row},
    };
    use std::io::Cursor;

    #[test]
    fn export_reports_monotonic_progress_and_finishes_only_after_a_readable_commit() {
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        let (library, a, b) = collection(source.path());
        let mut updates = Vec::new();
        export_with_progress(&library, &[a, b], destination.path(), |progress| {
            updates.push(progress)
        })
        .unwrap();
        assert_eq!(updates.first().unwrap().percent, 0.0);
        assert_eq!(updates.last().unwrap().percent, 100.0);
        assert!(updates.windows(2).all(|p| p[0].percent <= p[1].percent));
        assert!(updates.iter().any(|p| p.percent > 0.0 && p.percent < 100.0));
        assert_eq!(
            open(destination.path()).unwrap().library.playlists().len(),
            2
        );
        let mut failed = library.clone();
        failed.import(
            source.path().join("missing.wav"),
            "Missing".into(),
            String::new(),
        );
        let id = failed.tracks().last().unwrap().id;
        failed.add_to_playlist(a, id).unwrap();
        updates.clear();
        assert!(
            export_with_progress(&failed, &[a], destination.path(), |progress| updates
                .push(progress))
            .is_err()
        );
        assert!(updates.iter().all(|p| p.percent < 100.0));
    }

    fn collection(folder: &Path) -> (Library, u64, u64) {
        let audio = folder.join("track.wav");
        openbox_test_support::audio(&audio);
        let mut library = Library::default();
        let track = library.import(audio, "Test track".into(), "Test artist".into());
        let a = library.create_playlist("Opening").unwrap();
        let b = library.create_playlist("Closing").unwrap();
        library.add_to_playlist(a, track).unwrap();
        library.add_to_playlist(b, track).unwrap();
        (library, a, b)
    }

    #[test]
    fn detected_metadata_and_beat_grid_are_exported_as_device_records() {
        let music = tempfile::tempdir().unwrap();
        let usb = tempfile::tempdir().unwrap();
        let path = music.path().join("rhythm.wav");
        openbox_test_support::rhythm(&path);
        let analysis = openbox_analysis::analyze(&path).unwrap();
        let mut library = Library::default();
        let id = library.import(path, "Rhythm".into(), "Artist".into());
        library.set_details(
            id,
            openbox_domain::TrackDetails {
                bpm: analysis.bpm,
                key: analysis.key,
                analysis: Some(analysis.grid),
                album: "Album".into(),
                genre: "House".into(),
                year: 2024,
                track_number: 7,
                composer: "Composer".into(),
                comment: "Keep me".into(),
                ..Default::default()
            },
        );
        let playlist = library.create_playlist("Set").unwrap();
        library.add_to_playlist(playlist, id).unwrap();
        let device = export(&library, &[playlist], usb.path()).unwrap();
        let song = &device.library.tracks()[0];
        assert!((song.details.bpm.unwrap() - 120.0).abs() < 1.0);
        assert_eq!(song.details.key, "C");
        assert_eq!(song.details.year, 2024);
        assert_eq!(song.details.composer, "Composer");
        assert_eq!(song.details.comment, "Keep me");
        assert!(song.details.analysis.as_ref().unwrap().beats.len() > 40);
        let analysis = analysis_files(usb.path())
            .into_iter()
            .find(|p| p.extension().unwrap() == "DAT")
            .unwrap();
        let dat = ANLZ::read(&mut Cursor::new(std::fs::read(analysis).unwrap())).unwrap();
        let grid = dat
            .sections
            .iter()
            .find_map(|s| {
                if let rekordcrate::anlz::Content::BeatGrid(grid) = &s.content {
                    Some(grid)
                } else {
                    None
                }
            })
            .unwrap();
        assert!(grid.beats.len() > 40);
        let metadata = openbox_metadata::read(&song.path, &music.path().join("artwork")).unwrap();
        assert_eq!(metadata.details.album, "Album");
        assert_eq!(metadata.details.genre, "House");
        assert_eq!(metadata.details.key, "C");
    }

    #[test]
    fn artwork_is_registered_embedded_and_reused_without_touching_existing_assets() {
        let music = tempfile::tempdir().unwrap();
        let usb = tempfile::tempdir().unwrap();
        openbox_test_support::rekordbox_usb(usb.path());
        let (mut library, playlist, _) = collection(music.path());
        let original_cover = usb.path().join("PIONEER/Artwork/original.jpg");
        std::fs::create_dir_all(original_cover.parent().unwrap()).unwrap();
        std::fs::write(&original_cover, b"original artwork sentinel").unwrap();
        let database_path = usb.path().join(DATABASE);
        let database = database::Database::read(std::fs::read(&database_path).unwrap()).unwrap();
        let mut original = database.rows(0).unwrap()[0].to_vec();
        original[28..32].copy_from_slice(&9009u32.to_le_bytes());
        let updated = database
            .update(&[
                (0, vec![original], false),
                (
                    13,
                    vec![baken_export::pdb::rows::named(
                        9009,
                        "/PIONEER/Artwork/original.jpg",
                    )],
                    false,
                ),
            ])
            .unwrap();
        std::fs::write(database_path, updated).unwrap();
        let source = music.path().join("cover.ppm");
        let mut ppm = b"P6\n2 2\n255\n".to_vec();
        ppm.extend([255, 40, 90].repeat(4));
        std::fs::write(&source, ppm).unwrap();
        let cover = music.path().join("cover.jpg");
        assert!(
            std::process::Command::new("ffmpeg")
                .args(["-v", "error", "-i"])
                .arg(source)
                .arg(&cover)
                .status()
                .unwrap()
                .success()
        );
        let id = library.tracks()[0].id;
        library.set_details(
            id,
            openbox_domain::TrackDetails {
                artwork: Some(cover),
                album_artist: "Album artist".into(),
                ..Default::default()
            },
        );
        let before = openbox_test_support::files(usb.path());
        let device = export(&library, &[playlist], usb.path()).unwrap();
        let track = device.library.tracks().iter().find(|t| t.id != 42).unwrap();
        assert!(track.details.artwork.as_ref().unwrap().is_file());
        assert!(
            track
                .details
                .artwork
                .as_ref()
                .unwrap()
                .starts_with(usb.path())
        );
        assert_eq!(track.details.album_artist, "Album artist");
        let cover = image::open(track.details.artwork.as_ref().unwrap()).unwrap();
        assert_eq!((cover.width(), cover.height()), (240, 240));
        assert_eq!(rows(usb.path(), PageType::Artwork).len(), 2);
        assert_eq!(
            device
                .library
                .track(42)
                .unwrap()
                .details
                .artwork
                .as_ref()
                .unwrap(),
            &original_cover
        );
        let tags = openbox_metadata::read(&track.path, &music.path().join("cache")).unwrap();
        assert_eq!(tags.details.album_artist, "Album artist");
        assert!(tags.details.artwork.is_some());
        export(&library, &[playlist], usb.path()).unwrap();
        assert_eq!(rows(usb.path(), PageType::Artwork).len(), 2);
        for (path, bytes) in before {
            let after = std::fs::read(usb.path().join(&path)).unwrap();
            if path == Path::new(DATABASE) {
                assert_eq!(&after[4096..bytes.len()], &bytes[4096..]);
            } else {
                assert_eq!(after, bytes);
            }
        }
    }

    #[test]
    fn exported_beat_timing_matches_an_impulse_in_the_transcoded_audio() {
        let music = tempfile::tempdir().unwrap();
        let usb = tempfile::tempdir().unwrap();
        let path = music.path().join("impulse.wav");
        assert!(
            std::process::Command::new("ffmpeg")
                .args([
                    "-v",
                    "error",
                    "-f",
                    "lavfi",
                    "-i",
                    "aevalsrc=if(eq(n\\,44100)\\,0.9\\,0):s=44100:d=3",
                    "-c:a",
                    "pcm_s16le"
                ])
                .arg(&path)
                .status()
                .unwrap()
                .success()
        );
        let mut library = Library::default();
        let id = library.import(path, "Impulse".into(), String::new());
        library.set_details(
            id,
            openbox_domain::TrackDetails {
                bpm: Some(120.0),
                analysis: Some(openbox_domain::TrackAnalysis {
                    version: "fixture".into(),
                    beats: vec![openbox_domain::Beat {
                        time_seconds: 1.0,
                        bpm: 120.0,
                        number: 1,
                    }],
                }),
                ..Default::default()
            },
        );
        let playlist = library.create_playlist("Impulse").unwrap();
        library.add_to_playlist(playlist, id).unwrap();
        let device = export(&library, &[playlist], usb.path()).unwrap();
        let track = &device.library.tracks()[0];
        let decoded = std::process::Command::new("ffmpeg")
            .args(["-v", "error", "-i"])
            .arg(&track.path)
            .args(["-ac", "1", "-ar", "44100", "-f", "f32le", "pipe:1"])
            .output()
            .unwrap();
        assert!(decoded.status.success());
        let peak = decoded
            .stdout
            .chunks_exact(4)
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                f32::from_le_bytes((*a).try_into().unwrap())
                    .abs()
                    .total_cmp(&f32::from_le_bytes((*b).try_into().unwrap()).abs())
            })
            .unwrap()
            .0 as f64
            / 44100.0;
        let beat = track.details.analysis.as_ref().unwrap().beats[0].time_seconds;
        assert!(
            (peak - beat).abs() < 0.001,
            "Audio peak {peak}, exported beat {beat}"
        );
    }

    #[test]
    fn exporting_a_legacy_track_analyzes_it_without_changing_the_local_library() {
        let music = tempfile::tempdir().unwrap();
        let usb = tempfile::tempdir().unwrap();
        let path = music.path().join("legacy.wav");
        openbox_test_support::rhythm(&path);
        let mut library = Library::default();
        let id = library.import(path, "Legacy".into(), String::new());
        let playlist = library.create_playlist("Legacy set").unwrap();
        library.add_to_playlist(playlist, id).unwrap();
        let device = export(&library, &[playlist], usb.path()).unwrap();
        let details = &device.library.tracks()[0].details;
        assert!((details.bpm.unwrap() - 120.0).abs() < 1.0);
        assert_eq!(details.key, "C");
        assert!(details.analysis.as_ref().unwrap().beats.len() > 40);
        assert!(library.track(id).unwrap().details.analysis.is_none());
        assert!(library.track(id).unwrap().details.bpm.is_none());
    }

    fn rows(root: &Path, kind: PageType) -> Vec<Row> {
        let bytes = std::fs::read(root.join("PIONEER/rekordbox/export.pdb")).unwrap();
        let mut reader = Cursor::new(bytes);
        let header = Header::read_le(&mut reader).unwrap();
        let table = header.tables.iter().find(|t| t.page_type == kind).unwrap();
        header
            .read_pages(
                &mut reader,
                binrw::Endian::Little,
                (&table.first_page, &table.last_page),
            )
            .unwrap()
            .into_iter()
            .flat_map(|p| {
                p.row_groups
                    .into_iter()
                    .flat_map(|g| g.present_rows().collect::<Vec<_>>())
            })
            .collect()
    }

    #[test]
    fn a_batch_of_library_tracks_transfers_to_an_existing_usb_playlist_without_replacing_its_music()
    {
        let music = tempfile::tempdir().unwrap();
        let usb = tempfile::tempdir().unwrap();
        openbox_test_support::rekordbox_usb(usb.path());
        let before = openbox_test_support::files(usb.path());
        let mut library = Library::default();
        let tracks: Vec<_> = (0..3)
            .map(|n| {
                let path = music.path().join(format!("{n}.wav"));
                openbox_test_support::audio(&path);
                library.import(path, format!("New {n}"), "Artist".into())
            })
            .collect();
        let device = put_tracks(&open(usb.path()).unwrap(), &library, &tracks, 23, None).unwrap();
        let entries = &device.library.playlist(23).unwrap().track_ids;
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0], 42);
        assert_eq!(
            entries[1..]
                .iter()
                .map(|id| device.library.track(*id).unwrap().title.as_str())
                .collect::<Vec<_>>(),
            ["New 0", "New 1", "New 2"]
        );
        let mut updated = device.clone();
        updated.library.remove_tracks(23, &entries[1..]).unwrap();
        save_playlists(&updated).unwrap();
        assert_eq!(
            open(usb.path())
                .unwrap()
                .library
                .playlist(23)
                .unwrap()
                .track_ids,
            [42]
        );
        assert_eq!(open(usb.path()).unwrap().library.tracks().len(), 4);
        for (path, bytes) in before {
            let after = std::fs::read(usb.path().join(&path)).unwrap();
            if path == Path::new(DATABASE) {
                assert_eq!(&after[4096..bytes.len()], &bytes[4096..]);
            } else {
                assert!(after == bytes);
            }
        }
    }

    #[test]
    fn nested_exports_merge_by_folder_path_and_keep_unrelated_usb_playlists() {
        let usb = tempfile::tempdir().unwrap();
        openbox_test_support::rekordbox_usb(usb.path());
        let mut library = Library::default();
        let sets = library.create_folder("Sets", None).unwrap();
        let house = library.create_folder("House", Some(sets)).unwrap();
        let opening = library.create_playlist_in("Opening", Some(house)).unwrap();
        let techno = library.create_folder("Techno", Some(sets)).unwrap();
        let other = library.create_playlist_in("Opening", Some(techno)).unwrap();
        let device = export(&library, &[opening, other], usb.path()).unwrap();
        assert_eq!(
            device
                .library
                .folders()
                .iter()
                .filter(|f| f.name == "Sets")
                .count(),
            1
        );
        assert_eq!(device.library.playlist_label(23), "Sets / Original set");
        let imported = device
            .library
            .playlists()
            .iter()
            .find(|p| device.library.playlist_label(p.id) == "Sets / House / Opening")
            .unwrap();
        let id = imported.id;
        let parent = imported.parent.unwrap();
        assert!(rows(usb.path(), PageType::PlaylistTree).iter().any(|row|
            matches!(row, Row::PlaylistTreeNode(p) if p.id.0 as u64 == id && p.parent_id.0 as u64 == parent)));
        let repeated = export(&library, &[opening], usb.path()).unwrap();
        assert_eq!(repeated.library.playlists().len(), 3);
        assert_eq!(repeated.library.playlist(id).unwrap().parent, Some(parent));
    }

    #[test]
    fn a_usb_playlist_cannot_be_shadowed_by_a_transferred_folder() {
        let usb = tempfile::tempdir().unwrap();
        openbox_test_support::rekordbox_usb(usb.path());
        let before = openbox_test_support::files(usb.path());
        let mut library = Library::default();
        let sets = library.create_folder("Sets", None).unwrap();
        let folder = library.create_folder("Original set", Some(sets)).unwrap();
        let playlist = library.create_playlist_in("Opening", Some(folder)).unwrap();
        assert!(export(&library, &[playlist], usb.path()).is_err());
        for (path, bytes) in before {
            assert!(std::fs::read(usb.path().join(path)).unwrap() == bytes);
        }
    }

    #[test]
    fn an_existing_rekordbox_library_opens_without_writing_to_the_device() {
        let usb = tempfile::tempdir().unwrap();
        openbox_test_support::rekordbox_usb(usb.path());
        let before = openbox_test_support::files(usb.path());
        let device = open(usb.path()).unwrap();
        assert_eq!(device.library.tracks()[0].id, 42);
        assert_eq!(device.library.tracks()[0].title, "Original track");
        assert_eq!(device.library.tracks()[0].artist, "Original artist");
        assert_eq!(device.library.playlists()[0].id, 23);
        assert_eq!(device.library.playlists()[0].track_ids, [42]);
        assert!(openbox_test_support::files(usb.path()) == before);
    }

    #[test]
    fn rekordbox_playlist_edits_preserve_original_pages_audio_analysis_and_folders() {
        let usb = tempfile::tempdir().unwrap();
        openbox_test_support::rekordbox_usb(usb.path());
        let before = openbox_test_support::files(usb.path());
        let mut device = open(usb.path()).unwrap();
        device.library.rename_playlist(23, "Renamed set").unwrap();
        let encore = device.library.create_playlist("Encore").unwrap();
        assert!(encore > 80);
        device.library.add_to_playlist(encore, 42).unwrap();
        save_playlists(&device).unwrap();
        let restored = open(usb.path()).unwrap();
        assert_eq!(restored.library.playlist(23).unwrap().name, "Renamed set");
        assert_eq!(restored.library.playlist(encore).unwrap().track_ids, [42]);
        for (path, bytes) in before {
            let after = std::fs::read(usb.path().join(&path)).unwrap();
            if path == Path::new(DATABASE) {
                assert_eq!(&after[4096..bytes.len()], &bytes[4096..]);
            } else {
                assert_eq!(after, bytes);
            }
        }
        let folders = rows(usb.path(), PageType::PlaylistTree);
        assert!(
            folders
                .iter()
                .any(|r| matches!(r, Row::PlaylistTreeNode(p) if p.id.0 == 80 && p.is_folder()))
        );
        assert!(
            folders.iter().any(
                |r| matches!(r, Row::PlaylistTreeNode(p) if p.id.0 == 23 && p.parent_id.0 == 80)
            )
        );
        let changed = restored.library.clone();
        let mut device = restored;
        device.library.delete_playlist(encore).unwrap();
        save_playlists(&device).unwrap();
        assert_eq!(
            open(usb.path()).unwrap().library.playlists().len(),
            changed.playlists().len() - 1
        );
    }

    #[test]
    fn saves_back_up_the_previous_database_and_reject_stale_device_views() {
        let usb = tempfile::tempdir().unwrap();
        openbox_test_support::rekordbox_usb(usb.path());
        let original = std::fs::read(usb.path().join(DATABASE)).unwrap();
        let stale = open(usb.path()).unwrap();
        let mut device = stale.clone();
        device.library.rename_playlist(23, "Updated").unwrap();
        save_playlists(&device).unwrap();
        let backups = usb.path().join(".openbox/backups");
        assert!(
            std::fs::read_dir(backups).unwrap().any(|p| {
                std::fs::read(p.unwrap().path().join("export.pdb")).unwrap() == original
            })
        );
        let after = openbox_test_support::files(usb.path());
        assert!(save_playlists(&stale).unwrap_err().contains("changed"));
        assert!(openbox_test_support::files(usb.path()) == after);
    }

    #[test]
    fn a_usb_writer_lock_is_released_when_its_file_handle_closes() {
        let usb = tempfile::tempdir().unwrap();
        openbox_test_support::rekordbox_usb(usb.path());
        let device = open(usb.path()).unwrap();
        std::fs::create_dir_all(usb.path().join(".openbox")).unwrap();
        let lock = std::fs::File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(usb.path().join(".openbox/write.lock"))
            .unwrap();
        lock.try_lock().unwrap();
        let original = std::fs::read(usb.path().join(DATABASE)).unwrap();
        assert!(save_playlists(&device).is_err());
        assert_eq!(std::fs::read(usb.path().join(DATABASE)).unwrap(), original);
        drop(lock);
        save_playlists(&device).unwrap();
    }

    #[test]
    fn exporting_into_a_rekordbox_usb_merges_playlists_without_replacing_existing_music() {
        let music = tempfile::tempdir().unwrap();
        let usb = tempfile::tempdir().unwrap();
        openbox_test_support::rekordbox_usb(usb.path());
        let before = openbox_test_support::files(usb.path());
        let (library, a, b) = collection(music.path());
        let device = export(&library, &[a, b], usb.path()).unwrap();
        assert_eq!(device.library.tracks().len(), 2);
        assert_eq!(device.library.playlists().len(), 3);
        assert_eq!(device.library.playlist(23).unwrap().track_ids, [42]);
        let a = device
            .library
            .playlists()
            .iter()
            .find(|p| p.name == "Opening")
            .unwrap();
        let b = device
            .library
            .playlists()
            .iter()
            .find(|p| p.name == "Closing")
            .unwrap();
        assert_eq!(a.track_ids, b.track_ids);
        assert_ne!(a.track_ids[0], 42);
        assert_eq!(
            device.library.track(a.track_ids[0]).unwrap().artist,
            "Test artist"
        );
        for (path, bytes) in before {
            let after = std::fs::read(usb.path().join(&path)).unwrap();
            if path == Path::new(DATABASE) {
                assert_eq!(&after[4096..bytes.len()], &bytes[4096..]);
            } else {
                assert_eq!(after, bytes);
            }
        }
    }

    #[test]
    fn a_library_song_can_replace_a_usb_playlist_entry_without_deleting_the_original_song() {
        let music = tempfile::tempdir().unwrap();
        let usb = tempfile::tempdir().unwrap();
        openbox_test_support::rekordbox_usb(usb.path());
        let (library, _, _) = collection(music.path());
        let original = std::fs::read(usb.path().join("Contents/Original/track.wav")).unwrap();
        let device = open(usb.path()).unwrap();
        let updated = put_tracks(&device, &library, &[1], 23, Some(42)).unwrap();
        let entries = &updated.library.playlist(23).unwrap().track_ids;
        assert_eq!(entries.len(), 1);
        assert_ne!(entries[0], 42);
        assert!(updated.library.track(42).unwrap().path.is_file());
        assert_eq!(
            std::fs::read(updated.library.track(42).unwrap().path.clone()).unwrap(),
            original
        );
        let new_track = entries[0];
        let added = put_tracks(&updated, &library, &[1], 23, None).unwrap();
        assert_eq!(added.library.playlist(23).unwrap().track_ids[0], new_track);
        assert_eq!(added.library.playlist(23).unwrap().track_ids.len(), 1);
    }

    #[test]
    fn sending_the_same_playlist_again_reuses_audio_and_updates_only_that_playlist() {
        let music = tempfile::tempdir().unwrap();
        let usb = tempfile::tempdir().unwrap();
        openbox_test_support::rekordbox_usb(usb.path());
        let (mut library, a, b) = collection(music.path());
        export(&library, &[a, b], usb.path()).unwrap();
        let added = music.path().join("second.wav");
        openbox_test_support::audio(&added);
        let song = library.import(added, "Another track".into(), "Another artist".into());
        library.add_to_playlist(a, song).unwrap();
        let updated = export(&library, &[a], usb.path()).unwrap();
        assert_eq!(updated.library.tracks().len(), 3);
        assert_eq!(updated.library.playlists().len(), 3);
        let opening = updated
            .library
            .playlists()
            .iter()
            .find(|p| p.name == "Opening")
            .unwrap();
        let closing = updated
            .library
            .playlists()
            .iter()
            .find(|p| p.name == "Closing")
            .unwrap();
        assert_eq!(opening.track_ids.len(), 2);
        assert_eq!(closing.track_ids, [opening.track_ids[0]]);
        assert_eq!(updated.library.playlist(23).unwrap().track_ids, [42]);
    }

    #[test]
    fn a_failed_transfer_does_not_leave_new_audio_or_analysis_on_the_usb() {
        let music = tempfile::tempdir().unwrap();
        let usb = tempfile::tempdir().unwrap();
        openbox_test_support::rekordbox_usb(usb.path());
        std::fs::create_dir_all(usb.path().join(MANIFEST)).unwrap();
        let before = openbox_test_support::files(usb.path());
        let (library, a, _) = collection(music.path());
        assert!(export(&library, &[a], usb.path()).is_err());
        let mut after = openbox_test_support::files(usb.path());
        after.remove(Path::new(".openbox/write.lock"));
        assert!(after == before);
    }

    #[test]
    fn a_real_rekordbox_fixture_can_be_opened_and_edited_without_reencoding_its_track_records() {
        let usb = tempfile::tempdir().unwrap();
        let database = usb.path().join(DATABASE);
        let bytes = include_bytes!("../../../tests/fixtures/rekordbox-demo.pdb");
        std::fs::create_dir_all(database.parent().unwrap()).unwrap();
        std::fs::write(&database, bytes).unwrap();
        let mut device = open(usb.path()).unwrap();
        assert_eq!(device.library.tracks().len(), 2);
        let playlist = device.library.create_playlist("Fixture set").unwrap();
        let track = device.library.tracks()[0].id;
        device.library.add_to_playlist(playlist, track).unwrap();
        save_playlists(&device).unwrap();
        assert_eq!(
            open(usb.path())
                .unwrap()
                .library
                .playlist(playlist)
                .unwrap()
                .track_ids,
            [track]
        );
        assert_eq!(
            &std::fs::read(&database).unwrap()[4096..bytes.len()],
            &bytes[4096..]
        );
        assert_eq!(rows(usb.path(), PageType::Tracks).len(), 2);
    }

    #[test]
    fn usb_export_writes_shared_audio_and_independently_parseable_device_files() {
        let music = tempfile::tempdir().unwrap();
        let usb = tempfile::tempdir().unwrap();
        let (library, a, b) = collection(music.path());
        let device = export(&library, &[a, b], usb.path()).unwrap();

        assert_eq!(device.library.tracks().len(), 1);
        assert_eq!(device.library.playlists().len(), 2);
        let properties = device.database.rows(19).unwrap();
        let count = u32::from_le_bytes(properties[0][4..8].try_into().unwrap());
        assert_eq!(
            count, 1,
            "The device song count must include exported tracks"
        );
        assert_eq!(rows(usb.path(), PageType::Tracks).len(), 1);
        assert_eq!(rows(usb.path(), PageType::PlaylistTree).len(), 2);
        assert_eq!(rows(usb.path(), PageType::PlaylistEntries).len(), 2);
        let song = &device.library.tracks()[0];
        assert!(song.path.is_file());
        assert_eq!(song.path.extension().unwrap(), "mp3");
        let files = analysis_files(usb.path());
        assert_eq!(files.len(), 2);
        for path in files {
            let mut file = std::fs::File::open(path).unwrap();
            let analysis = ANLZ::read_be(&mut file).unwrap();
            assert!(!analysis.sections.is_empty());
        }
        assert_eq!(
            open(usb.path()).unwrap().library.playlists(),
            device.library.playlists()
        );
    }

    #[test]
    fn editing_usb_playlists_updates_the_device_database_without_touching_audio() {
        let music = tempfile::tempdir().unwrap();
        let usb = tempfile::tempdir().unwrap();
        let (library, a, b) = collection(music.path());
        let mut device = export(&library, &[a, b], usb.path()).unwrap();
        let song = &device.library.tracks()[0];
        let track_id = song.id;
        let audio_path = song.path.clone();
        let original = std::fs::read(&audio_path).unwrap();
        device.library.rename_playlist(a, "New opening").unwrap();
        device.library.delete_playlist(b).unwrap();
        let c = device.library.create_playlist("Encore").unwrap();
        device.library.add_to_playlist(c, track_id).unwrap();
        save_playlists(&device).unwrap();

        let restored = open(usb.path()).unwrap();
        assert_eq!(restored.library.playlists(), device.library.playlists());
        assert_eq!(std::fs::read(audio_path).unwrap(), original);
        let mut names: Vec<String> = rows(usb.path(), PageType::PlaylistTree)
            .into_iter()
            .filter_map(|r| {
                if let Row::PlaylistTreeNode(p) = r {
                    Some(p.name.into_string().unwrap())
                } else {
                    None
                }
            })
            .collect();
        names.sort();
        assert_eq!(names, ["Encore", "New opening"]);
    }

    #[test]
    fn a_damaged_rekordbox_database_is_rejected_without_replacing_it() {
        let music = tempfile::tempdir().unwrap();
        let usb = tempfile::tempdir().unwrap();
        let (library, a, _) = collection(music.path());
        let pioneer = usb.path().join("PIONEER/rekordbox");
        std::fs::create_dir_all(&pioneer).unwrap();
        let database = pioneer.join("export.pdb");
        std::fs::write(&database, "existing rekordbox library").unwrap();
        assert!(export(&library, &[a], usb.path()).is_err());
        assert_eq!(
            std::fs::read_to_string(database).unwrap(),
            "existing rekordbox library"
        );
    }

    #[test]
    fn cyclic_page_chains_are_rejected_without_writing_anything() {
        let usb = tempfile::tempdir().unwrap();
        openbox_test_support::rekordbox_usb(usb.path());
        let mut bytes = std::fs::read(usb.path().join(DATABASE)).unwrap();
        let first = u32::from_le_bytes(bytes[36..40].try_into().unwrap()) as usize;
        bytes[first * 4096 + 12..first * 4096 + 16].copy_from_slice(&(first as u32).to_le_bytes());
        std::fs::write(usb.path().join(DATABASE), bytes).unwrap();
        let before = openbox_test_support::files(usb.path());
        assert!(open(usb.path()).unwrap_err().contains("cyclic"));
        assert!(openbox_test_support::files(usb.path()) == before);
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_links_cannot_redirect_device_music_access_outside_the_usb() {
        let usb = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        openbox_test_support::rekordbox_usb(usb.path());
        std::fs::rename(
            usb.path().join("Contents"),
            external.path().join("Contents"),
        )
        .unwrap();
        std::os::unix::fs::symlink(
            external.path().join("Contents"),
            usb.path().join("Contents"),
        )
        .unwrap();
        let original = openbox_test_support::files(external.path());
        assert!(open(usb.path()).unwrap_err().contains("symbolic"));
        assert!(openbox_test_support::files(external.path()) == original);
    }

    #[test]
    fn a_failed_reexport_preserves_the_existing_usb_library() {
        let music = tempfile::tempdir().unwrap();
        let usb = tempfile::tempdir().unwrap();
        let (library, a, _) = collection(music.path());
        export(&library, &[a], usb.path()).unwrap();
        let before = std::fs::read(usb.path().join(DATABASE)).unwrap();
        std::fs::remove_file(&library.tracks()[0].path).unwrap();
        assert!(export(&library, &[a], usb.path()).is_err());
        assert_eq!(std::fs::read(usb.path().join(DATABASE)).unwrap(), before);
        assert_eq!(open(usb.path()).unwrap().library.playlists().len(), 1);
    }

    #[test]
    fn a_usb_can_be_reopened_under_a_different_mount_path() {
        let music = tempfile::tempdir().unwrap();
        let parent = tempfile::tempdir().unwrap();
        let first = parent.path().join("first");
        let second = parent.path().join("second");
        std::fs::create_dir(&first).unwrap();
        let (library, a, _) = collection(music.path());
        export(&library, &[a], &first).unwrap();
        std::fs::rename(first, &second).unwrap();
        let reopened = open(&second).unwrap();
        assert!(reopened.library.tracks()[0].path.starts_with(&second));
        assert!(reopened.library.tracks()[0].path.is_file());
        assert_eq!(reopened.library.playlists()[0].name, "Opening");
    }

    #[test]
    fn a_playlist_save_failure_keeps_the_previous_device_database() {
        let music = tempfile::tempdir().unwrap();
        let usb = tempfile::tempdir().unwrap();
        let (library, a, _) = collection(music.path());
        let mut device = export(&library, &[a], usb.path()).unwrap();
        let before = std::fs::read(usb.path().join(DATABASE)).unwrap();
        device.library.rename_playlist(a, "Changed").unwrap();
        let manifest = usb.path().join(MANIFEST);
        std::fs::remove_file(&manifest).unwrap();
        std::fs::create_dir(&manifest).unwrap();
        assert!(save_playlists(&device).is_err());
        assert_eq!(std::fs::read(usb.path().join(DATABASE)).unwrap(), before);
    }

    fn analysis_files(root: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for p in std::fs::read_dir(root.join("PIONEER/USBANLZ")).unwrap() {
            for hash in std::fs::read_dir(p.unwrap().path()).unwrap() {
                for file in std::fs::read_dir(hash.unwrap().path()).unwrap() {
                    let path = file.unwrap().path();
                    if matches!(
                        path.extension().and_then(|e| e.to_str()),
                        Some("DAT" | "EXT")
                    ) {
                        files.push(path);
                    }
                }
            }
        }
        files
    }
}
