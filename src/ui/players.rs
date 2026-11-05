use super::{
    App, Message,
    faders::{self, Kind},
    icons::{self, Icon},
    preview::Event,
    view::{self, action, panel},
    waveform::Wave,
};
use iced::{
    Alignment, Border, Color, Element, Font, Length, Theme,
    widget::{Space, canvas, column, container, row, text, vertical_slider},
};
use openbox_audio::{Band, DeckControl, DeckId};

impl App {
    pub(super) fn players(&self) -> Element<'_, Message> {
        let colors = self.preferences.palette();
        let mut content = column![
            row![
                container(self.deck(DeckId::A))
                    .id("deck-a-panel")
                    .padding(10)
                    .width(Length::Fill)
                    .height(376)
                    .style(move |_| super::surfaces::card(colors)),
                container(self.mixer())
                    .id("mixer-panel")
                    .padding(10)
                    .width(250)
                    .height(376)
                    .style(move |_| super::surfaces::card(colors)),
                container(self.deck(DeckId::B))
                    .id("deck-b-panel")
                    .padding(10)
                    .width(Length::Fill)
                    .height(376)
                    .style(move |_| super::surfaces::card(colors))
            ]
            .spacing(8)
            .height(376)
            .align_y(Alignment::Start),
        ]
        .spacing(8);
        if let Some(error) = &self.preview.error {
            content = content.push(self.preview_error(error));
        }
        container(content)
            .padding(8)
            .style(move |_| panel(colors.bg))
            .into()
    }

    fn deck(&self, id: DeckId) -> Element<'_, Message> {
        let colors = self.preferences.palette();
        let deck = &self.preview.decks[id.index()];
        let state = &self.preview.snapshot.decks[id.index()];
        let accent = if id == DeckId::A {
            colors.accent
        } else {
            colors.deck_b
        };
        let letter = if id == DeckId::A { "A" } else { "B" };
        let title = if deck.loading {
            deck.pending
                .as_ref()
                .map_or("Loading...", |t| t.title.as_str())
        } else {
            deck.track
                .as_ref()
                .map_or("Load a track to preview", |t| t.title.as_str())
        };
        let artist = deck
            .track
            .as_ref()
            .map_or("Select one track in either collection", |t| {
                t.artist.as_str()
            });
        let bpm = deck
            .track
            .as_ref()
            .and_then(|t| t.details.bpm)
            .map(|bpm| format!("{:.1}", bpm * (1.0 + deck.pitch / 100.0)))
            .unwrap_or_else(|| "--.-".into());
        let enabled = deck.clip.is_some() && !deck.loading && self.preview.engine.is_some();
        let control = move |control| Message::Preview(Event::Control(id, control));
        let wave = |overview| Wave {
            theme: self.preferences.waveform_theme,
            waveform: deck.clip.as_ref().map(|c| &c.waveform),
            deck: id,
            position: state.position,
            playing: state.playing && !self.settings_visible,
            rate: state.rate,
            sampled_at: state.sampled_at,
            cue: state.cue,
            span: deck.zoom,
            overview,
            color: accent,
            beats: deck
                .track
                .as_ref()
                .and_then(|t| t.details.analysis.as_ref())
                .map_or(&[], |g| g.beats.as_slice()),
        };
        let title_short: String = title.chars().take(42).collect();
        let header = row![
            container(text(letter).size(17).color(accent).font(Font::MONOSPACE))
                .padding([4, 9])
                .style(move |_| container::Style {
                    border: Border {
                        color: accent,
                        width: 1.0,
                        radius: 7.0.into()
                    },
                    ..panel(colors.bg)
                }),
            view::cover(
                deck.track
                    .as_ref()
                    .and_then(|t| t.details.artwork.as_deref()),
                36,
                colors
            ),
            column![
                icons::hint(text(title_short).size(14), title),
                text(artist.chars().take(45).collect::<String>())
                    .size(10)
                    .color(colors.muted)
            ]
            .spacing(2)
            .width(Length::Fill),
            column![
                text(bpm).size(21).font(Font::MONOSPACE).color(accent),
                text("BPM").size(8).color(colors.muted)
            ]
            .width(68),
            action(
                if deck.loading {
                    "Loading"
                } else if id == DeckId::A {
                    "Load A"
                } else {
                    "Load B"
                },
                Message::Preview(Event::Load(id)),
                self.preview_track().is_some() && !deck.loading
            ),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .height(40);
        let clock = row![
            text(time(state.position))
                .font(Font::MONOSPACE)
                .size(14)
                .color(colors.ink),
            text(format!(
                "-{}",
                time((state.duration - state.position).max(0.0))
            ))
            .font(Font::MONOSPACE)
            .size(11)
            .color(colors.muted),
            Space::new().width(Length::Fill),
            text(deck.track.as_ref().map_or("", |t| t.details.key.as_str()))
                .size(11)
                .color(accent),
            text(if deck.loading {
                "DECODING"
            } else if state.playing {
                "PLAYING"
            } else {
                "PAUSED"
            })
            .size(9)
            .font(Font::MONOSPACE)
            .color(if state.playing { accent } else { colors.muted }),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        let zoom = row![
            text("WAVEFORM")
                .size(8)
                .font(Font::MONOSPACE)
                .color(colors.muted),
            text("click to seek · drag to scrub")
                .size(9)
                .color(colors.muted),
            Space::new().width(Length::Fill),
            icons::hint(
                icons::action(
                    Icon::ZoomOut,
                    Message::Preview(Event::Zoom(id, deck.zoom * 2.0)),
                    deck.zoom < 64.0
                )
                .height(18)
                .width(24)
                .padding(1),
                "Zoom out to see more of the track"
            ),
            text(format!("{:.0}s", deck.zoom))
                .size(9)
                .color(colors.muted),
            icons::hint(
                icons::action(
                    Icon::ZoomIn,
                    Message::Preview(Event::Zoom(id, deck.zoom / 2.0)),
                    deck.zoom > 1.0
                )
                .height(18)
                .width(24)
                .padding(1),
                "Zoom in for finer waveform detail"
            ),
        ]
        .spacing(5)
        .align_y(Alignment::Center);
        let transport = row![
            icons::hint(
                icons::action(
                    if state.playing {
                        Icon::Pause
                    } else {
                        Icon::Play
                    },
                    control(DeckControl::PlayPause),
                    enabled
                )
                .width(72)
                .style(move |theme, status| {
                    let mut style = view::secondary(theme, status);
                    if enabled {
                        style.background = Some(accent.into());
                        style.border.color = accent;
                    }
                    style
                }),
                if state.playing {
                    "Pause playback"
                } else {
                    "Play this deck"
                }
            ),
            icons::hint(
                action("Cue", control(DeckControl::Cue), enabled),
                "Return to the cue point and pause"
            ),
            icons::hint(
                action("Set cue", control(DeckControl::SetCue), enabled),
                "Set the temporary cue at this position and pause"
            ),
            Space::new().width(Length::Fill),
            icons::hint(
                icons::action(
                    Icon::Back,
                    control(DeckControl::Seek(state.position - 0.05)),
                    enabled
                ),
                "Seek back 50 ms"
            ),
            icons::hint(
                icons::action(
                    Icon::Forward,
                    control(DeckControl::Seek(state.position + 0.05)),
                    enabled
                ),
                "Seek forward 50 ms"
            ),
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        let pitch = row![
            text("PITCH")
                .size(9)
                .font(Font::MONOSPACE)
                .color(colors.muted),
            faders::horizontal(Kind::Pitch, deck.pitch, accent, move |pitch| control(
                DeckControl::Rate(1.0 + pitch / 100.0)
            )),
            text(format!("{:+.2}%", deck.pitch))
                .size(11)
                .font(Font::MONOSPACE)
                .color(accent)
                .width(60),
            icons::hint(
                icons::action(Icon::Reset, control(DeckControl::Rate(1.0)), enabled),
                "Reset pitch to the track’s original speed"
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        column![
            header,
            clock,
            canvas(wave(true)).width(Length::Fill).height(28),
            zoom,
            canvas(wave(false)).width(Length::Fill).height(158),
            transport,
            container(pitch).id(format!("pitch-controls-{id:?}"))
        ]
        .spacing(6)
        .width(Length::Fill)
        .into()
    }

    fn mixer(&self) -> Element<'_, Message> {
        let colors = self.preferences.palette();
        let state = &self.preview.snapshot;
        let channel = |id: DeckId, accent| {
            let deck = &self.preview.decks[id.index()];
            let knob_hint = "Drag up/down · Shift for fine adjustment · right-click to reset";
            let mut controls = column![
                text(if id == DeckId::A {
                    "CHANNEL A"
                } else {
                    "CHANNEL B"
                })
                .size(9)
                .font(Font::MONOSPACE)
                .color(accent),
                row![
                    text("TRIM").size(8).color(colors.muted).width(26),
                    icons::hint(
                        super::knob::knob(0.0..=1.0, deck.gain, 0.8, 0.005, accent, move |value| {
                            Message::Preview(Event::Control(id, DeckControl::Gain(value)))
                        }),
                        knob_hint
                    ),
                    text(format!("{:.0}", deck.gain * 100.0))
                        .size(9)
                        .font(Font::MONOSPACE)
                        .color(colors.muted)
                        .width(28),
                ]
                .spacing(3)
                .align_y(Alignment::Center),
            ]
            .spacing(5);
            for (label, band) in [("HIGH", Band::High), ("MID", Band::Mid), ("LOW", Band::Low)] {
                let db = deck.eq[band.index()];
                controls = controls.push(
                    row![
                        text(label).size(8).color(colors.muted).width(26),
                        icons::hint(
                            super::knob::knob(-24.0..=6.0, db, 0.0, 0.1, accent, move |value| {
                                Message::Preview(Event::Control(id, DeckControl::Eq(band, value)))
                            }),
                            knob_hint
                        ),
                        text(format!("{db:+.1}"))
                            .size(9)
                            .font(Font::MONOSPACE)
                            .color(colors.muted)
                            .width(28),
                    ]
                    .spacing(3)
                    .align_y(Alignment::Center),
                );
            }
            controls
                .push(
                    column![
                        row![
                            canvas(FaderScale).width(15).height(94),
                            icons::hint(
                                vertical_slider(0.0..=1.0, deck.fader, move |value| {
                                    Message::Preview(Event::Control(id, DeckControl::Fader(value)))
                                })
                                .default(1.0)
                                .step(0.01)
                                .shift_step(0.001)
                                .width(30)
                                .height(94)
                                .style(move |theme, status| faders::vertical_style(
                                    theme, status, accent
                                )),
                                "Channel volume · Ctrl-click to reset · Shift for fine adjustment"
                            ),
                            channel_meter(state.decks[id.index()].peak, colors),
                        ]
                        .spacing(7)
                        .align_y(Alignment::Center),
                        text(format!("LEVEL {:3.0}%", deck.fader * 100.0))
                            .size(8)
                            .font(Font::MONOSPACE)
                            .color(colors.muted),
                    ]
                    .spacing(4)
                    .padding([5, 8]),
                )
                .width(Length::Fill)
        };
        column![
            row![
                text(if self.preview.starting {
                    "OPENING AUDIO"
                } else {
                    "MIXER"
                })
                .size(9)
                .font(Font::MONOSPACE)
                .color(colors.muted)
                .width(Length::Fill),
                icons::hint(
                    icons::action(Icon::Reset, Message::Preview(Event::ResetMixer), true)
                        .height(17)
                        .width(24)
                        .padding(0),
                    "Reset trim, EQ, channel faders, master and crossfader"
                )
            ]
            .align_y(Alignment::Center),
            row![
                channel(DeckId::A, colors.accent),
                channel(DeckId::B, colors.deck_b)
            ]
            .spacing(16),
            row![
                text("MASTER").size(8).color(colors.muted),
                faders::horizontal(
                    Kind::Master,
                    f64::from(self.preview.master),
                    colors.accent,
                    |value| Message::Preview(Event::Master(value as f32))
                ),
                text(if state.clipped { "CLIP" } else { "OUT" })
                    .size(8)
                    .color(if state.clipped {
                        colors.deck_b
                    } else {
                        colors.muted
                    })
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            meter(
                state.peak,
                if state.clipped {
                    colors.deck_b
                } else {
                    colors.accent
                }
            ),
            row![
                text("A").size(9).color(colors.accent),
                faders::horizontal(
                    Kind::Crossfade,
                    f64::from(self.preview.crossfade),
                    colors.accent,
                    |value| Message::Preview(Event::Crossfade(value as f32))
                ),
                text("B").size(9).color(colors.deck_b)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(5)
        .width(230)
        .into()
    }
}

fn channel_meter(peak: f32, colors: super::appearance::Palette) -> Element<'static, Message> {
    let level = if peak > 0.0 {
        (1.0 + 20.0 * peak.log10() / 48.0).clamp(0.0, 1.0)
    } else {
        0.0
    };
    column((0..12).rev().map(|i| {
        let color = if i >= 10 {
            colors.danger
        } else if i >= 8 {
            colors.deck_b
        } else {
            colors.success
        };
        container(Space::new().width(7).height(6))
            .style(move |_| container::Style {
                background: Some(
                    if level > i as f32 / 12.0 {
                        color
                    } else {
                        colors.line
                    }
                    .into(),
                ),
                ..Default::default()
            })
            .into()
    }))
    .spacing(2)
    .into()
}

fn meter(value: f32, accent: Color) -> Element<'static, Message> {
    canvas(OutputMeter { value, accent })
        .width(Length::Fill)
        .height(3)
        .into()
}
struct OutputMeter {
    value: f32,
    accent: Color,
}
impl canvas::Program<Message> for OutputMeter {
    type State = ();
    fn draw(
        &self,
        _: &(),
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let p = super::appearance::palette(theme);
        let width = (bounds.width - 23.0 * 2.0) / 24.0;
        for n in 0..24 {
            frame.fill_rectangle(
                iced::Point::new(n as f32 * (width + 2.0), 0.0),
                iced::Size::new(width, bounds.height),
                if self.value > n as f32 / 24.0 {
                    self.accent
                } else {
                    p.line
                },
            );
        }
        vec![frame.into_geometry()]
    }
}

fn time(seconds: f64) -> String {
    let centiseconds = (seconds.max(0.0) * 100.0) as u64;
    format!(
        "{:02}:{:02}.{:02}",
        centiseconds / 6000,
        centiseconds / 100 % 60,
        centiseconds % 100
    )
}

struct FaderScale;
impl canvas::Program<Message> for FaderScale {
    type State = ();
    fn draw(
        &self,
        _: &(),
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        use iced::{Point, Size};
        let colors = super::appearance::palette(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        for db in -30..=0 {
            let y = (1.0 - 10.0_f32.powf(db as f32 / 20.0)) * (bounds.height - 8.0) + 4.0;
            let labeled = [0, -3, -6, -12].contains(&db);
            frame.fill_rectangle(
                Point::new(if labeled { 11.0 } else { 13.0 }, y),
                Size::new(if labeled { 4.0 } else { 2.0 }, 1.0),
                colors.muted,
            );
            if labeled {
                frame.fill_text(canvas::Text {
                    content: db.to_string(),
                    position: Point::new(0.0, (y - 3.5).max(0.0)),
                    size: 7.0.into(),
                    color: colors.muted,
                    ..Default::default()
                });
            }
        }
        frame.fill_text(canvas::Text {
            content: "-∞".into(),
            position: Point::new(1.0, bounds.height - 7.0),
            size: 8.0.into(),
            color: colors.muted,
            ..Default::default()
        });
        vec![frame.into_geometry()]
    }
}
