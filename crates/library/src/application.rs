use crate::import::ImportReport;
use openbox_domain::Library;
use openbox_rekordbox::{self as usb, DeviceLibrary};
use openbox_storage as storage;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Library,
    Usb,
}

pub enum Edit {
    Create(String),
    CreateIn(String, Option<u64>),
    CreateWithTracks(String, Option<u64>, Vec<u64>),
    CreateFolder(String, Option<u64>),
    Rename(u64, String),
    Delete(u64),
    Add(u64, u64),
    AddMany(u64, Vec<u64>),
    RemoveMany(u64, Vec<u64>),
    Remove(u64, u64),
    Move(u64, u64, usize),
    MoveMany(u64, Vec<u64>, i32),
    RemoveTrack(u64),
    RemoveTracks(Vec<u64>),
}

#[derive(Clone)]
pub struct Workspace {
    pub library: Library,
    pub device: Option<DeviceLibrary>,
    library_path: PathBuf,
}

impl Workspace {
    pub fn artwork_directory(&self) -> PathBuf {
        self.library_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("artwork")
    }
    pub fn undo_usb_playlists(&mut self, previous: &Library) -> Result<(), String> {
        let mut device = self.device.clone().ok_or("Open a USB first")?;
        device.library.restore_playlists(previous);
        usb::save_playlists(&device)?;
        self.device = Some(usb::open(&device.root)?);
        Ok(())
    }
    pub fn load(path: PathBuf) -> Result<Self, String> {
        Ok(Self {
            library: storage::load(&path)?,
            device: None,
            library_path: path,
        })
    }

    pub fn collection(&self, target: Target) -> Option<&Library> {
        match target {
            Target::Library => Some(&self.library),
            Target::Usb => self.device.as_ref().map(|d| &d.library),
        }
    }

    pub fn edit(&mut self, target: Target, edit: Edit) -> Result<(), String> {
        let mut updated = self
            .collection(target)
            .ok_or("Open a USB library first")?
            .clone();
        edit.apply(&mut updated)?;
        match target {
            Target::Library => {
                storage::save(&self.library_path, &updated)?;
                self.library = updated;
            }
            Target::Usb => {
                let mut device = self.device.clone().ok_or("Open a USB library first")?;
                device.library = updated;
                usb::save_playlists(&device)?;
                self.device = Some(usb::open(&device.root)?);
            }
        }
        Ok(())
    }

    pub fn import(&mut self, report: ImportReport) -> Result<usize, String> {
        let mut updated = self.library.clone();
        let before = updated.tracks().len();
        for track in report.tracks {
            let id = updated.import(track.path, track.title, track.artist);
            updated.set_details(id, track.details);
        }
        storage::save(&self.library_path, &updated)?;
        let added = updated.tracks().len() - before;
        self.library = updated;
        Ok(added)
    }

    pub fn import_usb_playlist(&mut self, id: u64) -> Result<u64, String> {
        let device = self.device.as_ref().ok_or("Open a USB first")?;
        let playlist = device
            .library
            .playlist(id)
            .ok_or("USB playlist no longer exists")?;
        let imports = self
            .library_path
            .parent()
            .ok_or("Library has no parent directory")?
            .join("imports");
        std::fs::create_dir_all(&imports).map_err(|e| e.to_string())?;
        let assets = tempfile::Builder::new()
            .prefix("usb-")
            .tempdir_in(&imports)
            .map_err(|e| e.to_string())?;
        let mut updated = self.library.clone();
        let mut parent = None;
        for folder in device.library.ancestors(playlist.parent) {
            parent = Some(
                match updated
                    .folders()
                    .iter()
                    .find(|f| f.parent == parent && f.name.eq_ignore_ascii_case(&folder.name))
                {
                    Some(existing) => existing.id,
                    None => updated.create_folder(&folder.name, parent)?,
                },
            );
        }
        let mut name = playlist.name.clone();
        let mut suffix = 1;
        while updated
            .children(parent)
            .iter()
            .any(|n| n.name().eq_ignore_ascii_case(&name))
        {
            name = format!("{} (USB {suffix})", playlist.name);
            suffix += 1;
        }
        let imported = updated.create_playlist_in(&name, parent)?;
        for id in &playlist.track_ids {
            let track = device
                .library
                .track(*id)
                .ok_or("USB song no longer exists")?;
            let extension = track
                .path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("audio");
            let destination = assets.path().join(format!("{id}.{extension}"));
            let mut input = std::fs::File::open(&track.path)
                .map_err(|e| format!("Cannot import {}: {e}", track.title))?;
            let mut output = std::fs::File::create(&destination).map_err(|e| e.to_string())?;
            std::io::copy(&mut input, &mut output).map_err(|e| e.to_string())?;
            output.sync_all().map_err(|e| e.to_string())?;
            let local_id = updated.import(destination, track.title.clone(), track.artist.clone());
            let mut details = track.details.clone();
            if let Some(artwork) = &details.artwork {
                let destination = assets.path().join(format!("{id}.jpg"));
                std::fs::copy(artwork, &destination).map_err(|e| e.to_string())?;
                std::fs::File::open(&destination)
                    .and_then(|f| f.sync_all())
                    .map_err(|e| e.to_string())?;
                details.artwork = Some(destination);
            }
            updated.set_details(local_id, details);
            updated.add_to_playlist(imported, local_id)?;
        }
        storage::save(&self.library_path, &updated)?;
        let _ = assets.keep();
        self.library = updated;
        Ok(imported)
    }
}

impl Edit {
    fn apply(self, library: &mut Library) -> Result<(), String> {
        match self {
            Self::Create(name) => library.create_playlist(&name).map(|_| ()),
            Self::CreateIn(name, parent) => library.create_playlist_in(&name, parent).map(|_| ()),
            Self::CreateWithTracks(name, parent, tracks) => {
                let id = library.create_playlist_in(&name, parent)?;
                library.add_tracks(id, &tracks)
            }
            Self::CreateFolder(name, parent) => library.create_folder(&name, parent).map(|_| ()),
            Self::Rename(id, name) => library.rename_playlist(id, &name),
            Self::Delete(id) => library.delete_playlist(id),
            Self::AddMany(playlist, tracks) => library.add_tracks(playlist, &tracks),
            Self::RemoveMany(playlist, tracks) => library.remove_tracks(playlist, &tracks),
            Self::Add(playlist, track) => library.add_to_playlist(playlist, track),
            Self::Remove(playlist, track) => library.remove_from_playlist(playlist, track),
            Self::MoveMany(playlist, tracks, direction) => {
                library.move_tracks_by(playlist, &tracks, direction)
            }
            Self::Move(playlist, track, position) => library.move_track(playlist, track, position),
            Self::RemoveTracks(tracks) => {
                for id in tracks {
                    library.remove_track(id);
                }
                Ok(())
            }
            Self::RemoveTrack(track) => {
                library.remove_track(track);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creating_a_playlist_from_selected_tracks_publishes_one_complete_saved_playlist() {
        let local = tempfile::tempdir().unwrap();
        let path = local.path().join("library.json");
        let mut workspace = Workspace::load(path.clone()).unwrap();
        let tracks: Vec<_> = (0..3)
            .map(|n| {
                workspace
                    .library
                    .import(format!("{n}.wav").into(), n.to_string(), String::new())
            })
            .collect();
        workspace
            .edit(
                Target::Library,
                Edit::CreateWithTracks("Set".into(), None, vec![tracks[2], tracks[0]]),
            )
            .unwrap();
        let saved = Workspace::load(path.clone()).unwrap();
        assert_eq!(
            saved.library.playlists()[0].track_ids,
            [tracks[2], tracks[0]]
        );
        let before = std::fs::read(&path).unwrap();
        assert!(
            workspace
                .edit(
                    Target::Library,
                    Edit::CreateWithTracks("Invalid".into(), None, vec![999])
                )
                .is_err()
        );
        assert_eq!(std::fs::read(path).unwrap(), before);
        assert_eq!(workspace.library.playlists().len(), 1);
    }

    #[test]
    fn hundreds_of_selected_tracks_are_added_once_in_order_and_saved_atomically() {
        let local = tempfile::tempdir().unwrap();
        let path = local.path().join("library.json");
        let mut workspace = Workspace::load(path.clone()).unwrap();
        let playlist = workspace.library.create_playlist("Set").unwrap();
        let tracks: Vec<_> = (0..500)
            .map(|n| {
                workspace.library.import(
                    PathBuf::from(format!("/music/{n}.mp3")),
                    format!("Track {n}"),
                    "Artist".into(),
                )
            })
            .collect();
        workspace
            .edit(Target::Library, Edit::AddMany(playlist, tracks.clone()))
            .unwrap();
        workspace
            .edit(Target::Library, Edit::AddMany(playlist, tracks.clone()))
            .unwrap();
        assert_eq!(
            Workspace::load(path.clone())
                .unwrap()
                .library
                .playlist(playlist)
                .unwrap()
                .track_ids,
            tracks
        );
        let before = std::fs::read(&path).unwrap();
        assert!(
            workspace
                .edit(
                    Target::Library,
                    Edit::AddMany(playlist, vec![tracks[0], 9999])
                )
                .is_err()
        );
        assert_eq!(std::fs::read(path).unwrap(), before);
        workspace
            .edit(
                Target::Library,
                Edit::RemoveMany(playlist, tracks[..300].to_vec()),
            )
            .unwrap();
        assert_eq!(
            workspace.library.playlist(playlist).unwrap().track_ids,
            tracks[300..]
        );
        assert_eq!(workspace.library.tracks().len(), 500);
    }

    #[test]
    fn nested_usb_creation_and_import_keep_the_folder_path_after_reopening() {
        let local = tempfile::tempdir().unwrap();
        let usb = tempfile::tempdir().unwrap();
        openbox_test_support::rekordbox_usb(usb.path());
        let mut workspace = Workspace::load(local.path().join("library.json")).unwrap();
        workspace.device = Some(usb::open(usb.path()).unwrap());
        workspace
            .edit(Target::Usb, Edit::CreateFolder("House".into(), Some(80)))
            .unwrap();
        let house = workspace
            .device
            .as_ref()
            .unwrap()
            .library
            .folders()
            .last()
            .unwrap()
            .id;
        workspace
            .edit(Target::Usb, Edit::CreateIn("Opening".into(), Some(house)))
            .unwrap();
        let set = workspace
            .device
            .as_ref()
            .unwrap()
            .library
            .playlists()
            .last()
            .unwrap()
            .id;
        workspace.edit(Target::Usb, Edit::Add(set, 42)).unwrap();
        let reopened = usb::open(usb.path()).unwrap();
        assert_eq!(
            reopened.library.playlist_label(set),
            "Sets / House / Opening"
        );
        workspace
            .edit(Target::Library, Edit::CreateFolder("SETS".into(), None))
            .unwrap();
        let imported = workspace.import_usb_playlist(set).unwrap();
        let saved = Workspace::load(local.path().join("library.json")).unwrap();
        assert_eq!(
            saved.library.playlist_label(imported),
            "SETS / House / Opening"
        );
        let second = workspace.import_usb_playlist(set).unwrap();
        assert_eq!(
            workspace.library.playlist_label(second),
            "SETS / House / Opening (USB 1)"
        );
    }

    #[test]
    fn importing_a_usb_playlist_copies_audio_so_the_library_works_after_unplugging() {
        let local = tempfile::tempdir().unwrap();
        let usb = tempfile::tempdir().unwrap();
        openbox_test_support::rekordbox_usb(usb.path());
        let before = openbox_test_support::files(usb.path());
        let mut workspace = Workspace::load(local.path().join("library.json")).unwrap();
        workspace.device = Some(usb::open(usb.path()).unwrap());
        let imported = workspace.import_usb_playlist(23).unwrap();
        assert_eq!(
            workspace.library.playlist(imported).unwrap().name,
            "Original set"
        );
        let song = &workspace.library.tracks()[0];
        assert!(song.path.starts_with(local.path()));
        assert_eq!(
            std::fs::read(&song.path).unwrap(),
            before[&PathBuf::from("Contents/Original/track.wav")]
        );
        assert!(openbox_test_support::files(usb.path()) == before);
        let saved = Workspace::load(local.path().join("library.json")).unwrap();
        assert_eq!(saved.library.playlists(), workspace.library.playlists());
        usb.close().unwrap();
        assert!(saved.library.tracks()[0].path.is_file());
    }

    #[test]
    fn removing_a_usb_playlist_can_be_undone_without_deleting_its_audio() {
        let local = tempfile::tempdir().unwrap();
        let usb = tempfile::tempdir().unwrap();
        openbox_test_support::rekordbox_usb(usb.path());
        let mut workspace = Workspace::load(local.path().join("library.json")).unwrap();
        workspace.device = Some(usb::open(usb.path()).unwrap());
        let before = workspace.device.as_ref().unwrap().library.clone();
        workspace.edit(Target::Usb, Edit::Delete(23)).unwrap();
        assert!(
            workspace
                .device
                .as_ref()
                .unwrap()
                .library
                .playlists()
                .is_empty()
        );
        assert!(before.tracks()[0].path.is_file());
        workspace.undo_usb_playlists(&before).unwrap();
        assert_eq!(
            workspace.device.as_ref().unwrap().library.playlists(),
            before.playlists()
        );
        assert_eq!(
            usb::open(usb.path())
                .unwrap()
                .library
                .playlist(23)
                .unwrap()
                .track_ids,
            [42]
        );
    }

    #[test]
    fn a_missing_usb_song_does_not_partially_import_a_playlist() {
        let local = tempfile::tempdir().unwrap();
        let usb = tempfile::tempdir().unwrap();
        openbox_test_support::rekordbox_usb(usb.path());
        let path = local.path().join("library.json");
        let mut workspace = Workspace::load(path.clone()).unwrap();
        workspace.device = Some(usb::open(usb.path()).unwrap());
        std::fs::remove_file(&workspace.device.as_ref().unwrap().library.tracks()[0].path).unwrap();
        assert!(workspace.import_usb_playlist(23).is_err());
        assert!(workspace.library.playlists().is_empty());
        assert!(!path.exists());
        assert!(openbox_test_support::files(local.path()).is_empty());
    }

    #[test]
    fn library_changes_are_saved_before_the_workspace_reports_success() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.json");
        let mut workspace = Workspace::load(path.clone()).unwrap();
        workspace
            .edit(Target::Library, Edit::Create("Opening".into()))
            .unwrap();
        let saved = Workspace::load(path).unwrap();
        assert_eq!(saved.library.playlists()[0].name, "Opening");
        assert_eq!(workspace.library.playlists(), saved.library.playlists());
    }

    #[test]
    fn imported_tracks_retain_metadata_and_reimporting_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let audio = dir.path().join("Artist - Song.wav");
        openbox_test_support::audio(&audio);
        let report = crate::import::scan(&[audio], &dir.path().join("artwork")).unwrap();
        let mut workspace = Workspace::load(dir.path().join("library.json")).unwrap();
        assert_eq!(workspace.import(report.clone()).unwrap(), 1);
        assert_eq!(workspace.import(report).unwrap(), 0);
        assert_eq!(workspace.library.tracks()[0].details.duration_seconds, 2);
        assert_eq!(workspace.library.tracks()[0].title, "Song");
    }

    #[test]
    fn rescanning_an_existing_track_refreshes_its_title_and_artist() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("music.mp3");
        let mut workspace = Workspace::load(dir.path().join("library.json")).unwrap();
        let id = workspace
            .library
            .import(path.clone(), "Old title".into(), "Old artist".into());
        let report = ImportReport {
            tracks: vec![openbox_domain::Track {
                id: 0,
                path,
                title: "Tagged title".into(),
                artist: "Tagged artist".into(),
                details: Default::default(),
            }],
            skipped: Vec::new(),
        };
        assert_eq!(workspace.import(report).unwrap(), 0);
        assert_eq!(workspace.library.track(id).unwrap().title, "Tagged title");
        assert_eq!(workspace.library.track(id).unwrap().artist, "Tagged artist");
        assert_eq!(
            Workspace::load(dir.path().join("library.json"))
                .unwrap()
                .library
                .track(id)
                .unwrap()
                .title,
            "Tagged title"
        );
    }

    #[test]
    fn reanalysis_keeps_playlist_membership_and_usb_artwork_is_local_after_import() {
        let dir = tempfile::tempdir().unwrap();
        let usb = tempfile::tempdir().unwrap();
        let path = dir.path().join("music.wav");
        openbox_test_support::rhythm(&path);
        let mut workspace = Workspace::load(dir.path().join("library.json")).unwrap();
        let id = workspace
            .library
            .import(path.clone(), "Before".into(), String::new());
        let playlist = workspace.library.create_playlist("Set").unwrap();
        workspace.library.add_to_playlist(playlist, id).unwrap();
        let report = crate::import::scan(&[path], &workspace.artwork_directory()).unwrap();
        assert_eq!(workspace.import(report).unwrap(), 0);
        assert_eq!(
            workspace.library.playlist(playlist).unwrap().track_ids,
            [id]
        );
        let track = workspace.library.track(id).unwrap();
        assert!(track.details.analysis.as_ref().unwrap().beats.len() > 40);
        let ppm = dir.path().join("cover.ppm");
        let mut bytes = b"P6\n2 2\n255\n".to_vec();
        bytes.extend([10, 120, 220].repeat(4));
        std::fs::write(&ppm, bytes).unwrap();
        let cover = dir.path().join("cover.jpg");
        assert!(
            std::process::Command::new("ffmpeg")
                .args(["-v", "error", "-i"])
                .arg(ppm)
                .arg(&cover)
                .status()
                .unwrap()
                .success()
        );
        let mut details = track.details.clone();
        details.artwork = Some(cover);
        workspace.library.set_details(id, details);
        workspace.device = Some(usb::export(&workspace.library, &[playlist], usb.path()).unwrap());
        let imported = workspace.import_usb_playlist(playlist).unwrap();
        let local = Workspace::load(dir.path().join("library.json")).unwrap();
        let track = local
            .library
            .track(local.library.playlist(imported).unwrap().track_ids[0])
            .unwrap();
        assert!(
            track
                .details
                .artwork
                .as_ref()
                .unwrap()
                .starts_with(dir.path())
        );
        assert!(
            !track
                .details
                .artwork
                .as_ref()
                .unwrap()
                .starts_with(usb.path())
        );
        usb.close().unwrap();
        assert!(track.path.is_file());
        assert!(track.details.artwork.as_ref().unwrap().is_file());
        assert!(track.details.analysis.as_ref().unwrap().beats.len() > 40);
    }

    #[test]
    fn a_failed_save_does_not_change_the_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let blocked = dir.path().join("blocked");
        let mut workspace = Workspace::load(blocked.join("library.json")).unwrap();
        std::fs::write(blocked, "not a directory").unwrap();
        assert!(
            workspace
                .edit(Target::Library, Edit::Create("Opening".into()))
                .is_err()
        );
        assert!(workspace.library.playlists().is_empty());
    }
}
