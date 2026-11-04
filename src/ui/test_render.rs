use super::App;
use iced::{Rectangle, Size, mouse};
use iced_graphics::core::Renderer as _;
use iced_runtime::user_interface::{Cache, UserInterface};

pub(super) fn render(app: &App, cursor: mouse::Cursor) -> tiny_skia::Pixmap {
    let bounds = Rectangle::with_size(Size::new(1440.0, 900.0));
    let mut renderer = super::rendering::new_renderer();
    let mut ui = UserInterface::build(app.view(), bounds.size(), Cache::new(), &mut renderer);
    renderer.reset(bounds);
    ui.draw(&mut renderer, &app.theme(), &Default::default(), cursor);
    let mut pixels = tiny_skia::Pixmap::new(1440, 900).unwrap();
    let mut mask = tiny_skia::Mask::new(1440, 900).unwrap();
    let viewport = iced_graphics::Viewport::with_physical_size(Size::new(1440, 900), 1.0);
    super::rendering::backend(&mut renderer).draw(&mut pixels.as_mut(), &mut mask, &viewport, &[bounds], app.theme().palette().background);
    pixels
}
