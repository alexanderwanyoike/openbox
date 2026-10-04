mod browser;
mod knob;
mod players;
mod preview;
mod selection;
mod view;
mod waveform;

use browser::{SortColumn, TrackBrowser};
use iced::{
    Event, Subscription, Task, event, keyboard,
    widget::operation::{self, RelativeOffset},
};
use openbox_domain::{Library, PlaylistNode};
use openbox_library::{
    application::{Edit, Target, Workspace},
    import::{self, ImportReport},
};
use openbox_rekordbox::{self as usb, DeviceLibrary, ExportPhase, ExportProgress};
use std::{
    collections::HashSet,
    fmt,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistChoice {
    id: u64,
    name: String,
}

impl fmt::Display for PlaylistChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.fmt(f)
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Switch(Target),
    SelectPlaylist(Option<u64>),
    OpenPlaylist(Target, u64),
    OpenFolder(Target, Option<u64>),
    ToggleTree(Target, u64),
    SelectFolder(Option<u64>),
    ToggleFolder(u64),
    SelectTrack(u64),
    ToggleTrack(u64),
    ToggleSource(u64),
    SelectAll(bool),
    ClearSelection(bool),
    Sort(bool, SortColumn),
    Scrolled(bool, f32, f32),
    Keyboard(keyboard::Event),
    Search(String),
    NewName(String),
    RenameName(String),
    CreatePlaylist,
    CreateFromSelection,
    CreateFolder,
    RenamePlaylist,
    DeletePlaylist,
    RemoveTrack,
    ConfirmDelete,
    CancelDelete,
    AddTo(PlaylistChoice),
    RemoveFromPlaylist,
    MoveTrack(i32),
    ImportFolder,
    ImportFiles,
    PathsSelected(Option<Vec<PathBuf>>),
    Imported(Result<ImportReport, String>),
    Analyze,
    ChooseUsb,
    UsbSelected(Option<PathBuf>),
    Connected(Result<Option<DeviceLibrary>, String>),
    RefreshUsb,
    Export,
    Exported(Result<DeviceLibrary, String>),
    ImportUsbPlaylist,
    PlaylistImported(Result<(Library, u64), String>),
    BrowseLibrary,
    SourceSearch(String),
    SelectSource(u64),
    AddSource,
    ReplaceSource,
    SendTrackToUsb(PlaylistChoice),
    UsbUpdated(Result<DeviceLibrary, String>),
    UndoUsb,
    Tick,
    Preview(preview::Event),
}

#[derive(Clone)]
enum Deletion {
    Playlist(u64),
    Tracks(Vec<u64>),
}

#[derive(Clone)]
pub struct App {
    workspace: Workspace,
    target: Target,
    selected_playlist: Option<u64>,
    selected_folder: Option<u64>,
    library_expanded: HashSet<u64>,
    usb_expanded: HashSet<u64>,
    tracks: TrackBrowser,
    modifiers: keyboard::Modifiers,
    source_focused: bool,
    new_name: String,
    rename_name: String,
    usb_root: Option<PathBuf>,
    busy: bool,
    browse_library: bool,
    source: TrackBrowser,
    undo_usb: Option<Library>,
    pending_undo: Option<Library>,
    deletion: Option<Deletion>,
    status: String,
    error: bool,
    export_progress: Option<ExportProgress>,
    progress_updates: Option<Arc<Mutex<ExportProgress>>>,
    import_updates: Option<Arc<Mutex<import::ImportProgress>>>,
    import_progress: Option<import::ImportProgress>,
    analyzing: bool,
    preview: preview::Preview,
}

impl App {
    pub fn load(library_path: PathBuf, usb_root: Option<PathBuf>) -> Result<Self, String> {
        let mut workspace = Workspace::load(library_path)?;
        if let Some(root) = &usb_root
            && root.join("PIONEER/rekordbox/export.pdb").exists()
        {
            workspace.device = Some(usb::open(root)?);
        }
        Ok(Self {
            workspace,
            target: Target::Library,
            selected_playlist: None,
            selected_folder: None,
            library_expanded: HashSet::new(),
            usb_expanded: HashSet::new(),
            tracks: TrackBrowser::default(),
            modifiers: keyboard::Modifiers::default(),
            source_focused: false,
            new_name: String::new(),
            rename_name: String::new(),
            usb_root,
            busy: false,
            browse_library: false,
            source: TrackBrowser::default(),
            undo_usb: None,
            pending_undo: None,
            deletion: None,
            status: "Ready. Add music or select a playlist.".into(),
            error: false,
            export_progress: None,
            progress_updates: None,
            import_updates: None,
            import_progress: None,
            analyzing: false,
            preview: preview::Preview::default(),
        })
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                self.preview.tick();
                if let Some(updates) = &self.progress_updates {
                    self.export_progress = Some(updates.lock().unwrap().clone());
                }
                if let Some(updates) = &self.import_updates {
                    self.import_progress = Some(updates.lock().unwrap().clone());
                }
            }
            Message::Preview(preview::Event::Load(deck)) => {
                if let Some(track) = self.preview_track() {
                    return self.preview.load(deck, track).map(Message::Preview);
                }
            }
            Message::Preview(event) => {
                let task = self.preview.update(event).map(Message::Preview);
                self.refresh_preview_metadata();
                return task;
            }
            Message::Switch(target) => {
                self.target = target;
                self.select_playlist(None);
                self.tracks.query.clear();
                self.browse_library = false;
                self.deletion = None;
                return self.reveal_selected();
            }
            Message::OpenPlaylist(target, id) => {
                self.target = target;
                self.select_playlist(Some(id));
                return self.reset_scroll(false);
            }
            Message::OpenFolder(target, id) => {
                self.target = target;
                self.select_folder(id);
                return self.reveal_selected();
            }
            Message::ToggleTree(target, id) => {
                let expanded = match target {
                    Target::Library => &mut self.library_expanded,
                    Target::Usb => &mut self.usb_expanded,
                };
                if !expanded.remove(&id) {
                    expanded.insert(id);
                }
            }
            Message::SelectPlaylist(id) => {
                self.select_playlist(id);
                return self.reset_scroll(false);
            }
            Message::SelectFolder(id) => {
                self.select_folder(id);
                return self.reveal_selected();
            }
            Message::ToggleFolder(id) => {
                if !self.expanded_mut().remove(&id) {
                    self.expanded_mut().insert(id);
                }
            }
            Message::SelectTrack(id) => {
                self.click_track(id, false, false);
                return Self::focus_tracks();
            }
            Message::ToggleTrack(id) => {
                self.click_track(id, false, true);
                return Self::focus_tracks();
            }
            Message::ToggleSource(id) => {
                self.click_track(id, true, true);
                return Self::focus_tracks();
            }
            Message::SelectAll(source) => {
                let ids = self.visible_ids(source);
                self.browser_mut(source).selection.select_all(&ids);
                self.source_focused = source;
                return Self::focus_tracks();
            }
            Message::ClearSelection(source) => self.browser_mut(source).selection.clear(),
            Message::Sort(source, column) => {
                self.browser_mut(source).sort_by(column);
                return self.reset_scroll(source);
            }
            Message::Scrolled(source, offset, height) => {
                let browser = self.browser_mut(source);
                browser.offset = offset;
                browser.height = height;
            }
            Message::Search(value) => return self.search_tracks(value, false),
            Message::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                self.modifiers = modifiers
            }
            Message::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                self.modifiers = modifiers;
                match key {
                    keyboard::Key::Character(c)
                        if c.eq_ignore_ascii_case("a") && modifiers.command() =>
                    {
                        return self.update(Message::SelectAll(self.source_focused));
                    }
                    keyboard::Key::Named(keyboard::key::Named::Escape) => {
                        self.deletion = None;
                        return self.update(Message::ClearSelection(self.source_focused));
                    }
                    keyboard::Key::Named(keyboard::key::Named::Delete)
                        if self.selected_playlist.is_some() && !self.source_focused =>
                    {
                        return self.update(Message::RemoveFromPlaylist);
                    }
                    _ => {}
                }
            }
            Message::NewName(value) => self.new_name = value,
            Message::RenameName(value) => self.rename_name = value,
            Message::CreateFromSelection if !self.busy && self.target == Target::Library => {
                let tracks = self.selected_ids(false);
                if tracks.is_empty() {
                    return Task::none();
                }
                let mut name = "New playlist".to_string();
                let mut suffix = 2;
                while self
                    .workspace
                    .library
                    .children(self.selected_folder)
                    .iter()
                    .any(|node| node.name().eq_ignore_ascii_case(&name))
                {
                    name = format!("New playlist {suffix}");
                    suffix += 1;
                }
                if self.edit(
                    Edit::CreateWithTracks(name, self.selected_folder, tracks),
                    "Playlist created from selection",
                ) {
                    let id = self.workspace.library.playlists().last().map(|p| p.id);
                    self.select_playlist(id);
                    return self
                        .reveal_selected()
                        .chain(operation::focus("playlist-name"))
                        .chain(operation::select_all("playlist-name"));
                }
            }
            Message::CreatePlaylist if !self.busy => {
                if self.edit(
                    Edit::CreateIn(self.new_name.clone(), self.selected_folder),
                    "Playlist created",
                ) {
                    let id = self
                        .collection()
                        .and_then(|l| l.playlists().last())
                        .map(|p| p.id);
                    self.select_playlist(id);
                    self.new_name.clear();
                    return self.reveal_selected();
                }
            }
            Message::CreateFolder if !self.busy => {
                if self.edit(
                    Edit::CreateFolder(self.new_name.clone(), self.selected_folder),
                    "Folder created",
                ) {
                    let id = self
                        .collection()
                        .and_then(|l| l.folders().last())
                        .map(|f| f.id);
                    self.select_folder(id);
                    self.new_name.clear();
                    return self.reveal_selected();
                }
            }
            Message::RenamePlaylist if !self.busy => {
                if let Some(id) = self.selected_playlist {
                    self.edit(
                        Edit::Rename(id, self.rename_name.clone()),
                        "Playlist renamed",
                    );
                }
            }
            Message::DeletePlaylist if !self.busy => {
                if self.target == Target::Usb {
                    if let Some(id) = self.selected_playlist
                        && self.edit(Edit::Delete(id), "USB playlist removed. Undo is available.")
                    {
                        self.select_playlist(None);
                    }
                } else {
                    self.deletion = self.selected_playlist.map(Deletion::Playlist);
                }
            }
            Message::RemoveTrack if !self.busy => {
                if self.target == Target::Library {
                    let tracks = self.selected_ids(false);
                    if !tracks.is_empty() {
                        self.deletion = Some(Deletion::Tracks(tracks));
                    }
                }
            }
            Message::ConfirmDelete if !self.busy => {
                if let Some(deletion) = self.deletion.take() {
                    match deletion {
                        Deletion::Playlist(id) => {
                            if self
                                .edit(Edit::Delete(id), "Playlist deleted. Audio files retained.")
                            {
                                self.select_playlist(None);
                            }
                        }
                        Deletion::Tracks(ids) => {
                            if self.edit(
                                Edit::RemoveTracks(ids),
                                "Removed from library. Source file retained.",
                            ) {
                                self.tracks.selection.clear();
                            }
                        }
                    }
                }
            }
            Message::CancelDelete => self.deletion = None,
            Message::AddTo(playlist) if !self.busy => {
                let tracks = self.selected_ids(false);
                if !tracks.is_empty() {
                    self.edit(
                        Edit::AddMany(playlist.id, tracks.clone()),
                        &format!("Added {} tracks to {}", tracks.len(), playlist.name),
                    );
                }
            }
            Message::RemoveFromPlaylist if !self.busy => {
                let tracks = self.selected_ids(false);
                if let Some(playlist) = self.selected_playlist
                    && !tracks.is_empty()
                    && self.edit(
                        Edit::RemoveMany(playlist, tracks),
                        "Selected tracks removed from playlist",
                    )
                {
                    self.tracks.selection.clear();
                }
            }
            Message::MoveTrack(direction) if !self.busy => {
                let tracks = self.selected_ids(false);
                if let Some(playlist) = self.selected_playlist
                    && !tracks.is_empty()
                    && self.tracks.sort == SortColumn::Order
                    && self.tracks.query.is_empty()
                {
                    self.edit(
                        Edit::MoveMany(playlist, tracks, direction),
                        "Playlist order saved",
                    );
                }
            }
            Message::ImportFolder if !self.busy => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Add a music folder")
                            .pick_folder()
                            .await
                            .map(|p| vec![p.path().into()])
                    },
                    Message::PathsSelected,
                );
            }
            Message::ImportFiles if !self.busy => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Add music files")
                            .add_filter(
                                "Audio",
                                &[
                                    "mp3", "wav", "aif", "aiff", "flac", "m4a", "aac", "ogg",
                                    "opus",
                                ],
                            )
                            .pick_files()
                            .await
                            .map(|files| files.iter().map(|p| p.path().into()).collect())
                    },
                    Message::PathsSelected,
                );
            }
            Message::PathsSelected(Some(paths)) if !self.busy => {
                self.analyzing = false;
                return self.analyze_paths(paths);
            }
            Message::Analyze if !self.busy && self.target == Target::Library => {
                let ids = self.selected_ids(false);
                let paths = self
                    .workspace
                    .library
                    .tracks()
                    .iter()
                    .filter(|t| ids.is_empty() || ids.contains(&t.id))
                    .map(|t| t.path.clone())
                    .collect();
                self.analyzing = true;
                return self.analyze_paths(paths);
            }
            Message::Imported(result) => {
                self.busy = false;
                self.import_updates = None;
                self.import_progress = None;
                match result {
                    Ok(report) => {
                        let skipped = report.skipped.clone();
                        let analyzed = report.tracks.len();
                        let ungridded = report
                            .tracks
                            .iter()
                            .filter(|t| {
                                t.details
                                    .analysis
                                    .as_ref()
                                    .is_none_or(|g| g.beats.is_empty())
                            })
                            .count();
                        match self.workspace.import(report) {
                            Ok(added) => {
                                self.refresh_preview_metadata();
                                let summary = if self.analyzing {
                                    format!(
                                        "Analyzed {analyzed} tracks. {ungridded} without a detected grid. {} skipped.",
                                        skipped.len()
                                    )
                                } else {
                                    format!(
                                        "Added {added} tracks. {ungridded} without a detected grid. {} files skipped.",
                                        skipped.len()
                                    )
                                };
                                self.notice(
                                    if skipped.is_empty() {
                                        summary
                                    } else {
                                        format!("{summary} {}", skipped[0])
                                    },
                                    !skipped.is_empty(),
                                );
                            }
                            Err(error) => self.notice(error, true),
                        }
                    }
                    Err(error) => self.notice(error, true),
                }
            }
            Message::ChooseUsb if !self.busy => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Select the root of your mounted FAT32 USB")
                            .pick_folder()
                            .await
                            .map(|p| p.path().into())
                    },
                    Message::UsbSelected,
                );
            }
            Message::UsbSelected(Some(root)) if !self.busy => {
                self.usb_root = Some(root);
                self.workspace.device = None;
                self.begin("Reading USB library...");
                return self.connect();
            }
            Message::RefreshUsb if !self.busy => {
                self.begin("Refreshing USB library...");
                return self.connect();
            }
            Message::Connected(result) => {
                self.busy = false;
                match result {
                    Ok(device) => {
                        self.workspace.device = device;
                        self.undo_usb = None;
                        self.target = Target::Usb;
                        self.select_playlist(None);
                        self.notice(
                            if self.workspace.device.is_some() {
                                "USB library opened"
                            } else {
                                "Ready for your first export"
                            },
                            false,
                        );
                    }
                    Err(error) => {
                        self.workspace.device = None;
                        self.notice(error, true);
                    }
                }
            }
            Message::Export if !self.busy => {
                if let Some(root) = self.usb_root.clone() {
                    let library = self.workspace.library.clone();
                    let ids = self.export_ids();
                    self.browse_library = false;
                    self.begin("Exporting audio and waveforms to USB...");
                    let updates = self.start_progress();
                    return Task::perform(
                        background(move || {
                            usb::export_with_progress(&library, &ids, &root, |progress| {
                                *updates.lock().unwrap() = progress
                            })
                        }),
                        Message::Exported,
                    );
                }
            }
            Message::Exported(result) => {
                self.busy = false;
                self.finish_progress(result.is_ok());
                match result {
                    Ok(device) => {
                        let tracks = device.library.tracks().len();
                        let playlists = device.library.playlists().len();
                        self.workspace.device = Some(device);
                        self.target = Target::Usb;
                        self.select_playlist(None);
                        self.notice(format!("USB updated: {tracks} tracks and {playlists} playlists. Existing music kept."), false);
                    }
                    Err(error) => self.notice(error, true),
                }
            }
            Message::ImportUsbPlaylist if !self.busy => {
                if let Some(id) = self.selected_playlist {
                    let mut workspace = self.workspace.clone();
                    self.begin("Copying USB playlist into your library...");
                    return Task::perform(
                        background(move || {
                            let id = workspace.import_usb_playlist(id)?;
                            Ok((workspace.library, id))
                        }),
                        Message::PlaylistImported,
                    );
                }
            }
            Message::PlaylistImported(result) => {
                self.busy = false;
                match result {
                    Ok((library, id)) => {
                        self.workspace.library = library;
                        self.target = Target::Library;
                        self.select_playlist(Some(id));
                        self.notice(
                            "Playlist imported. Its audio is available without the USB.",
                            false,
                        );
                        return self.reveal_selected();
                    }
                    Err(error) => self.notice(error, true),
                }
            }
            Message::BrowseLibrary => {
                self.browse_library = !self.browse_library;
                self.source.selection.clear();
            }
            Message::SourceSearch(value) => return self.search_tracks(value, true),
            Message::SelectSource(id) => {
                self.click_track(id, true, false);
                return Self::focus_tracks();
            }
            Message::AddSource if !self.busy => {
                let songs = self.selected_ids(true);
                if let Some(playlist) = self.selected_playlist
                    && !songs.is_empty()
                {
                    return self.transfer_songs(songs, playlist, None);
                }
            }
            Message::ReplaceSource if !self.busy => {
                if let (Some(song), Some(playlist), Some(old)) = (
                    self.source.selection.single(),
                    self.selected_playlist,
                    self.tracks.selection.single(),
                ) {
                    return self.transfer_songs(vec![song], playlist, Some(old));
                }
            }
            Message::SendTrackToUsb(playlist) if !self.busy => {
                let songs = self.selected_ids(false);
                if !songs.is_empty() {
                    return self.transfer_songs(songs, playlist.id, None);
                }
            }
            Message::UsbUpdated(result) => {
                self.busy = false;
                self.finish_progress(result.is_ok());
                match result {
                    Ok(device) => {
                        self.workspace.device = Some(device);
                        self.undo_usb = self.pending_undo.take();
                        self.tracks.selection.clear();
                        self.notice("USB playlist updated. Original audio kept.", false);
                    }
                    Err(error) => {
                        self.pending_undo = None;
                        self.notice(error, true);
                    }
                }
            }
            Message::UndoUsb if !self.busy => {
                if let Some(previous) = self.undo_usb.clone() {
                    match self.workspace.undo_usb_playlists(&previous) {
                        Ok(()) => {
                            self.undo_usb = None;
                            self.select_playlist(None);
                            self.notice("USB playlist change undone", false);
                        }
                        Err(error) => self.notice(error, true),
                    }
                }
            }
            _ => {}
        }
        Task::none()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let keyboard = event::listen_with(|event, status, _| match event {
            Event::Keyboard(event @ keyboard::Event::ModifiersChanged(_)) => {
                Some(Message::Keyboard(event))
            }
            Event::Keyboard(
                ref event @ keyboard::Event::KeyPressed {
                    ref key, modifiers, ..
                },
            ) if status == event::Status::Ignored
                && (matches!(key, keyboard::Key::Character(c) if c.eq_ignore_ascii_case("a") && modifiers.command())
                    || matches!(
                        key,
                        keyboard::Key::Named(
                            keyboard::key::Named::Escape | keyboard::key::Named::Delete
                        )
                    )) =>
            {
                Some(Message::Keyboard(event.clone()))
            }
            _ => None,
        });
        let audio_live = self.preview.engine.is_some()
            && (self.preview.snapshot.decks.iter().any(|d| d.playing)
                || self.preview.snapshot.peak > 0.002);
        if self.progress_updates.is_some() || self.import_updates.is_some() || audio_live {
            let interval = if audio_live && self.preview.visible {
                33
            } else {
                80
            };
            Subscription::batch([
                keyboard,
                iced::time::every(Duration::from_millis(interval)).map(|_| Message::Tick),
            ])
        } else {
            keyboard
        }
    }

    fn preview_track(&self) -> Option<openbox_domain::Track> {
        if self.source_focused && self.browse_library && self.target == Target::Usb {
            self.workspace
                .library
                .track(self.source.selection.single()?)
                .cloned()
        } else {
            self.collection()?
                .track(self.tracks.selection.single()?)
                .cloned()
        }
    }

    fn focus_tracks() -> Task<Message> {
        // A reserved focus ID clears text inputs after buttons capture a track-list click.
        operation::focus("track-selection")
    }

    fn browser(&self, source: bool) -> &TrackBrowser {
        if source { &self.source } else { &self.tracks }
    }

    fn browser_mut(&mut self, source: bool) -> &mut TrackBrowser {
        if source {
            &mut self.source
        } else {
            &mut self.tracks
        }
    }

    fn visible_ids(&self, source: bool) -> Vec<u64> {
        if source {
            self.source.ids(&self.workspace.library, None)
        } else {
            self.collection()
                .map(|l| self.tracks.ids(l, self.selected_playlist))
                .unwrap_or_default()
        }
    }

    fn selected_ids(&self, source: bool) -> Vec<u64> {
        self.browser(source)
            .selection
            .ordered(&self.visible_ids(source))
    }

    fn click_track(&mut self, id: u64, source: bool, toggle: bool) {
        let visible = self.visible_ids(source);
        let additive = toggle || self.modifiers.command();
        let range = !toggle && self.modifiers.shift();
        self.browser_mut(source)
            .selection
            .click(id, &visible, additive, range);
        self.source_focused = source;
    }

    fn search_tracks(&mut self, query: String, source: bool) -> Task<Message> {
        self.browser_mut(source).query = query;
        self.browser_mut(source).offset = 0.0;
        let visible = self.visible_ids(source);
        self.browser_mut(source).selection.retain(&visible);
        self.source_focused = source;
        self.reset_scroll(source)
    }

    fn reset_scroll(&self, source: bool) -> Task<Message> {
        operation::snap_to(
            if source {
                "source-tracks"
            } else {
                "main-tracks"
            },
            RelativeOffset { x: 0.0, y: 0.0 },
        )
    }

    fn collection(&self) -> Option<&Library> {
        self.workspace.collection(self.target)
    }

    fn select_playlist(&mut self, id: Option<u64>) {
        self.browse_library = self.target == Target::Usb && id.is_some();
        self.source.selection.clear();
        self.selected_playlist = id;
        self.selected_folder = id
            .and_then(|id| self.collection()?.playlist(id))
            .and_then(|p| p.parent);
        self.expand_ancestors();
        self.tracks = TrackBrowser::default();
        self.source_focused = false;
        self.rename_name = id
            .and_then(|id| self.collection()?.playlist(id))
            .map(|p| p.name.clone())
            .unwrap_or_default();
    }

    fn select_folder(&mut self, id: Option<u64>) {
        self.select_playlist(None);
        self.selected_folder = id;
        self.expand_ancestors();
        if let Some(id) = id {
            self.expanded_mut().insert(id);
        }
    }

    fn expanded_mut(&mut self) -> &mut HashSet<u64> {
        match self.target {
            Target::Library => &mut self.library_expanded,
            Target::Usb => &mut self.usb_expanded,
        }
    }

    fn expand_ancestors(&mut self) {
        let ids: Vec<_> = self
            .collection()
            .map(|l| {
                l.ancestors(self.selected_folder)
                    .iter()
                    .map(|f| f.id)
                    .collect()
            })
            .unwrap_or_default();
        self.expanded_mut().extend(ids);
    }

    fn tree_nodes(&self, target: Target) -> Vec<(PlaylistNode<'_>, usize)> {
        fn visit<'a>(
            library: &'a Library,
            parent: Option<u64>,
            depth: usize,
            expanded: &HashSet<u64>,
            result: &mut Vec<(PlaylistNode<'a>, usize)>,
        ) {
            for node in library.children(parent) {
                result.push((node, depth));
                if matches!(node, PlaylistNode::Folder(_)) && expanded.contains(&node.id()) {
                    visit(library, Some(node.id()), depth + 1, expanded, result);
                }
            }
        }
        let mut result = Vec::new();
        if let Some(library) = self.workspace.collection(target) {
            let expanded = match target {
                Target::Library => &self.library_expanded,
                Target::Usb => &self.usb_expanded,
            };
            visit(library, None, 0, expanded, &mut result);
        }
        result
    }

    fn reveal_selected(&self) -> Task<Message> {
        let mut rows = Vec::new();
        for target in [Target::Library, Target::Usb] {
            rows.extend([(target, None); 3]);
            rows.extend(
                self.tree_nodes(target)
                    .iter()
                    .map(|(node, _)| (target, Some(node.id()))),
            );
        }
        let selected = (self.target, self.selected_playlist.or(self.selected_folder));
        let index = rows.iter().position(|row| *row == selected).unwrap_or(0);
        let y = index as f32 / rows.len().saturating_sub(1).max(1) as f32;
        operation::snap_to("playlist-tree", RelativeOffset { x: 0.0, y })
    }

    fn edit(&mut self, edit: Edit, success: &str) -> bool {
        let previous = self.workspace.device.as_ref().map(|d| d.library.clone());
        match self.workspace.edit(self.target, edit) {
            Ok(()) => {
                if self.target == Target::Usb {
                    self.undo_usb = previous;
                }
                self.notice(success, false);
                true
            }
            Err(error) => {
                self.notice(error, true);
                false
            }
        }
    }

    fn notice(&mut self, status: impl Into<String>, error: bool) {
        self.status = status.into();
        self.error = error;
    }

    fn begin(&mut self, status: &str) {
        self.busy = true;
        self.export_progress = None;
        self.progress_updates = None;
        self.import_progress = None;
        self.import_updates = None;
        self.notice(status, false);
    }

    fn analyze_paths(&mut self, paths: Vec<PathBuf>) -> Task<Message> {
        self.begin("Reading tags and analyzing BPM, key and beats...");
        let artwork = self.workspace.artwork_directory();
        let updates = Arc::new(Mutex::new(import::ImportProgress::default()));
        self.import_updates = Some(updates.clone());
        Task::perform(
            background(move || {
                import::scan_with_progress(&paths, &artwork, |progress| {
                    *updates.lock().unwrap() = progress
                })
            }),
            Message::Imported,
        )
    }

    fn refresh_preview_metadata(&mut self) {
        for deck in &mut self.preview.decks {
            for track in [deck.track.as_mut(), deck.pending.as_mut()]
                .into_iter()
                .flatten()
            {
                if let Some(current) = self
                    .workspace
                    .library
                    .tracks()
                    .iter()
                    .find(|t| t.path == track.path)
                    && current != track
                {
                    *track = current.clone();
                }
            }
        }
    }

    fn start_progress(&mut self) -> Arc<Mutex<ExportProgress>> {
        let progress = ExportProgress {
            percent: 0.0,
            phase: ExportPhase::Checking,
            track: String::new(),
            completed: 0,
            total: 0,
        };
        self.export_progress = Some(progress.clone());
        let updates = Arc::new(Mutex::new(progress));
        self.progress_updates = Some(updates.clone());
        updates
    }

    fn finish_progress(&mut self, success: bool) {
        self.export_progress = self.progress_updates.take().and_then(|updates| {
            success.then(|| {
                let mut progress = updates.lock().unwrap().clone();
                progress.percent = 100.0;
                progress.phase = ExportPhase::Complete;
                progress
            })
        });
    }

    fn export_ids(&self) -> Vec<u64> {
        if self.target == Target::Library
            && let Some(id) = self.selected_playlist
        {
            return vec![id];
        }
        self.workspace
            .library
            .playlists()
            .iter()
            .filter(|p| {
                self.selected_folder.is_none_or(|folder| {
                    self.workspace
                        .library
                        .ancestors(p.parent)
                        .iter()
                        .any(|f| f.id == folder)
                })
            })
            .map(|p| p.id)
            .collect()
    }

    fn transfer_songs(
        &mut self,
        songs: Vec<u64>,
        playlist: u64,
        replace: Option<u64>,
    ) -> Task<Message> {
        let Some(device) = self.workspace.device.clone() else {
            return Task::none();
        };
        let library = self.workspace.library.clone();
        self.pending_undo = Some(device.library.clone());
        self.begin(&format!("Transferring {} tracks to USB...", songs.len()));
        let updates = self.start_progress();
        Task::perform(
            background(move || {
                usb::put_tracks_with_progress(
                    &device,
                    &library,
                    &songs,
                    playlist,
                    replace,
                    |progress| *updates.lock().unwrap() = progress,
                )
            }),
            Message::UsbUpdated,
        )
    }

    fn connect(&self) -> Task<Message> {
        let Some(root) = self.usb_root.clone() else {
            return Task::done(Message::Connected(Err("Select a USB first".into())));
        };
        Task::perform(
            background(move || {
                if root.join("PIONEER/rekordbox/export.pdb").exists() {
                    usb::open(&root).map(Some)
                } else if root.join("PIONEER").exists() {
                    Err("No traditional Rekordbox export.pdb found on this USB.".into())
                } else {
                    Ok(None)
                }
            }),
            Message::Connected,
        )
    }
}

async fn background<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| format!("Operation stopped: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_reanalysis_refreshes_loaded_deck_metadata_without_moving_playback() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = App::load(dir.path().join("library.json"), None).unwrap();
        let id = app.workspace.library.import(
            dir.path().join("music.wav"),
            "Before".into(),
            String::new(),
        );
        let mut track = app.workspace.library.track(id).unwrap().clone();
        app.preview.decks[0].track = Some(track.clone());
        app.preview.decks[1].pending = Some(track.clone());
        app.preview.snapshot.decks[0].position = 12.5;
        track.title = "After".into();
        track.details.bpm = Some(128.0);
        let _ = app.update(Message::Imported(Ok(ImportReport {
            tracks: vec![track],
            skipped: Vec::new(),
        })));
        assert_eq!(app.preview.decks[0].track.as_ref().unwrap().title, "After");
        assert_eq!(
            app.preview.decks[0].track.as_ref().unwrap().details.bpm,
            Some(128.0)
        );
        assert_eq!(
            app.preview.decks[1].pending.as_ref().unwrap().title,
            "After"
        );
        assert_eq!(app.preview.snapshot.decks[0].position, 12.5);
    }

    #[test]
    fn the_status_bar_receives_live_export_progress_and_clears_it_after_failure() {
        let folder = tempfile::tempdir().unwrap();
        let mut app = App::load(folder.path().join("library.json"), None).unwrap();
        app.begin("Exporting");
        let updates = app.start_progress();
        updates.lock().unwrap().percent = 42.0;
        let _ = app.update(Message::Tick);
        assert_eq!(app.export_progress.as_ref().unwrap().percent, 42.0);
        let _ = app.update(Message::Exported(Err("Drive disconnected".into())));
        assert!(app.export_progress.is_none());
        assert!(!app.busy);
        assert!(app.error);
    }

    #[test]
    fn keyboard_selection_adds_all_filtered_tracks_in_a_single_playlist_operation() {
        let local = tempfile::tempdir().unwrap();
        let path = local.path().join("library.json");
        let mut app = App::load(path.clone(), None).unwrap();
        let playlist = app.workspace.library.create_playlist("Set").unwrap();
        let selected: Vec<_> = (0..300)
            .map(|n| {
                app.workspace.library.import(
                    format!("house-{n}.wav").into(),
                    format!("House {n}"),
                    "Artist".into(),
                )
            })
            .collect();
        app.workspace
            .library
            .import("hidden.wav".into(), "Techno".into(), "Artist".into());
        let _ = app.update(Message::Search("house".into()));
        let _ = app.update(Message::SelectTrack(selected[0]));
        let _ = app.update(Message::Keyboard(keyboard::Event::ModifiersChanged(
            keyboard::Modifiers::CTRL,
        )));
        let _ = app.update(Message::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Character("a".into()),
            modified_key: keyboard::Key::Character("a".into()),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyA),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::CTRL,
            text: None,
            repeat: false,
        }));
        let _ = app.update(Message::AddTo(PlaylistChoice {
            id: playlist,
            name: "Set".into(),
        }));
        let saved = Workspace::load(path).unwrap();
        assert_eq!(
            saved.library.playlist(playlist).unwrap().track_ids,
            selected
        );
        let _ = app.update(Message::OpenPlaylist(Target::Library, playlist));
        let _ = app.update(Message::SelectAll(false));
        let _ = app.update(Message::RemoveFromPlaylist);
        assert!(
            app.workspace
                .library
                .playlist(playlist)
                .unwrap()
                .track_ids
                .is_empty()
        );
        assert_eq!(app.workspace.library.tracks().len(), 301);
    }

    #[test]
    fn selected_folders_control_creation_and_collapsing_hides_their_children() {
        let local = tempfile::tempdir().unwrap();
        let mut app = App::load(local.path().join("library.json"), None).unwrap();
        let _ = app.update(Message::NewName("Sets".into()));
        let _ = app.update(Message::CreateFolder);
        let sets = app.selected_folder.unwrap();
        let _ = app.update(Message::NewName("House".into()));
        let _ = app.update(Message::CreateFolder);
        let house = app.selected_folder.unwrap();
        let _ = app.update(Message::NewName("Opening".into()));
        let _ = app.update(Message::CreatePlaylist);
        let playlist = app.selected_playlist.unwrap();
        assert_eq!(
            app.workspace.library.playlist_label(playlist),
            "Sets / House / Opening"
        );
        assert_eq!(
            app.tree_nodes(Target::Library)
                .iter()
                .map(|(n, depth)| (n.id(), *depth))
                .collect::<Vec<_>>(),
            [(sets, 0), (house, 1), (playlist, 2)]
        );
        let _ = app.update(Message::ToggleFolder(sets));
        assert_eq!(app.tree_nodes(Target::Library).len(), 1);
        let _ = app.update(Message::SelectFolder(Some(sets)));
        assert_eq!(app.tree_nodes(Target::Library).len(), 3);
        assert_eq!(app.export_ids(), [playlist]);
        let _ = app.update(Message::SelectFolder(None));
        let _ = app.update(Message::NewName("Root playlist".into()));
        let _ = app.update(Message::CreatePlaylist);
        assert_eq!(
            app.workspace.library.playlists().last().unwrap().parent,
            None
        );
    }
}
