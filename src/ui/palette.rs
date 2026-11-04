use iced::{Color, Theme, theme::Base};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Light,
    #[default]
    Dark,
}
impl Mode {
    pub const ALL: [Self; 2] = [Self::Light, Self::Dark];
}
impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Material {
    #[default]
    Aluminium,
    Ceramic,
    Slate,
    Graphite,
    Paper,
    Copper,
    Carbon,
}
impl Material {
    pub const ALL: [Self; 7] = [
        Self::Aluminium,
        Self::Ceramic,
        Self::Slate,
        Self::Graphite,
        Self::Paper,
        Self::Copper,
        Self::Carbon,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Aluminium => "Aluminium",
            Self::Ceramic => "Ceramic",
            Self::Slate => "Slate",
            Self::Graphite => "Graphite",
            Self::Paper => "Paper",
            Self::Copper => "Copper",
            Self::Carbon => "Carbon",
        }
    }
}
impl fmt::Display for Material {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Accent {
    #[default]
    Blue,
    Cobalt,
    Emerald,
    Vermilion,
    Violet,
    Amber,
}
impl Accent {
    pub const ALL: [Self; 6] = [
        Self::Blue,
        Self::Cobalt,
        Self::Emerald,
        Self::Vermilion,
        Self::Violet,
        Self::Amber,
    ];
    pub fn color(self, mode: Mode) -> Color {
        let hex = match (self, mode) {
            (Self::Blue, Mode::Light) => 0x0066eb,
            (Self::Blue, Mode::Dark) => 0x328dff,
            (Self::Cobalt, Mode::Light) => 0x1554ed,
            (Self::Cobalt, Mode::Dark) => 0x427eff,
            (Self::Emerald, Mode::Light) => 0x008341,
            (Self::Emerald, Mode::Dark) => 0x00d87b,
            (Self::Vermilion, Mode::Light) => 0xbf400b,
            (Self::Vermilion, Mode::Dark) => 0xff701f,
            (Self::Violet, Mode::Light) => 0x9421d9,
            (Self::Violet, Mode::Dark) => 0xc627ff,
            (Self::Amber, Mode::Light) => 0x9d6200,
            (Self::Amber, Mode::Dark) => 0xffbc0a,
        };
        rgb(hex)
    }
}
impl fmt::Display for Accent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Blue => "Blue",
            Self::Cobalt => "Cobalt",
            Self::Emerald => "Emerald",
            Self::Vermilion => "Vermilion",
            Self::Violet => "Violet",
            Self::Amber => "Amber",
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Palette {
    pub bg: Color,
    pub panel: Color,
    pub raised: Color,
    pub line: Color,
    pub ink: Color,
    pub muted: Color,
    pub accent: Color,
    pub selected: Color,
    pub deck_b: Color,
    pub accent_text: Color,
    pub danger: Color,
    pub success: Color,
}

pub(super) fn compose(mode: Mode, material: Material, accent: Accent) -> Palette {
    let (bg, panel, raised, line) = match (material, mode) {
        (Material::Aluminium, Mode::Light) => (0xd7dce3, 0xf8f9fb, 0xe7ebf0, 0xb8c1ce),
        (Material::Aluminium, Mode::Dark) => (0x0d1015, 0x1b2028, 0x2b3340, 0x414c5e),
        (Material::Ceramic, Mode::Light) => (0xdedbd4, 0xfffdf8, 0xebe7df, 0xc4bbac),
        (Material::Ceramic, Mode::Dark) => (0x12110e, 0x222019, 0x322e23, 0x4d4736),
        (Material::Slate, Mode::Light) => (0xcbd6e3, 0xf5f9ff, 0xe2eaf5, 0xa8b8cc),
        (Material::Slate, Mode::Dark) => (0x0b121d, 0x172437, 0x263951, 0x3b5475),
        (Material::Graphite, Mode::Light) => (0xd3d5d9, 0xfafafa, 0xe6e7eb, 0xb6bbc4),
        (Material::Graphite, Mode::Dark) => (0x0c0d10, 0x1b1d23, 0x2a2e37, 0x414754),
        (Material::Paper, Mode::Light) => (0xe2dbcd, 0xfffdf4, 0xeee8d9, 0xc6b89f),
        (Material::Paper, Mode::Dark) => (0x15120c, 0x272115, 0x39301f, 0x57482e),
        (Material::Copper, Mode::Light) => (0xdfcfc4, 0xfff7f0, 0xf0e2d8, 0xc3a68f),
        (Material::Copper, Mode::Dark) => (0x19100b, 0x2d1f16, 0x412d20, 0x65452f),
        (Material::Carbon, Mode::Light) => (0xeeeeee, 0xffffff, 0xeaeaea, 0xd3d3d3),
        (Material::Carbon, Mode::Dark) => (0x000000, 0x111111, 0x242424, 0x292929),
    };
    let dark = mode == Mode::Dark;
    let accent = accent.color(mode);
    let linear = |v: f32| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let brightness =
        0.2126 * linear(accent.r) + 0.7152 * linear(accent.g) + 0.0722 * linear(accent.b);
    Palette {
        bg: rgb(bg),
        panel: rgb(panel),
        raised: rgb(raised),
        line: rgb(line),
        ink: rgb(if dark { 0xf0f2f6 } else { 0x18212d }),
        muted: rgb(if dark { 0xb3bbc7 } else { 0x49525f }),
        accent,
        selected: mix(rgb(panel), accent, if dark { 0.18 } else { 0.12 }),
        deck_b: if dark { rgb(0xffbc0a) } else { rgb(0x9d6200) },
        accent_text: if brightness > 0.179 {
            rgb(0x07101f)
        } else {
            Color::WHITE
        },
        danger: rgb(if dark { 0xff7a83 } else { 0xb42431 }),
        success: rgb(if dark { 0x00d87b } else { 0x008341 }),
    }
}

pub(super) fn theme(mode: Mode, material: Material, accent: Accent) -> Theme {
    let p = compose(mode, material, accent);
    let name = serde_json::to_string(&(mode, material, accent)).unwrap_or_default();
    Theme::custom(
        name,
        iced::theme::Palette {
            background: p.bg,
            text: p.ink,
            primary: p.accent,
            success: p.success,
            warning: p.deck_b,
            danger: p.danger,
        },
    )
}

pub(super) fn palette(theme: &Theme) -> Palette {
    let (mode, material, accent) = serde_json::from_str(theme.name()).unwrap_or_default();
    compose(mode, material, accent)
}

fn rgb(hex: u32) -> Color {
    Color::from_rgb8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}
fn mix(a: Color, b: Color, amount: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * amount,
        g: a.g + (b.g - a.g) * amount,
        b: a.b + (b.b - a.b) * amount,
        a: 1.0,
    }
}
