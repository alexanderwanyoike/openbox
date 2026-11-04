pub const APPLICATION_ID: &str = "io.github.alexanderwanyoike.openbox";

use iced::window::icon::{self, Icon};

pub fn window_icon() -> Result<Icon, String> {
    let image = image::load_from_memory(include_bytes!("../resources/icons/openbox-256.png"))
        .map_err(|error| format!("Cannot decode application icon: {error}"))?
        .into_rgba8();
    let (width, height) = image.dimensions();
    icon::from_rgba(image.into_raw(), width, height).map_err(|error| error.to_string())
}
