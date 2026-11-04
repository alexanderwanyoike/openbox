pub(super) fn new_renderer() -> iced::Renderer {
    iced_tiny_skia::Renderer::new(iced::Font::DEFAULT, 16.0.into())
}

pub(super) fn backend(renderer: &mut iced::Renderer) -> &mut iced_tiny_skia::Renderer {
    renderer
}
