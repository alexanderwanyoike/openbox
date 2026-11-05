use super::icons::{self, Icon};
use super::view::{action, nav, quiet};
use super::*;
use iced::{
    Alignment, Element, Length,
    widget::{
        Column, Space, button, column, container, row, scrollable, text, text_input, tooltip,
    },
};

impl App {
    pub(super) fn sidebar(&self) -> Element<'static, Message> {
        let colors = self.preferences.palette();
        let mut tree = Column::new().spacing(3);
        for target in [Target::Library, Target::Usb] {
            let collection = self.workspace.collection(target);
            let count = collection.map_or(0, |l| l.tracks().len());
            let title = if target == Target::Library {
                "Collection"
            } else {
                "Devices"
            };
            tree = tree.push(container(text(title).size(11).color(colors.muted)).padding([10, 9]));
            let selected_collection = self.target == target
                && self.selected_playlist.is_none()
                && self.selected_folder.is_none();
            tree = tree.push(
                container(
                    button(
                        row![
                            icons::themed(
                                if target == Target::Library {
                                    Icon::Library
                                } else {
                                    Icon::Usb
                                },
                                true
                            ),
                            text(if target == Target::Library {
                                "All tracks"
                            } else {
                                "USB collection"
                            })
                            .size(12)
                            .width(Length::Fill),
                            text(count).size(10).color(colors.muted),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    )
                    .padding([8, 10])
                    .width(Length::Fill)
                    .style(move |theme, status| nav(theme, status, selected_collection))
                    .on_press(Message::Switch(target)),
                )
                .id(if target == Target::Library {
                    "collection-row"
                } else {
                    "device-row"
                }),
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
                        icons::themed(
                            if matches!(node, PlaylistNode::Folder(_)) {
                                Icon::Folder
                            } else {
                                Icon::Playlist
                            },
                            true
                        ),
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
                        text(if self.usb_recovery {
                            "USB save interrupted."
                        } else {
                            "Connect a DJ USB to browse its playlists."
                        })
                        .size(11)
                        .color(colors.muted),
                    )
                    .padding(10),
                );
            }
            if target == Target::Usb && self.usb_recovery {
                tree = tree.push(
                    container(action("Recover USB", Message::RecoverUsb, !self.busy))
                        .padding([0, 10])
                        .id("usb-recovery"),
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
            tooltip(
                text(location.clone())
                    .size(10)
                    .color(colors.muted)
                    .wrapping(iced::widget::text::Wrapping::None)
                    .height(16),
                text(location.clone()),
                tooltip::Position::Top
            ),
            row![
                text_input("New name", &self.new_name)
                    .size(11)
                    .padding(8)
                    .style(super::view::input)
                    .width(Length::Fill)
                    .on_input(Message::NewName)
                    .on_submit(Message::CreatePlaylist),
                icons::hint(
                    icons::action(Icon::Playlist, Message::CreatePlaylist, can_create),
                    "Create playlist in this location"
                ),
                icons::hint(
                    icons::action(Icon::Folder, Message::CreateFolder, can_create),
                    "Create folder in this location"
                ),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        ]
        .spacing(7);
        container(column![
            scrollable(tree.padding([8, 9]))
                .direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::hidden()
                ))
                .id("playlist-tree")
                .height(Length::Fill),
            container(create).padding([8, super::surfaces::INSET]),
            row![
                icons::hint(
                    icons::action(
                        Icon::Refresh,
                        Message::RefreshUsb,
                        self.usb_root.is_some() && !self.busy
                    ),
                    "Reread the USB library from disk"
                ),
                action(
                    "Undo",
                    Message::UndoUsb,
                    self.undo_usb.is_some() && !self.busy
                )
                .style(quiet),
                Space::new().width(Length::Fill),
            ]
            .spacing(6)
            .padding([0, super::surfaces::INSET]),
            Space::new().height(12),
        ])
        .id("collection-panel")
        .width(212)
        .height(Length::Fill)
        .style(move |_| super::surfaces::workspace(colors))
        .into()
    }
}
