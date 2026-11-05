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
