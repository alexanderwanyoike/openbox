use iced::theme::Base;
use iced::{Color, Theme};
use serde::{Deserialize, Serialize};
use std::{fmt, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiTheme {
    Obsidian,
    Berlin,
    Acid,
    Vaporwave,
    Nord,
    Dracula,
    Gruvbox,
    TokyoNight,
    Daylight,
    #[default]
    #[serde(other)]
    Openbox,
}

impl UiTheme {
    pub const ALL: [Self; 10] = [
        Self::Openbox,
        Self::Obsidian,
        Self::Berlin,
        Self::Acid,
        Self::Vaporwave,
        Self::Nord,
        Self::Dracula,
        Self::Gruvbox,
        Self::TokyoNight,
        Self::Daylight,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Openbox => "Openbox",
            Self::Obsidian => "Obsidian",
            Self::Berlin => "Berlin",
            Self::Acid => "Acid",
            Self::Vaporwave => "Vaporwave",
            Self::Nord => "Nord",
            Self::Dracula => "Dracula",
            Self::Gruvbox => "Gruvbox",
            Self::TokyoNight => "Tokyo Night",
            Self::Daylight => "Daylight",
        }
    }

    pub(super) fn palette(self) -> Palette {
        let (bg, ink, accent, deck_b) = match self {
            Self::Openbox => (
                rgb(19, 22, 26),
                rgb(214, 222, 232),
                rgb(64, 166, 250),
                rgb(242, 171, 74),
            ),
            Self::Obsidian => (
                rgb(10, 10, 12),
                rgb(222, 226, 230),
                rgb(125, 200, 255),
                rgb(240, 168, 90),
            ),
            Self::Berlin => (
                rgb(6, 6, 6),
                rgb(215, 215, 215),
                rgb(235, 95, 80),
                rgb(218, 187, 141),
            ),
            Self::Acid => (
                rgb(8, 10, 6),
                rgb(214, 230, 202),
                rgb(170, 255, 0),
                rgb(255, 150, 55),
            ),
            Self::Vaporwave => (
                rgb(24, 14, 42),
                rgb(230, 220, 245),
                rgb(255, 113, 206),
                rgb(1, 205, 254),
            ),
            Self::Nord => (
                rgb(46, 52, 64),
                rgb(216, 222, 233),
                rgb(136, 192, 208),
                rgb(235, 203, 139),
            ),
            Self::Dracula => (
                rgb(40, 42, 54),
                rgb(248, 248, 242),
                rgb(189, 147, 249),
                rgb(255, 184, 108),
            ),
            Self::Gruvbox => (
                rgb(40, 40, 40),
                rgb(235, 219, 178),
                rgb(254, 128, 25),
                rgb(131, 165, 152),
            ),
            Self::TokyoNight => (
                rgb(26, 27, 38),
                rgb(192, 202, 245),
                rgb(122, 162, 247),
                rgb(255, 158, 100),
            ),
            Self::Daylight => (
                rgb(241, 243, 247),
                rgb(31, 40, 54),
                rgb(22, 97, 173),
                rgb(153, 77, 14),
            ),
        };
        let surface = if self == Self::Daylight {
            Color::BLACK
        } else {
            Color::WHITE
        };
        Palette {
            bg,
            ink,
            accent,
            deck_b,
            panel: mix(bg, surface, 0.025),
            raised: mix(bg, surface, 0.055),
            line: mix(bg, surface, 0.16),
            muted: mix(bg, ink, 0.64),
            selected: mix(bg, accent, 0.23),
            accent_text: if self == Self::Daylight {
                Color::WHITE
            } else {
                rgb(12, 14, 18)
            },
            danger: if self == Self::Daylight {
                rgb(180, 36, 49)
            } else {
                rgb(245, 111, 119)
            },
            success: if self == Self::Daylight {
                rgb(21, 127, 65)
            } else {
                rgb(76, 199, 127)
            },
        }
    }

    pub fn theme(self) -> Theme {
        let p = self.palette();
        Theme::custom(
            self.name(),
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
}

impl fmt::Display for UiTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
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

pub(super) fn palette(theme: &Theme) -> Palette {
    UiTheme::ALL
        .into_iter()
        .find(|t| t.name() == theme.name())
        .unwrap_or_default()
        .palette()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaveformTheme {
    Spectrum,
    Ice,
    Ember,
    Acid,
    Violet,
    Nord,
    Dracula,
    Mono,
    Paper,
    #[default]
    #[serde(other)]
    Classic,
}

impl WaveformTheme {
    pub const ALL: [Self; 10] = [
        Self::Classic,
        Self::Spectrum,
        Self::Ice,
        Self::Ember,
        Self::Acid,
        Self::Violet,
        Self::Nord,
        Self::Dracula,
        Self::Mono,
        Self::Paper,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Classic => "Classic",
            Self::Spectrum => "Spectrum",
            Self::Ice => "Ice",
            Self::Ember => "Ember",
            Self::Acid => "Acid",
            Self::Violet => "Violet",
            Self::Nord => "Nord",
            Self::Dracula => "Dracula",
            Self::Mono => "Monochrome",
            Self::Paper => "Paper",
        }
    }
    pub(super) fn palette(self) -> WavePalette {
        let (bg, low, mid, high, past) = match self {
            Self::Classic => (
                rgb(6, 8, 11),
                rgb(10, 99, 247),
                rgb(255, 158, 18),
                rgb(255, 237, 194),
                rgb(224, 133, 20),
            ),
            Self::Spectrum => (
                rgb(7, 10, 14),
                rgb(235, 65, 86),
                rgb(63, 204, 132),
                rgb(83, 167, 255),
                rgb(116, 88, 121),
            ),
            Self::Ice => (
                rgb(7, 17, 24),
                rgb(32, 108, 214),
                rgb(50, 205, 222),
                rgb(217, 251, 255),
                rgb(60, 116, 147),
            ),
            Self::Ember => (
                rgb(22, 9, 7),
                rgb(217, 51, 38),
                rgb(255, 135, 32),
                rgb(255, 227, 151),
                rgb(132, 71, 53),
            ),
            Self::Acid => (
                rgb(7, 13, 7),
                rgb(40, 155, 61),
                rgb(164, 240, 32),
                rgb(247, 255, 166),
                rgb(76, 106, 43),
            ),
            Self::Violet => (
                rgb(16, 9, 25),
                rgb(124, 68, 217),
                rgb(235, 105, 221),
                rgb(255, 218, 251),
                rgb(121, 81, 149),
            ),
            Self::Nord => (
                rgb(28, 34, 44),
                rgb(94, 129, 172),
                rgb(136, 192, 208),
                rgb(236, 239, 244),
                rgb(129, 141, 157),
            ),
            Self::Dracula => (
                rgb(24, 25, 34),
                rgb(189, 147, 249),
                rgb(80, 250, 123),
                rgb(255, 184, 108),
                rgb(98, 114, 164),
            ),
            Self::Mono => (
                rgb(7, 8, 10),
                rgb(119, 131, 145),
                rgb(184, 193, 203),
                rgb(244, 246, 249),
                rgb(79, 91, 105),
            ),
            Self::Paper => (
                rgb(244, 242, 235),
                rgb(24, 74, 139),
                rgb(174, 92, 23),
                rgb(101, 49, 114),
                rgb(137, 149, 163),
            ),
        };
        let light = self == Self::Paper;
        WavePalette {
            bg,
            bands: [low, mid, high],
            past,
            text: if light {
                rgb(53, 64, 80)
            } else {
                rgb(164, 180, 200)
            },
            tick: if light {
                rgb(111, 121, 136)
            } else {
                rgb(91, 106, 125)
            },
            bar: if light {
                rgb(176, 38, 46)
            } else {
                rgb(246, 77, 81)
            },
            playhead: if light {
                rgb(16, 22, 31)
            } else {
                rgb(247, 247, 237)
            },
            marker: if light {
                rgb(187, 89, 0)
            } else {
                rgb(255, 153, 31)
            },
        }
    }
}
impl fmt::Display for WaveformTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct WavePalette {
    pub bg: Color,
    pub bands: [Color; 3],
    pub past: Color,
    pub text: Color,
    pub tick: Color,
    pub bar: Color,
    pub playhead: Color,
    pub marker: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Preferences {
    pub ui_theme: UiTheme,
    pub waveform_theme: WaveformTheme,
}
impl Preferences {
    pub fn load(path: &Path) -> Result<Self, String> {
        match std::fs::read(path) {
            Ok(bytes) => {
                serde_json::from_slice(&bytes).map_err(|e| format!("Cannot read settings: {e}"))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("Cannot read settings: {e}")),
        }
    }
    pub fn save(self, path: &Path) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(&self).map_err(|e| e.to_string())?;
        openbox_storage::write_atomic(path, &bytes)
            .map_err(|e| format!("Cannot save settings: {e}"))
    }
}

pub(super) fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}
fn mix(a: Color, b: Color, amount: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * amount,
        g: a.g + (b.g - a.g) * amount,
        b: a.b + (b.b - a.b) * amount,
        a: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contrast(a: Color, b: Color) -> f32 {
        let luminance = |c: Color| {
            let linear = |v: f32| if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
            0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
        };
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    #[test]
    fn mode_material_and_accent_compose_and_persist_with_readable_text() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("preferences.json");
        for mode in Mode::ALL {
            for material in Material::ALL {
                for accent in Accent::ALL {
                    let preferences = Preferences { mode, material, accent, waveform_theme: WaveformTheme::Ice };
                    preferences.save(&file).unwrap();
                    assert_eq!(Preferences::load(&file).unwrap(), preferences);
                    let p = preferences.palette();
                    assert!(contrast(p.ink, p.panel) >= 7.0);
                    assert!(contrast(p.muted, p.panel) >= 4.5);
                    assert!(contrast(p.accent_text, p.accent) >= 4.5);
                    let high = p.accent.r.max(p.accent.g).max(p.accent.b);
                    let low = p.accent.r.min(p.accent.g).min(p.accent.b);
                    assert!((high - low) / high >= 0.7);
                    assert_eq!(palette(&preferences.theme()).panel, p.panel);
                }
            }
        }
    }
}
