pub(super) fn new_renderer() -> iced::Renderer {
    let software = iced_tiny_skia::Renderer::new(iced::Font::DEFAULT, 16.0.into());
    #[cfg(feature = "gpu")]
    { iced::Renderer::Secondary(software) }
    #[cfg(not(feature = "gpu"))]
    { software }
}

pub(super) fn backend(renderer: &mut iced::Renderer) -> &mut iced_tiny_skia::Renderer {
    #[cfg(feature = "gpu")]
    { match renderer { iced::Renderer::Secondary(software) => software, _ => unreachable!("Tests explicitly use the software renderer") } }
    #[cfg(not(feature = "gpu"))]
    { renderer }
}
