use super::icons::{self, Icon};
use super::*;
use browser::ROW_HEIGHT;
use iced::{
    Alignment, Border, Color, Element, Font, Length, Theme,
    widget::{
        Column, Row, Space, button, checkbox, column, container, lazy, opaque, pick_list,
        progress_bar, row, rule::horizontal, scrollable, stack, text, text_input, tooltip,
    },
};

impl App {
    pub fn theme(&self) -> Theme {
        self.preferences.theme()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let colors = self.preferences.palette();
        let ready = if self.busy { "WORKING" } else { "READY" };
        let mut status = row![
            text(ready)
                .size(9)
                .font(Font::MONOSPACE)
                .color(if self.busy {
                    colors.accent
                } else {
                    colors.muted
                }),
            text(&self.status).size(11).color(if self.error {
                colors.danger
            } else {
                colors.muted
            }),
        ]
        .spacing(14)
        .align_y(Alignment::Center);
        status = status.push(Space::new().width(Length::Fill));
        if let Some(progress) = &self.export_progress {
            let phase = match progress.phase {
                ExportPhase::Checking => "Checking",
                ExportPhase::Encoding => "Encoding",
                ExportPhase::Waveforms => "Waveforms",
                ExportPhase::Publishing => "Writing files",
                ExportPhase::Saving => "Saving database",
                ExportPhase::Complete => "Export complete",
            };
            status = status
                .push(
                    text(format!(
                        "{phase}  {}/{}",
                        progress.completed, progress.total
                    ))
                    .size(10)
                    .color(colors.muted),
                )
                .push(
                    progress_bar(0.0..=100.0, progress.percent)
                        .length(140)
                        .girth(5),
                )
                .push(
                    text(format!("{:.0}%", progress.percent))
                        .size(11)
                        .font(Font::MONOSPACE)
                        .color(colors.accent)
                        .width(38),
                );
        } else if let Some(progress) = &self.import_progress {
            let percent = progress.completed as f32 / progress.total.max(1) as f32 * 100.0;
            status = status
                .push(
                    text(format!(
                        "Analyzing {}/{}",
                        progress.completed, progress.total
                    ))
                    .size(10)
                    .color(colors.muted),
                )
                .push(progress_bar(0.0..=100.0, percent).length(140).girth(5))
                .push(
                    text(format!("{percent:.0}%"))
                        .size(11)
                        .font(Font::MONOSPACE)
                        .color(colors.accent),
                );
        } else {
            status = status.push(
                text("Shift: range   Ctrl: toggle   Ctrl+A: all   Esc: clear")
                    .size(10)
                    .color(colors.muted),
            );
        }
        let mut body = column![].spacing(8).height(Length::Fill);
        if self.preview.visible {
            body = body.push(self.players());
        }
        body = body.push(
            container(lazy(self.browser_revision, move |_| {
                row![self.sidebar(), self.content()]
                    .spacing(8)
                    .height(Length::Fill)
            }))
            .height(Length::Fill)
            .width(Length::Fill)
            .padding([0, 8]),
        );
        let main = column![
            self.header(),
            horizontal(1),
            body,
            horizontal(1),
            container(status)
                .padding([7, 14])
                .style(move |_| panel(colors.panel))
        ]
        .height(Length::Fill);
        let overlay: Element<'_, Message> = if self.settings_visible {
            opaque(
                container(self.settings())
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .padding(24)
                    .style(|_| {
                        panel(Color {
                            a: 0.45,
                            ..Color::BLACK
                        })
                    }),
            )
            .into()
        } else {
            Space::new().into()
        };
        stack![main, overlay]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn header(&self) -> Element<'_, Message> {
        let colors = self.preferences.palette();
        let usb_name = self
            .usb_root
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "No USB connected".into());
        let (label, message, enabled) = if self.target == Target::Usb {
            (
                "Import playlist",
                Message::ImportUsbPlaylist,
                self.selected_playlist.is_some(),
            )
        } else {
            (
                if self.selected_folder.is_some() && self.selected_playlist.is_none() {
                    "Export folder"
                } else if self.selected_playlist.is_some() {
                    "Export playlist"
                } else {
                    "Export playlists"
                },
                Message::Export,
                self.usb_root.is_some() && !self.export_ids().is_empty(),
            )
        };
        container(
            row![
                text("Openbox").size(18).color(colors.ink),
                Space::new().width(36),
                Space::new().width(24),
                icons::hint(
                    icons::labeled(Icon::Import, "Files", Message::ImportFiles, !self.busy, 85),
                    "Import music files into your library"
                ),
                icons::hint(
                    icons::labeled(
                        Icon::Folder,
                        "Folder",
                        Message::ImportFolder,
                        !self.busy,
                        96
                    ),
                    "Import music from a folder and its subfolders"
                ),
                icons::hint(
                    icons::labeled(
                        Icon::Players,
                        "Players",
                        Message::Preview(preview::Event::Toggle),
                        true,
                        90
                    )
                    .style(if self.preview.visible {
                        primary
                    } else {
                        secondary
                    }),
                    if self.preview.visible {
                        "Hide the decks without stopping playback"
                    } else {
                        "Show two preview decks and the mixer"
                    }
                ),
                icons::hint(
                    icons::labeled(
                        Icon::Analyze,
                        "Analyze",
                        Message::Analyze,
                        !self.busy
                            && self.target == Target::Library
                            && !self.workspace.library.tracks().is_empty(),
                        112
                    ),
                    if self.tracks.selection.len() == 0 {
                        "Detect BPM, beat grid and key for the library"
                    } else {
                        "Detect BPM, beat grid and key for selected tracks"
                    }
                ),
                Space::new().width(Length::Fill),
                text(usb_name).size(11).color(colors.muted),
                icons::hint(
                    icons::labeled(Icon::Usb, "USB", Message::ChooseUsb, !self.busy, 93),
                    "Open a mounted Rekordbox USB"
                ),
                action(label, message, enabled && !self.busy).style(primary),
                icons::hint(
                    icons::action(Icon::Settings, Message::OpenSettings, true),
                    "Settings: interface and waveform themes"
                ),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding([10, 16])
        .height(50)
        .style(move |_| super::surfaces::chrome(colors))
        .into()
    }

    fn sidebar(&self) -> Element<'static, Message> {
        let colors = self.preferences.palette();
        let mut tree = Column::new().spacing(3);
        for target in [Target::Library, Target::Usb] {
            let collection = self.workspace.collection(target);
            let count = collection.map_or(0, |l| l.tracks().len());
            let title = if target == Target::Library {
                "COLLECTION"
            } else {
                "DEVICES"
            };
            tree = tree.push(
                container(
                    text(title)
                        .size(9)
                        .font(Font::MONOSPACE)
                        .color(colors.muted),
                )
                .padding([12, 10]),
            );
            let selected_collection = self.target == target
                && self.selected_playlist.is_none()
                && self.selected_folder.is_none();
            tree = tree.push(
                button(row![
                    text(if target == Target::Library {
                        "All tracks"
                    } else {
                        "USB collection"
                    })
                    .size(12)
                    .width(Length::Fill),
                    text(count).size(10).color(colors.muted),
                ])
                .padding([8, 10])
                .width(Length::Fill)
                .style(move |theme, status| nav(theme, status, selected_collection))
                .on_press(Message::Switch(target)),
            );
            tree = tree.push(
                row![
                    container(text("Playlists").size(11).color(colors.muted))
                        .padding([7, 10])
                        .width(Length::Fill),
                    action(
                        "Root",
                        Message::OpenFolder(target, None),
                        collection.is_some()
                    )
                    .style(quiet),
                ]
                .align_y(Alignment::Center),
            );
            for (node, depth) in self.tree_nodes(target) {
                let expanded = match target {
                    Target::Library => &self.library_expanded,
                    Target::Usb => &self.usb_expanded,
                };
                let (icon, message, count, selected) = match node {
                    PlaylistNode::Folder(f) => (
                        if expanded.contains(&f.id) {
                            "▾"
                        } else {
                            "▸"
                        },
                        Message::OpenFolder(target, Some(f.id)),
                        String::new(),
                        self.target == target
                            && self.selected_playlist.is_none()
                            && self.selected_folder == Some(f.id),
                    ),
                    PlaylistNode::Playlist(p) => (
                        "",
                        Message::OpenPlaylist(target, p.id),
                        p.track_ids.len().to_string(),
                        self.target == target && self.selected_playlist == Some(p.id),
                    ),
                };
                let label = button(
                    row![
                        text(node.name().to_owned())
                            .size(12)
                            .wrapping(iced::widget::text::Wrapping::None)
                            .width(Length::Fill),
                        text(count).size(10).color(colors.muted),
                    ]
                    .spacing(4),
                )
                .height(28)
                .padding([5, 4])
                .width(Length::Fill)
                .style(move |theme, status| nav(theme, status, selected))
                .on_press(message);
                tree = tree.push(
                    row![
                        Space::new().width((depth as f32 * 12.0).min(72.0)),
                        button(text(icon).size(12))
                            .width(20)
                            .height(28)
                            .padding(0)
                            .style(quiet)
                            .on_press_maybe(
                                matches!(node, PlaylistNode::Folder(_))
                                    .then_some(Message::ToggleTree(target, node.id()))
                            ),
                        label,
                    ]
                    .align_y(Alignment::Center),
                );
            }
            if target == Target::Usb && collection.is_none() {
                tree = tree.push(
                    container(
                        text("Connect a Rekordbox USB to browse its playlists.")
                            .size(11)
                            .color(colors.muted),
                    )
                    .padding(10),
                );
            }
        }
        let location = self
            .selected_folder
            .and_then(|id| self.collection().map(|l| l.playlist_label(id)))
            .unwrap_or_else(|| "Playlist root".into());
        let can_create =
            !self.busy && self.collection().is_some() && !self.new_name.trim().is_empty();
        let create = column![
            text(if self.target == Target::Library {
                "NEW IN COLLECTION"
            } else {
                "NEW ON USB"
            })
            .size(9)
            .font(Font::MONOSPACE)
            .color(colors.muted),
            tooltip(
                text(location.clone())
                    .size(11)
                    .wrapping(iced::widget::text::Wrapping::None)
                    .height(16),
                text(location.clone()),
                tooltip::Position::Top
            ),
            text_input("Playlist or folder name", &self.new_name)
                .size(12)
                .padding(8)
                .on_input(Message::NewName)
                .on_submit(Message::CreatePlaylist),
            row![
                action("+ Playlist", Message::CreatePlaylist, can_create).width(Length::Fill),
                action("+ Folder", Message::CreateFolder, can_create).width(Length::Fill),
            ]
            .spacing(6),
        ]
        .spacing(7);
        container(column![
            scrollable(tree).id("playlist-tree").height(Length::Fill),
            horizontal(1),
            container(create).padding(12),
            row![
                icons::hint(
                    icons::labeled(
                        Icon::Refresh,
                        "Refresh",
                        Message::RefreshUsb,
                        self.usb_root.is_some() && !self.busy,
                        144
                    ),
                    "Reread the USB library from disk"
                ),
                action(
                    "Undo",
                    Message::UndoUsb,
                    self.undo_usb.is_some() && !self.busy
                ),
            ]
            .spacing(6)
            .padding([0, 12]),
            Space::new().height(12),
        ])
        .width(224)
        .height(Length::Fill)
        .style(move |_| super::surfaces::card(colors))
        .into()
    }

    fn content(&self) -> Element<'static, Message> {
        let colors = self.preferences.palette();
        if self.selected_folder.is_some() && self.selected_playlist.is_none() {
            return self.folder_content();
        }
        let mut body = Column::new().spacing(0);
        if self.target == Target::Usb && self.selected_playlist.is_some() {
            body = body.push(
                container(
                    row![
                        text("DEVICE PLAYLIST")
                            .font(Font::MONOSPACE)
                            .size(9)
                            .color(colors.accent),
                        Space::new().width(Length::Fill),
                        action(
                            if self.browse_library {
                                "Hide collection"
                            } else {
                                "Show collection"
                            },
                            Message::BrowseLibrary,
                            true
                        )
                        .style(quiet),
                        text("Select tracks in the collection, then add them to the USB playlist.")
                            .size(11)
                            .color(colors.muted),
                    ]
                    .spacing(16)
                    .align_y(Alignment::Center),
                )
                .padding([8, 14])
                .style(move |_| panel(colors.panel)),
            );
        }
        if self.browse_library && self.target == Target::Usb && self.selected_playlist.is_some() {
            body = body.push(
                row![self.track_pane(true), self.track_pane(false)]
                    .spacing(1)
                    .height(Length::Fill),
            );
        } else {
            body = body.push(self.track_pane(false));
        }
        if self.deletion.is_some() {
            body = body.push(self.delete_panel());
        }
        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn track_pane(&self, source: bool) -> Element<'static, Message> {
        let colors = self.preferences.palette();
        let library = if source {
            Some(&self.workspace.library)
        } else {
            self.collection()
        };
        let browser = self.browser(source);
        let playlist = if source { None } else { self.selected_playlist };
        let title = if source {
            "Collection".into()
        } else if let Some(id) = playlist {
            library.map(|l| l.playlist_label(id)).unwrap_or_default()
        } else if self.target == Target::Library {
            "All tracks".into()
        } else {
            "USB collection".into()
        };
        let count = library.map_or(0, |l| browser.visible(l, playlist).len());
        let heading = row![
            text(title).size(14).width(Length::Fill),
            text(format!("{count} tracks")).size(11).color(colors.muted),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        let query = text_input(
            "Search title, artist, album, genre, BPM, key...",
            &browser.query,
        )
        .size(12)
        .padding(8)
        .on_input(if source {
            Message::SourceSearch
        } else {
            Message::Search
        });
        let mut tools = row![
            query.width(Length::Fill),
            action("Select all", Message::SelectAll(source), count > 0).style(quiet),
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        if !source && playlist.is_some() {
            tools = row![
                text_input("Playlist name", &self.rename_name)
                    .id("playlist-name")
                    .size(12)
                    .padding(8)
                    .width(180)
                    .on_input(Message::RenameName)
                    .on_submit(Message::RenamePlaylist),
                action("Rename", Message::RenamePlaylist, !self.busy),
                action("Delete playlist", Message::DeletePlaylist, !self.busy).style(quiet),
                Space::new().width(Length::Fill),
            ]
            .spacing(6)
            .align_y(Alignment::Center);
        }
        let mut controls = column![heading, tools].spacing(10);
        if !source && playlist.is_some() {
            controls = controls.push(
                row![
                    text_input("Search playlist...", &browser.query)
                        .size(12)
                        .padding(8)
                        .on_input(Message::Search)
                        .width(Length::Fill),
                    action("Select all", Message::SelectAll(false), count > 0).style(quiet),
                ]
                .spacing(6),
            );
        }
        container(column![
            container(controls)
                .padding(14)
                .style(move |_| panel(colors.panel)),
            self.track_table(source),
            self.selection_bar(source),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| super::surfaces::card(colors))
        .into()
    }

    fn track_table(&self, source: bool) -> Element<'static, Message> {
        let colors = self.preferences.palette();
        let browser = self.browser(source);
        let library = if source {
            Some(&self.workspace.library)
        } else {
            self.collection()
        };
        let playlist = if source { None } else { self.selected_playlist };
        let visible = library
            .map(|l| browser.visible(l, playlist))
            .unwrap_or_default();
        let split =
            self.browse_library && self.target == Target::Usb && self.selected_playlist.is_some();
        let mut headings = Row::new().spacing(0).align_y(Alignment::Center);
        let all_selected =
            !visible.is_empty() && visible.iter().all(|t| browser.selection.contains(t.id));
        headings = headings.push(
            container(checkbox(all_selected).size(13).on_toggle(move |value| {
                if value {
                    Message::SelectAll(source)
                } else {
                    Message::ClearSelection(source)
                }
            }))
            .padding([0, 10])
            .width(32),
        );
        for (label, column, width) in columns(split) {
            let indicator = if browser.sort == column {
                if browser.descending { " ▾" } else { " ▴" }
            } else {
                ""
            };
            headings = headings.push(
                button(
                    text(format!("{label}{indicator}"))
                        .size(10)
                        .color(colors.muted),
                )
                .width(width)
                .padding([9, 6])
                .style(quiet)
                .on_press(Message::Sort(source, column)),
            );
        }
        let header = container(headings).style(move |_| panel(colors.panel));
        let mut rows = Column::new().spacing(0);
        if visible.is_empty() {
            rows = rows.push(
                container(
                    column![
                        text(if !browser.query.is_empty() {
                            "No matching tracks"
                        } else if playlist.is_some() {
                            "This playlist is empty"
                        } else if self.target == Target::Usb && !source {
                            "Connect a USB library"
                        } else {
                            "Import your music"
                        })
                        .size(17),
                        text(if playlist.is_some() && self.target == Target::Usb {
                            "Select a range or all tracks in the collection, then add them below."
                        } else {
                            "Import a folder, select tracks, and add the selection to a playlist."
                        })
                        .size(12)
                        .color(colors.muted),
                    ]
                    .spacing(10),
                )
                .padding(24),
            );
        } else {
            let window = browser.window(visible.len());
            rows = rows.push(Space::new().height(window.start as f32 * ROW_HEIGHT));
            for index in window.clone() {
                let track = visible[index];
                let selected = browser.selection.contains(track.id);
                let mut cells = Row::new().spacing(0).align_y(Alignment::Center);
                let bpm = track
                    .details
                    .bpm
                    .map(|b| format!("{b:.1}"))
                    .unwrap_or_default();
                let duration = format!(
                    "{}:{:02}",
                    track.details.duration_seconds / 60,
                    track.details.duration_seconds % 60
                );
                for (_, column, width) in columns(split) {
                    let value = match column {
                        SortColumn::Order => (index + 1).to_string(),
                        SortColumn::Title => track.title.clone(),
                        SortColumn::Artist => track.artist.clone(),
                        SortColumn::Album => track.details.album.clone(),
                        SortColumn::Genre => track.details.genre.clone(),
                        SortColumn::Bpm => bpm.clone(),
                        SortColumn::Key => track.details.key.clone(),
                        SortColumn::Duration => duration.clone(),
                    };
                    let color = if selected || column == SortColumn::Title {
                        colors.ink
                    } else if column == SortColumn::Key {
                        colors.accent
                    } else {
                        colors.muted
                    };
                    let label: Element<'_, Message> = text(value)
                        .size(12)
                        .color(color)
                        .wrapping(iced::widget::text::Wrapping::None)
                        .into();
                    let label: Element<'_, Message> = if column == SortColumn::Title {
                        row![cover(track.details.artwork.as_deref(), 22, colors), label]
                            .spacing(7)
                            .align_y(Alignment::Center)
                            .into()
                    } else {
                        label
                    };
                    cells = cells.push(container(label).width(width).padding([0, 6]));
                }
                let id = track.id;
                let check = container(checkbox(selected).size(13).on_toggle(move |_| {
                    if source {
                        Message::ToggleSource(id)
                    } else {
                        Message::ToggleTrack(id)
                    }
                }))
                .width(32)
                .padding([0, 10]);
                let line = row![
                    check,
                    button(cells)
                        .width(Length::Fill)
                        .height(ROW_HEIGHT)
                        .padding(0)
                        .style(move |theme, status| track_style(
                            theme,
                            status,
                            selected,
                            index % 2 == 1
                        ))
                        .on_press(if source {
                            Message::SelectSource(id)
                        } else {
                            Message::SelectTrack(id)
                        })
                ]
                .height(ROW_HEIGHT)
                .align_y(Alignment::Center);
                rows = rows.push(container(line).style(move |_| {
                    panel(if selected {
                        colors.selected
                    } else if index % 2 == 1 {
                        super::surfaces::blend(colors.panel, colors.raised, 0.24)
                    } else {
                        colors.panel
                    })
                }));
            }
            rows = rows.push(Space::new().height((visible.len() - window.end) as f32 * ROW_HEIGHT));
        }
        column![
            header,
            scrollable(rows)
                .id(if source {
                    "source-tracks"
                } else {
                    "main-tracks"
                })
                .on_scroll(move |v| Message::Scrolled(
                    source,
                    v.absolute_offset().y,
                    v.bounds().height
                ))
                .height(Length::Fill),
        ]
        .height(Length::Fill)
        .into()
    }

    fn selection_bar(&self, source: bool) -> Element<'static, Message> {
        let colors = self.preferences.palette();
        let selection = &self.browser(source).selection;
        let count = selection.len();
        let enabled = count > 0 && !self.busy;
        let mut actions = row![
            text(format!("{count} selected"))
                .size(11)
                .color(if enabled { colors.accent } else { colors.muted })
                .width(84),
            action("Clear", Message::ClearSelection(source), count > 0).style(quiet),
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        if source {
            actions = actions
                .push(Space::new().width(Length::Fill))
                .push(action(
                    "Replace",
                    Message::ReplaceSource,
                    selection.single().is_some()
                        && self.tracks.selection.single().is_some()
                        && !self.busy,
                ))
                .push(action("Add to USB playlist →", Message::AddSource, enabled).style(primary));
        } else {
            let choices: Vec<_> = self
                .collection()
                .map(|l| {
                    l.playlists()
                        .iter()
                        .map(|p| PlaylistChoice {
                            id: p.id,
                            name: l.playlist_label(p.id),
                        })
                        .collect()
                })
                .unwrap_or_default();
            if enabled {
                actions = actions.push(
                    pick_list(choices, None::<PlaylistChoice>, Message::AddTo)
                        .placeholder("Add to playlist...")
                        .text_size(12)
                        .padding(6)
                        .width(180),
                );
            } else {
                actions = actions.push(
                    action("Add to playlist...", Message::SelectAll(false), false).width(180),
                );
            }
            if self.selected_playlist.is_some() {
                let can_order = enabled
                    && self.tracks.sort == SortColumn::Order
                    && self.tracks.query.is_empty();
                actions = actions
                    .push(action("↑", Message::MoveTrack(-1), can_order))
                    .push(action("↓", Message::MoveTrack(1), can_order))
                    .push(action("Remove", Message::RemoveFromPlaylist, enabled));
            } else if self.target == Target::Library {
                actions = actions
                    .push(action(
                        "+ New playlist",
                        Message::CreateFromSelection,
                        enabled,
                    ))
                    .push(action("Remove", Message::RemoveTrack, enabled).style(quiet));
            }
            if self.target == Target::Library
                && enabled
                && let Some(device) = &self.workspace.device
            {
                let choices: Vec<_> = device
                    .library
                    .playlists()
                    .iter()
                    .map(|p| PlaylistChoice {
                        id: p.id,
                        name: device.library.playlist_label(p.id),
                    })
                    .collect();
                if !choices.is_empty() {
                    actions = actions.push(
                        pick_list(choices, None::<PlaylistChoice>, Message::SendTrackToUsb)
                            .placeholder("Send to USB playlist...")
                            .text_size(12)
                            .padding(6)
                            .width(180),
                    );
                }
            }
            actions = actions.push(Space::new().width(Length::Fill));
        }
        container(actions)
            .padding([10, 12])
            .height(48)
            .style(move |_| panel(colors.raised))
            .into()
    }

    fn folder_content(&self) -> Element<'static, Message> {
        let colors = self.preferences.palette();
        let library = self.collection();
        let folder = self.selected_folder.and_then(|id| library?.folder(id));
        let path = folder
            .map(|f| library.unwrap().playlist_label(f.id))
            .unwrap_or_default();
        let parent = folder.and_then(|f| f.parent);
        let mut children = Column::new().spacing(1);
        if let Some(library) = library {
            for node in library.children(self.selected_folder) {
                let (kind, message, count) = match node {
                    PlaylistNode::Folder(f) => (
                        "Folder",
                        Message::OpenFolder(self.target, Some(f.id)),
                        String::new(),
                    ),
                    PlaylistNode::Playlist(p) => (
                        "Playlist",
                        Message::OpenPlaylist(self.target, p.id),
                        format!("{} tracks", p.track_ids.len()),
                    ),
                };
                children = children.push(
                    button(
                        row![
                            text(kind).size(11).color(colors.muted).width(70),
                            text(node.name().to_owned()).size(13).width(Length::Fill),
                            text(count).size(11).color(colors.muted),
                        ]
                        .spacing(12),
                    )
                    .padding([10, 12])
                    .width(Length::Fill)
                    .style(secondary)
                    .on_press(message),
                );
            }
            if library.children(self.selected_folder).is_empty() {
                children = children.push(
                    container(
                        text("Create a playlist or subfolder using the sidebar.")
                            .size(12)
                            .color(colors.muted),
                    )
                    .padding(20),
                );
            }
        }
        container(
            column![
                row![
                    text(path).size(19).width(Length::Fill),
                    action(
                        "Parent folder",
                        Message::OpenFolder(self.target, parent),
                        true
                    )
                ]
                .align_y(Alignment::Center),
                text("FOLDER")
                    .size(10)
                    .font(Font::MONOSPACE)
                    .color(colors.muted),
                horizontal(1),
                scrollable(children).height(Length::Fill),
            ]
            .spacing(12),
        )
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn delete_panel(&self) -> Element<'static, Message> {
        let colors = self.preferences.palette();
        let label = match &self.deletion {
            Some(Deletion::Playlist(_)) => "Delete this playlist?".into(),
            Some(Deletion::Tracks(ids)) => format!(
                "Remove {} tracks from the collection and its playlists?",
                ids.len()
            ),
            None => String::new(),
        };
        container(
            row![
                column![
                    text(label).size(13),
                    text("Audio files will be kept.")
                        .size(11)
                        .color(colors.muted)
                ]
                .spacing(4),
                Space::new().width(Length::Fill),
                action("Cancel", Message::CancelDelete, true),
                action("Confirm", Message::ConfirmDelete, !self.busy).style(button::danger),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding(12)
        .style(move |_| panel(colors.raised))
        .into()
    }
}

fn columns(split: bool) -> Vec<(&'static str, SortColumn, Length)> {
    let mut columns = vec![
        ("#", SortColumn::Order, Length::Fixed(32.0)),
        ("TITLE", SortColumn::Title, Length::FillPortion(4)),
        ("ARTIST", SortColumn::Artist, Length::FillPortion(3)),
    ];
    if !split {
        columns.extend([
            ("ALBUM", SortColumn::Album, Length::FillPortion(2)),
            ("GENRE", SortColumn::Genre, Length::FillPortion(2)),
        ]);
    }
    columns.extend([
        ("BPM", SortColumn::Bpm, Length::Fixed(52.0)),
        ("KEY", SortColumn::Key, Length::Fixed(40.0)),
        ("TIME", SortColumn::Duration, Length::Fixed(48.0)),
    ]);
    columns
}

pub(super) fn action(
    label: &str,
    message: Message,
    enabled: bool,
) -> iced::widget::Button<'_, Message> {
    button(text(label).size(12))
        .padding([6, 10])
        .style(secondary)
        .on_press_maybe(enabled.then_some(message))
}

pub(super) fn panel(color: Color) -> container::Style {
    container::Style {
        background: Some(color.into()),
        ..Default::default()
    }
}

pub(super) fn secondary(theme: &Theme, status: button::Status) -> button::Style {
    let colors = super::appearance::palette(theme);
    button::Style {
        background: Some(
            if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                colors.line
            } else {
                super::surfaces::blend(colors.panel, colors.raised, 0.35)
            }
            .into(),
        ),
        text_color: if status == button::Status::Disabled {
            colors.muted
        } else {
            colors.ink
        },
        border: Border {
            color: Color {
                a: 0.65,
                ..colors.line
            },
            width: 1.0,
            radius: 7.0.into(),
        },
        ..Default::default()
    }
}

fn primary(theme: &Theme, status: button::Status) -> button::Style {
    let colors = super::appearance::palette(theme);
    let mut style = secondary(theme, status);
    if status != button::Status::Disabled {
        style.background = Some(colors.accent.into());
        style.text_color = colors.accent_text;
        style.border.color = colors.accent;
    }
    style
}

pub(super) fn quiet(theme: &Theme, status: button::Status) -> button::Style {
    let colors = super::appearance::palette(theme);
    button::Style {
        text_color: if status == button::Status::Hovered {
            colors.accent
        } else {
            colors.muted
        },
        background: matches!(status, button::Status::Hovered | button::Status::Pressed)
            .then_some(colors.raised.into()),
        ..Default::default()
    }
}

fn nav(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let colors = super::appearance::palette(theme);
    button::Style {
        text_color: colors.ink,
        background: if selected {
            Some(colors.selected.into())
        } else if status == button::Status::Hovered {
            Some(colors.raised.into())
        } else {
            None
        },
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn track_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
    alternate: bool,
) -> button::Style {
    let colors = super::appearance::palette(theme);
    button::Style {
        text_color: colors.ink,
        background: Some(
            if selected {
                colors.selected
            } else if status == button::Status::Hovered {
                colors.raised
            } else if alternate {
                super::surfaces::blend(colors.panel, colors.raised, 0.24)
            } else {
                colors.panel
            }
            .into(),
        ),
        ..Default::default()
    }
}

pub(super) fn cover(
    path: Option<&std::path::Path>,
    size: u32,
    colors: super::appearance::Palette,
) -> Element<'static, Message> {
    if let Some(path) = path {
        iced::widget::image(iced::widget::image::Handle::from_path(path))
            .width(size)
            .height(size)
            .content_fit(iced::ContentFit::Cover)
            .into()
    } else {
        container(text("♫").size((size / 2).max(10)).color(colors.muted))
            .center_x(size)
            .center_y(size)
            .style(move |_| panel(colors.raised))
            .into()
    }
}
