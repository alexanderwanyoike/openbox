use super::{
    App, Message, Target,
    icons::{self, Icon},
    preview, view,
};
use iced::{
    Alignment, Border, Color, Element, Length, Task,
    widget::{Space, column, container, mouse_area, row, text},
    window,
};
use iced_aw::menu::{Item, Menu, MenuBar};

#[derive(Debug, Clone, Copy)]
pub enum WindowAction {
    Drag,
    Minimize,
    Maximize,
    Close,
    Resize,
}

pub(super) fn window_action(action: WindowAction) -> Task<Message> {
    window::latest().then(move |id| {
        id.map_or_else(Task::none, |id| match action {
            WindowAction::Drag => window::drag(id),
            WindowAction::Minimize => window::minimize(id, true),
            WindowAction::Maximize => window::toggle_maximize(id),
            WindowAction::Close => window::close(id),
            WindowAction::Resize => window::drag_resize(id, window::Direction::SouthEast),
        })
    })
}

impl App {
    pub(super) fn header(&self) -> Element<'_, Message> {
        let p = self.preferences.palette();
        let caption = self.library_name.as_str();
        let title = mouse_area(
            container(text(caption).size(11).color(p.ink))
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .on_press(Message::Window(WindowAction::Drag))
        .on_double_click(Message::Window(WindowAction::Maximize));
        let chrome = container(
            row![
                row![
                    icons::themed(Icon::Brand, true),
                    text("Openbox").size(13).font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..iced::Font::default()
                    })
                ]
                .spacing(7)
                .align_y(Alignment::Center),
                self.application_menu(),
                title,
                icons::hint(
                    icons::action(Icon::Settings, Message::OpenSettings, true),
                    "Settings"
                ),
                icons::hint(
                    icons::action(
                        Icon::Minimize,
                        Message::Window(WindowAction::Minimize),
                        true
                    )
                    .width(32)
                    .height(48)
                    .padding([16, 8]),
                    "Minimize"
                ),
                icons::hint(
                    icons::action(
                        Icon::Maximize,
                        Message::Window(WindowAction::Maximize),
                        true
                    )
                    .width(32)
                    .height(48)
                    .padding([16, 8]),
                    "Maximize or restore"
                ),
                icons::hint(
                    icons::action(Icon::Close, Message::Window(WindowAction::Close), true),
                    "Close Openbox"
                ),
            ]
            .spacing(3)
            .align_y(Alignment::Center),
        )
        .id("application-chrome")
        .padding(iced::Padding {
            left: 12.0,
            right: 0.0,
            top: 0.0,
            bottom: 0.0,
        })
        .height(48)
        .style(move |_| super::surfaces::chrome(p));
        column![chrome, self.workspace_toolbar()].into()
    }

    fn application_menu(&self) -> Element<'_, Message> {
        let menu = |label, entries| {
            Item::with_menu(
                container(text(label).size(11)).padding([6, 8]),
                Menu::new(entries).width(230.0).spacing(2.0),
            )
        };
        let item = |label, message, enabled| {
            Item::new(
                view::action(label, message, enabled)
                    .width(Length::Fill)
                    .padding([8, 10])
                    .style(view::quiet),
            )
        };
        MenuBar::new(vec![
            menu(
                "File",
                vec![
                    item("Import files...", Message::ImportFiles, !self.busy),
                    item("Import folder...", Message::ImportFolder, !self.busy),
                    item("Open USB...", Message::ChooseUsb, !self.busy),
                    item("Quit", Message::Window(WindowAction::Close), true),
                ],
            ),
            menu(
                "Edit",
                vec![
                    item("Select all", Message::SelectAll(self.source_focused), true),
                    item(
                        "Clear selection",
                        Message::ClearSelection(self.source_focused),
                        true,
                    ),
                    item(
                        "Undo USB edit",
                        Message::UndoUsb,
                        !self.busy && self.undo_usb.is_some(),
                    ),
                ],
            ),
            menu(
                "View",
                vec![
                    item(
                        if self.preview.visible {
                            "Hide players"
                        } else {
                            "Show players"
                        },
                        Message::Preview(preview::Event::Toggle),
                        true,
                    ),
                    item("Settings...", Message::OpenSettings, true),
                ],
            ),
        ])
        .spacing(1.0)
        .style(|theme, _| {
            let p = super::appearance::palette(theme);
            iced_aw::style::menu_bar::Style {
                bar_background: Color::TRANSPARENT.into(),
                bar_border: Border::default(),
                menu_background: p.raised.into(),
                menu_border: Border {
                    color: p.line,
                    width: 1.0,
                    radius: 9.0.into(),
                },
                menu_shadow: Default::default(),
                path: p.selected.into(),
                ..Default::default()
            }
        })
        .into()
    }

    fn workspace_toolbar(&self) -> Element<'_, Message> {
        let p = self.preferences.palette();
        let name = if let Some(id) = self.selected_playlist {
            self.collection()
                .map(|l| l.playlist_label(id))
                .unwrap_or_default()
        } else if let Some(id) = self.selected_folder {
            self.collection()
                .map(|l| l.playlist_label(id))
                .unwrap_or_default()
        } else if self.target == Target::Usb {
            "USB collection".into()
        } else {
            "Collection".into()
        };
        let count = self.collection().map_or(0, |l| l.tracks().len());
        let export = self.target == Target::Library;
        let players_visible = self.preview.visible;
        let enabled = !self.busy
            && if export {
                self.usb_root.is_some() && !self.export_ids().is_empty()
            } else {
                self.selected_playlist.is_some()
            };
        container(
            row![
                icons::themed(if export { Icon::Library } else { Icon::Usb }, true),
                text(name).size(12).width(Length::Fill),
                text(format!("{count} tracks")).size(11).color(p.muted),
                icons::hint(
                    icons::action(Icon::Import, Message::ImportFiles, !self.busy),
                    "Import music files"
                ),
                icons::hint(
                    icons::action(Icon::Folder, Message::ImportFolder, !self.busy),
                    "Import a music folder"
                ),
                icons::hint(
                    icons::action(
                        Icon::Analyze,
                        Message::Analyze,
                        !self.busy && export && count > 0
                    ),
                    "Analyze BPM, grid and key"
                ),
                icons::hint(
                    icons::action(
                        Icon::Players,
                        Message::Preview(preview::Event::Toggle),
                        true
                    )
                    .style(move |theme, status| {
                        let mut style = view::quiet(theme, status);
                        if players_visible {
                            style.background = Some(p.selected.into());
                            style.text_color = p.accent;
                        }
                        style
                    }),
                    "Show or hide preview decks"
                ),
                container(Space::new().width(1).height(20)).style(move |_| view::panel(p.line)),
                icons::hint(
                    icons::action(Icon::Usb, Message::ChooseUsb, !self.busy),
                    "Open a mounted Rekordbox USB"
                ),
                icons::hint(
                    icons::labeled(
                        if export { Icon::Forward } else { Icon::Import },
                        if export { "Export" } else { "Import" },
                        if export {
                            Message::Export
                        } else {
                            Message::ImportUsbPlaylist
                        },
                        enabled,
                        86
                    )
                    .style(view::quiet),
                    if export {
                        "Export selected playlists to USB"
                    } else {
                        "Import USB playlist into collection"
                    }
                ),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .id("workspace-toolbar")
        .padding([0, 18])
        .center_y(48)
        .style(move |_| super::surfaces::chrome(p))
        .into()
    }
}
