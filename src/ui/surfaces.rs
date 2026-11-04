use super::appearance::Palette;
use iced::{Background, Border, Color, gradient, widget::container};

pub(super) fn card(p: Palette) -> container::Style {
    container::Style {
        background: Some(p.panel.into()),
        border: Border {
            color: Color { a: 0.4, ..p.line },
            width: 1.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn chrome(p: Palette) -> container::Style {
    let top = blend(p.raised, Color::WHITE, 0.04);
    container::Style {
        background: Some(Background::Gradient(
            gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2))
                .add_stop(0.0, top)
                .add_stop(1.0, p.raised)
                .into(),
        )),
        ..Default::default()
    }
}

pub(super) fn blend(a: Color, b: Color, amount: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * amount,
        g: a.g + (b.g - a.g) * amount,
        b: a.b + (b.b - a.b) * amount,
        a: 1.0,
    }
}
