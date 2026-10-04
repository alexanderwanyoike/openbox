use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: u64,
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    #[serde(default)]
    pub details: TrackDetails,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrackDetails {
    pub album: String,
    pub genre: String,
    pub key: String,
    pub bpm: Option<f64>,
    pub duration_seconds: u32,
    pub album_artist: String,
    pub composer: String,
    pub comment: String,
    pub year: u32,
    pub track_number: u32,
    pub disc_number: u32,
    pub artwork: Option<PathBuf>,
    pub analysis: Option<TrackAnalysis>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TrackAnalysis {
    pub version: String,
    pub beats: Vec<Beat>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Beat {
    pub time_seconds: f64,
    pub bpm: f64,
    pub number: u16,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Library {
    tracks: Vec<Track>,
    playlists: Vec<Playlist>,
    #[serde(default)]
    folders: Vec<Folder>,
    #[serde(default)]
    playlist_id_floor: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Playlist {
    pub id: u64,
    pub name: String,
    pub track_ids: Vec<u64>,
    #[serde(default)]
    pub parent: Option<u64>,
    #[serde(default)]
    pub sort_order: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Folder {
    pub id: u64,
    pub name: String,
    pub parent: Option<u64>,
    pub sort_order: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum PlaylistNode<'a> {
    Folder(&'a Folder),
    Playlist(&'a Playlist),
}

impl<'a> PlaylistNode<'a> {
    pub fn id(self) -> u64 {
        match self {
            Self::Folder(f) => f.id,
            Self::Playlist(p) => p.id,
        }
    }

    pub fn name(self) -> &'a str {
        match self {
            Self::Folder(f) => &f.name,
            Self::Playlist(p) => &p.name,
        }
    }

    pub fn parent(self) -> Option<u64> {
        match self {
            Self::Folder(f) => f.parent,
            Self::Playlist(p) => p.parent,
        }
    }

    pub fn sort_order(self) -> u32 {
        match self {
            Self::Folder(f) => f.sort_order,
            Self::Playlist(p) => p.sort_order,
        }
    }
}

impl Library {
    pub fn restore_playlists(&mut self, previous: &Library) {
        self.playlists = previous.playlists.clone();
        self.folders = previous.folders.clone();
        self.playlist_id_floor = self.playlist_id_floor.max(previous.playlist_id_floor);
    }
    pub fn from_parts(
        tracks: Vec<Track>,
        playlists: Vec<Playlist>,
        folders: Vec<Folder>,
        playlist_id_floor: u64,
    ) -> Result<Self, String> {
        let track_ids: std::collections::HashSet<_> = tracks.iter().map(|t| t.id).collect();
        let playlist_ids: std::collections::HashSet<_> = playlists
            .iter()
            .map(|p| p.id)
            .chain(folders.iter().map(|f| f.id))
            .collect();
        if track_ids.len() != tracks.len()
            || playlist_ids.len() != playlists.len() + folders.len()
            || track_ids.contains(&0)
            || playlist_ids.contains(&0)
            || playlists
                .iter()
                .any(|p| p.track_ids.iter().any(|id| !track_ids.contains(id)))
        {
            return Err("Device database contains invalid track or playlist references".into());
        }
        let library = Self {
            tracks,
            playlists,
            folders,
            playlist_id_floor,
        };
        for node in library.nodes() {
            let mut current = node.parent();
            let mut visited = std::collections::HashSet::from([node.id()]);
            while let Some(id) = current {
                if !visited.insert(id) {
                    return Err("Cyclic playlist folder hierarchy".into());
                }
                current = library.folder(id).ok_or("Missing playlist folder")?.parent;
            }
        }
        Ok(library)
    }

    pub fn import(&mut self, path: PathBuf, title: String, artist: String) -> u64 {
        if let Some(track) = self.tracks.iter_mut().find(|track| track.path == path) {
            track.title = title;
            track.artist = artist;
            return track.id;
        }
        let id = self.tracks.iter().map(|track| track.id).max().unwrap_or(0) + 1;
        self.tracks.push(Track {
            id,
            path,
            title,
            artist,
            details: TrackDetails::default(),
        });
        id
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn track(&self, id: u64) -> Option<&Track> {
        self.tracks.iter().find(|t| t.id == id)
    }

    pub fn playlists(&self) -> &[Playlist] {
        &self.playlists
    }

    pub fn set_details(&mut self, id: u64, details: TrackDetails) {
        if let Some(track) = self.tracks.iter_mut().find(|t| t.id == id) {
            track.details = details;
        }
    }

    pub fn set_path(&mut self, id: u64, path: PathBuf) {
        if let Some(track) = self.tracks.iter_mut().find(|t| t.id == id) {
            track.path = path;
        }
    }

    pub fn folders(&self) -> &[Folder] {
        &self.folders
    }

    pub fn folder(&self, id: u64) -> Option<&Folder> {
        self.folders.iter().find(|f| f.id == id)
    }

    pub fn nodes(&self) -> impl Iterator<Item = PlaylistNode<'_>> {
        self.folders
            .iter()
            .map(PlaylistNode::Folder)
            .chain(self.playlists.iter().map(PlaylistNode::Playlist))
    }

    pub fn children(&self, parent: Option<u64>) -> Vec<PlaylistNode<'_>> {
        let mut children: Vec<_> = self.nodes().filter(|n| n.parent() == parent).collect();
        children.sort_by_key(|n| (n.sort_order(), n.id()));
        children
    }

    pub fn ancestors(&self, parent: Option<u64>) -> Vec<&Folder> {
        let mut ancestors = Vec::new();
        let mut current = parent;
        while let Some(folder) = current.and_then(|id| self.folder(id)) {
            if ancestors.iter().any(|f: &&Folder| f.id == folder.id) {
                break;
            }
            ancestors.push(folder);
            current = folder.parent;
        }
        ancestors.reverse();
        ancestors
    }

    pub fn playlist_label(&self, id: u64) -> String {
        let Some(node) = self.nodes().find(|n| n.id() == id) else {
            return String::new();
        };
        self.ancestors(node.parent())
            .iter()
            .map(|f| f.name.as_str())
            .chain(std::iter::once(node.name()))
            .collect::<Vec<_>>()
            .join(" / ")
    }

    fn next_node(&self, parent: Option<u64>) -> (u64, u32) {
        let id = self
            .nodes()
            .map(|n| n.id())
            .max()
            .unwrap_or(0)
            .max(self.playlist_id_floor)
            + 1;
        let order = self
            .children(parent)
            .iter()
            .map(|n| n.sort_order())
            .max()
            .map_or(0, |n| n + 1);
        (id, order)
    }

    pub fn create_playlist(&mut self, name: &str) -> Result<u64, String> {
        self.create_playlist_in(name, None)
    }

    pub fn create_playlist_in(&mut self, name: &str, parent: Option<u64>) -> Result<u64, String> {
        let name = self.valid_name(name, None, parent)?.to_owned();
        let (id, sort_order) = self.next_node(parent);
        self.playlists.push(Playlist {
            id,
            name,
            track_ids: Vec::new(),
            parent,
            sort_order,
        });
        Ok(id)
    }

    pub fn create_folder(&mut self, name: &str, parent: Option<u64>) -> Result<u64, String> {
        let name = self.valid_name(name, None, parent)?.to_owned();
        let (id, sort_order) = self.next_node(parent);
        self.folders.push(Folder {
            id,
            name,
            parent,
            sort_order,
        });
        Ok(id)
    }

    pub fn add_to_playlist(&mut self, playlist: u64, track: u64) -> Result<(), String> {
        self.add_tracks(playlist, &[track])
    }

    pub fn add_tracks(&mut self, playlist: u64, tracks: &[u64]) -> Result<(), String> {
        let known: std::collections::HashSet<_> = self.tracks.iter().map(|t| t.id).collect();
        if tracks.iter().any(|id| !known.contains(id)) {
            return Err("Track not found".into());
        }
        let playlist = self
            .playlists
            .iter_mut()
            .find(|p| p.id == playlist)
            .ok_or("Playlist not found")?;
        let mut existing: std::collections::HashSet<_> =
            playlist.track_ids.iter().copied().collect();
        playlist
            .track_ids
            .extend(tracks.iter().copied().filter(|id| existing.insert(*id)));
        Ok(())
    }

    pub fn remove_tracks(&mut self, playlist: u64, tracks: &[u64]) -> Result<(), String> {
        let playlist = self
            .playlists
            .iter_mut()
            .find(|p| p.id == playlist)
            .ok_or("Playlist not found")?;
        let removed: std::collections::HashSet<_> = tracks.iter().copied().collect();
        playlist.track_ids.retain(|id| !removed.contains(id));
        Ok(())
    }

    pub fn playlist(&self, id: u64) -> Option<&Playlist> {
        self.playlists.iter().find(|p| p.id == id)
    }

    pub fn move_track(&mut self, playlist: u64, track: u64, position: usize) -> Result<(), String> {
        let p = self
            .playlists
            .iter_mut()
            .find(|p| p.id == playlist)
            .ok_or("Playlist not found")?;
        let current = p
            .track_ids
            .iter()
            .position(|id| *id == track)
            .ok_or("Track not in playlist")?;
        if position >= p.track_ids.len() {
            return Err("Position outside playlist".into());
        }
        p.track_ids.remove(current);
        p.track_ids.insert(position, track);
        Ok(())
    }

    pub fn move_tracks_by(
        &mut self,
        playlist: u64,
        tracks: &[u64],
        direction: i32,
    ) -> Result<(), String> {
        if ![-1, 1].contains(&direction) {
            return Err("Choose up or down".into());
        }
        let playlist = self
            .playlists
            .iter_mut()
            .find(|p| p.id == playlist)
            .ok_or("Playlist not found")?;
        let selected: std::collections::HashSet<_> = tracks.iter().copied().collect();
        let present: std::collections::HashSet<_> = playlist.track_ids.iter().copied().collect();
        if !selected.is_subset(&present) {
            return Err("Track not in playlist".into());
        }
        if direction == -1 {
            for i in 1..playlist.track_ids.len() {
                if selected.contains(&playlist.track_ids[i])
                    && !selected.contains(&playlist.track_ids[i - 1])
                {
                    playlist.track_ids.swap(i, i - 1);
                }
            }
        } else {
            for i in (0..playlist.track_ids.len().saturating_sub(1)).rev() {
                if selected.contains(&playlist.track_ids[i])
                    && !selected.contains(&playlist.track_ids[i + 1])
                {
                    playlist.track_ids.swap(i, i + 1);
                }
            }
        }
        Ok(())
    }

    pub fn remove_from_playlist(&mut self, playlist: u64, track: u64) -> Result<(), String> {
        let p = self
            .playlists
            .iter_mut()
            .find(|p| p.id == playlist)
            .ok_or("Playlist not found")?;
        p.track_ids.retain(|id| *id != track);
        Ok(())
    }

    pub fn rename_playlist(&mut self, id: u64, name: &str) -> Result<(), String> {
        let parent = self.playlist(id).ok_or("Playlist not found")?.parent;
        let name = self.valid_name(name, Some(id), parent)?.to_owned();
        let playlist = self
            .playlists
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or("Playlist not found")?;
        playlist.name = name;
        Ok(())
    }

    pub fn delete_playlist(&mut self, id: u64) -> Result<(), String> {
        if self.playlist(id).is_none() {
            return Err("Playlist not found".into());
        }
        self.playlists.retain(|p| p.id != id);
        Ok(())
    }

    pub fn remove_track(&mut self, id: u64) {
        self.tracks.retain(|t| t.id != id);
        for playlist in &mut self.playlists {
            playlist.track_ids.retain(|t| *t != id);
        }
    }

    fn valid_name<'a>(
        &self,
        name: &'a str,
        except: Option<u64>,
        parent: Option<u64>,
    ) -> Result<&'a str, String> {
        if parent.is_some_and(|id| self.folder(id).is_none()) {
            return Err("Playlist folder not found".into());
        }
        let name = name.trim();
        if name.is_empty() {
            return Err("Enter a playlist name".into());
        }
        if self.nodes().any(|n| {
            n.parent() == parent
                && Some(n.id()) != except
                && n.name().to_lowercase() == name.to_lowercase()
        }) {
            return Err("A playlist or folder with that name already exists here".into());
        }
        Ok(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_selected_blocks_preserves_their_order_and_does_not_cross_the_end() {
        let mut library = Library::default();
        let tracks: Vec<_> = (0..6)
            .map(|n| library.import(format!("{n}.wav").into(), n.to_string(), String::new()))
            .collect();
        let playlist = library.create_playlist("Set").unwrap();
        library.add_tracks(playlist, &tracks).unwrap();
        let selected = [tracks[1], tracks[2], tracks[4]];
        library.move_tracks_by(playlist, &selected, -1).unwrap();
        assert_eq!(
            library.playlist(playlist).unwrap().track_ids,
            [
                tracks[1], tracks[2], tracks[0], tracks[4], tracks[3], tracks[5]
            ]
        );
        library.move_tracks_by(playlist, &selected, 1).unwrap();
        assert_eq!(library.playlist(playlist).unwrap().track_ids, tracks);
        library.move_tracks_by(playlist, &tracks, -1).unwrap();
        assert_eq!(library.playlist(playlist).unwrap().track_ids, tracks);
        assert!(library.move_tracks_by(playlist, &[999], -1).is_err());
        assert_eq!(library.playlist(playlist).unwrap().track_ids, tracks);
    }

    #[test]
    fn device_folder_cycles_and_missing_parents_are_rejected() {
        let folder = Folder {
            id: 80,
            name: "Sets".into(),
            parent: Some(80),
            sort_order: 0,
        };
        assert!(Library::from_parts(Vec::new(), Vec::new(), vec![folder.clone()], 80).is_err());
        let missing = Folder {
            parent: Some(999),
            ..folder
        };
        assert!(Library::from_parts(Vec::new(), Vec::new(), vec![missing], 80).is_err());
    }

    #[test]
    fn playlists_can_be_created_in_nested_folders_with_names_scoped_to_their_parent() {
        let mut library = Library::default();
        let sets = library.create_folder("Sets", None).unwrap();
        let house = library.create_folder("House", Some(sets)).unwrap();
        let set = library.create_playlist_in("Opening", Some(house)).unwrap();
        let root = library.create_playlist("Opening").unwrap();
        assert_eq!(library.playlist(set).unwrap().parent, Some(house));
        assert_eq!(library.folder(house).unwrap().parent, Some(sets));
        assert_eq!(library.playlist_label(set), "Sets / House / Opening");
        assert!(library.create_playlist_in("opening", Some(house)).is_err());
        assert!(library.create_playlist_in("Invalid", Some(root)).is_err());
        assert_eq!(library.children(Some(house))[0].id(), set);
    }

    #[test]
    fn importing_the_same_file_twice_keeps_one_track_and_its_identity() {
        let mut library = Library::default();
        let first = library.import("/music/track.wav".into(), "Track".into(), "Artist".into());
        let second = library.import("/music/track.wav".into(), "Track".into(), "Artist".into());

        assert_eq!(first, second);
        assert_eq!(library.tracks().len(), 1);
        assert_eq!(library.tracks()[0].title, "Track");
    }

    #[test]
    fn a_track_can_belong_to_two_playlists_without_copying_or_duplicate_entries() {
        let mut library = Library::default();
        let track = library.import("/music/track.wav".into(), "Track".into(), "Artist".into());
        let warmup = library.create_playlist("Warm up").unwrap();
        let closing = library.create_playlist("Closing").unwrap();
        library.add_to_playlist(warmup, track).unwrap();
        library.add_to_playlist(warmup, track).unwrap();
        library.add_to_playlist(closing, track).unwrap();

        assert_eq!(library.playlist(warmup).unwrap().track_ids, [track]);
        assert_eq!(library.playlist(closing).unwrap().track_ids, [track]);
        assert_eq!(library.tracks().len(), 1);
    }

    #[test]
    fn invalid_playlist_changes_leave_the_library_unchanged() {
        let mut library = Library::default();
        let playlist = library.create_playlist("  Warm up  ").unwrap();
        assert!(library.create_playlist("warm UP").is_err());
        assert!(library.create_playlist("  ").is_err());
        assert!(library.add_to_playlist(playlist, 999).is_err());
        assert_eq!(library.playlist(playlist).unwrap().name, "Warm up");
        assert!(library.playlist(playlist).unwrap().track_ids.is_empty());
    }

    #[test]
    fn organising_a_playlist_changes_its_order_without_removing_library_tracks() {
        let mut library = Library::default();
        let a = library.import("/music/a.wav".into(), "A".into(), "".into());
        let b = library.import("/music/b.wav".into(), "B".into(), "".into());
        let p = library.create_playlist("Set").unwrap();
        library.add_to_playlist(p, a).unwrap();
        library.add_to_playlist(p, b).unwrap();
        library.move_track(p, b, 0).unwrap();
        assert_eq!(library.playlist(p).unwrap().track_ids, [b, a]);
        assert!(library.move_track(p, b, 2).is_err());
        library.remove_from_playlist(p, a).unwrap();
        assert_eq!(library.playlist(p).unwrap().track_ids, [b]);
        assert_eq!(library.tracks().len(), 2);
    }

    #[test]
    fn deleting_and_renaming_keep_other_playlists_and_tracks_intact() {
        let mut library = Library::default();
        let track = library.import("/music/a.wav".into(), "A".into(), "Artist".into());
        let set = library.create_playlist("Set").unwrap();
        let other = library.create_playlist("Other").unwrap();
        library.add_to_playlist(set, track).unwrap();
        library.add_to_playlist(other, track).unwrap();
        assert!(library.rename_playlist(set, "other").is_err());
        library.rename_playlist(set, "Opening").unwrap();
        assert_eq!(library.playlist(set).unwrap().name, "Opening");
        library.delete_playlist(set).unwrap();
        assert!(library.playlist(set).is_none());
        assert_eq!(library.playlist(other).unwrap().track_ids, [track]);
        assert_eq!(library.tracks().len(), 1);
        library.remove_track(track);
        assert!(library.playlist(other).unwrap().track_ids.is_empty());
        assert!(library.tracks().is_empty());
    }
}
