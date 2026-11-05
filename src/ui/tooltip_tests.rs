use super::*;
use iced::{Length, Padding, Point, mouse, widget::container};

#[test]
fn tooltips_draw_an_opaque_readable_surface() {
    let preferences = appearance::Preferences::default();
    let view = container(
        icons::hint(
            view::action("Reset", Message::ClearSelection(false), true),
            "Reset trim, EQ, channel faders, master and crossfader",
        )
        .delay(std::time::Duration::ZERO),
    )
    .padding(Padding {
        left: 500.0,
        top: 100.0,
        right: 0.0,
        bottom: 0.0,
    })
    .width(Length::Fill)
    .height(Length::Fill);
    let pixels = super::test_render::render_widget(
        view.into(),
        preferences.theme(),
        mouse::Cursor::Available(Point::new(510.0, 110.0)),
    );
    let pixel = pixels.pixel(430, 140).unwrap();
    assert_eq!(
        (pixel.red(), pixel.green(), pixel.blue()),
        (36, 36, 36),
        "Tooltip background is transparent or missing"
    );
}
