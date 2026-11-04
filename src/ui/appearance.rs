pub use super::palette::{Accent, Material, Mode};
pub(super) use super::palette::{Palette, palette};
use iced::{Color, Theme};
use serde::{Deserialize, Serialize};
use std::{fmt, path::Path};

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
    pub mode: Mode,
    pub material: Material,
    pub accent: Accent,
    pub waveform_theme: WaveformTheme,
}
impl Preferences {
    pub fn palette(self) -> Palette {
        super::palette::compose(self.mode, self.material, self.accent)
    }
    pub fn theme(self) -> Theme {
        super::palette::theme(self.mode, self.material, self.accent)
    }
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
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_surfaces_match_showbiz_native_chrome() {
        let p = Preferences::default().palette();
        assert_eq!(p.bg, rgb(16, 16, 16));
        assert_eq!(p.panel, rgb(27, 27, 27));
        assert_eq!(
            super::super::surfaces::chrome(p).background,
            Some(p.bg.into())
        );
    }

    fn contrast(a: Color, b: Color) -> f32 {
        let luminance = |c: Color| {
            let linear = |v: f32| {
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
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
                    let preferences = Preferences {
                        mode,
                        material,
                        accent,
                        waveform_theme: WaveformTheme::Ice,
                    };
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
