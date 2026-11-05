use super::*;
use iced::{Rectangle, mouse};
use iced_graphics::core::widget::{Id, Operation};
use iced_runtime::user_interface::{Cache, UserInterface};

#[test]
fn split_browser_has_a_straight_internal_seam_and_rounded_outer_footer_corners() {
    struct Sidebar(Option<Rectangle>);
    impl Operation for Sidebar {
        fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
            children(self);
        }
        fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
            if id.is_some_and(|id| *id == Id::new("collection-panel")) {
                self.0 = Some(bounds);
            }
        }
    }
    let folder = tempfile::tempdir().unwrap();
    let root = folder.path().join("usb");
    openbox_test_support::rekordbox_usb(&root);
    let mut app = App::load(folder.path().join("library.json"), Some(root)).unwrap();
    let _ = app.update(Message::OpenPlaylist(Target::Usb, 23));
    for mode in appearance::Mode::ALL {
        app.preferences.mode = mode;
        let mut renderer = super::rendering::new_renderer();
        let mut ui = UserInterface::build(
            app.view(),
            iced::Size::new(1440.0, 900.0),
            Cache::new(),
            &mut renderer,
        );
        let mut sidebar = Sidebar(None);
        ui.operate(&renderer, &mut sidebar);
        let sidebar = sidebar.0.unwrap();
        let pixels = super::test_render::render(&app, mouse::Cursor::Unavailable);
        let pane_left = sidebar.x + sidebar.width + 8.0;
        let pane_right = 1432.0;
        let seam = (pane_left + pane_right) * 0.5;
        let pixel = pixels
            .pixel((seam - 3.0) as u32, (sidebar.y + 45.0) as u32)
            .unwrap();
        let panel = app.preferences.palette().panel;
        assert_eq!(
            (pixel.red(), pixel.green(), pixel.blue()),
            (
                (panel.r * 255.0).round() as u8,
                (panel.g * 255.0).round() as u8,
                (panel.b * 255.0).round() as u8
            ),
            "{mode:?}: touching rounded cards leave a notch inside the browser"
        );
        let bottom = (sidebar.y + sidebar.height - 1.0) as u32;
        assert_eq!(
            pixels.pixel(1431, bottom),
            pixels.pixel(1439, bottom),
            "{mode:?}: the action bar must follow the outer rounded corner"
        );
    }
}

#[test]
fn shared_playlist_toolbar_accepts_name_edits() {
    use iced_graphics::core::widget::operation::Focusable;
    struct FocusName;
    impl Operation for FocusName {
        fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
            children(self);
        }
        fn focusable(&mut self, id: Option<&Id>, _: Rectangle, state: &mut dyn Focusable) {
            if id.is_some_and(|id| *id == Id::new("playlist-name")) {
                state.focus();
            }
        }
    }
    let folder = tempfile::tempdir().unwrap();
    let root = folder.path().join("usb");
    openbox_test_support::rekordbox_usb(&root);
    let mut app = App::load(folder.path().join("library.json"), Some(root)).unwrap();
    let _ = app.update(Message::OpenPlaylist(Target::Usb, 23));
    let mut renderer = super::rendering::new_renderer();
    let mut ui = UserInterface::build(
        app.view(),
        iced::Size::new(1440.0, 900.0),
        Cache::new(),
        &mut renderer,
    );
    ui.operate(&renderer, &mut FocusName);
    let mut messages = Vec::new();
    let _ = ui.update(
        &[iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Character("x".into()),
            modified_key: keyboard::Key::Character("x".into()),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyX),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: Some("x".into()),
            repeat: false,
        })],
        mouse::Cursor::Unavailable,
        &mut renderer,
        &mut iced_graphics::core::clipboard::Null,
        &mut messages,
    );
    assert!(
        messages
            .iter()
            .any(|m| matches!(m,Message::RenameName(name) if name.contains('x'))),
        "The playlist name field must remain editable after moving into the shared toolbar"
    );
}

#[test]
fn selected_playlist_tracks_can_be_removed_at_the_minimum_window_width() {
    struct Sidebar(Option<Rectangle>);
    impl Operation for Sidebar {
        fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
            children(self);
        }
        fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
            if id.is_some_and(|id| *id == Id::new("collection-panel")) {
                self.0 = Some(bounds);
            }
        }
    }
    let folder = tempfile::tempdir().unwrap();
    let root = folder.path().join("usb");
    openbox_test_support::rekordbox_usb(&root);
    let mut app = App::load(folder.path().join("library.json"), Some(root)).unwrap();
    let _ = app.update(Message::OpenPlaylist(Target::Usb, 23));
    let _ = app.update(Message::ToggleTrack(42));
    let mut renderer = super::rendering::new_renderer();
    let mut ui = UserInterface::build(
        app.view(),
        iced::Size::new(1000.0, 900.0),
        Cache::new(),
        &mut renderer,
    );
    let mut sidebar = Sidebar(None);
    ui.operate(&renderer, &mut sidebar);
    let bounds = sidebar.0.unwrap();
    let cursor = mouse::Cursor::Available(iced::Point::new(961.0, bounds.y + bounds.height - 24.0));
    let mut messages = Vec::new();
    let _ = ui.update(
        &[
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
        cursor,
        &mut renderer,
        &mut iced_graphics::core::clipboard::Null,
        &mut messages,
    );
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Message::RemoveFromPlaylist)),
        "The remove action must be visible and clickable inside a 1000 px window"
    );
}

#[test]
fn delete_confirmation_preserves_the_browsers_outer_corners() {
    let folder = tempfile::tempdir().unwrap();
    let mut app = App::load(folder.path().join("library.json"), None).unwrap();
    let playlist = app.workspace.library.create_playlist("Local set").unwrap();
    let _ = app.update(Message::OpenPlaylist(Target::Library, playlist));
    let _ = app.update(Message::DeletePlaylist);
    assert!(app.deletion.is_some());
    for mode in appearance::Mode::ALL {
        app.preferences.mode = mode;
        let pixels = super::test_render::render(&app, mouse::Cursor::Unavailable);
        assert_eq!(
            pixels.pixel(1431, 861),
            pixels.pixel(1439, 861),
            "{mode:?}: confirmation must not square off the browser frame"
        );
    }
}
