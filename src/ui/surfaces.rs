use super::appearance::Palette;
use iced::{Border, Color, widget::container};

pub(super) const INSET: u16 = 16;
pub(super) const CORNER: f32 = 12.0;
pub(super) const CONTROL_CORNER: f32 = 6.0;
pub(super) const CONTROL_HEIGHT: u32 = 32;
pub(super) const GAP: u32 = 8;

pub(super) fn workspace(p: Palette) -> container::Style {
    let mut style = card(p);
    style.border.width = 0.0;
    style
}

pub(super) fn footer(p: Palette, left_edge: bool, right_edge: bool) -> container::Style {
    container::Style {
        background: Some(p.raised.into()),
        border: Border {
            radius: iced::border::Radius {
                top_left: 0.0,
                top_right: 0.0,
                bottom_left: if left_edge { CORNER } else { 0.0 },
                bottom_right: if right_edge { CORNER } else { 0.0 },
            },
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(super) fn card(p: Palette) -> container::Style {
    container::Style {
        background: Some(p.panel.into()),
        border: Border {
            color: Color { a: 0.4, ..p.line },
            width: 1.0,
            radius: CORNER.into(),
        },
        ..Default::default()
    }
}

pub(super) fn chrome(p: Palette) -> container::Style {
    container::Style {
        background: Some(p.bg.into()),
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
