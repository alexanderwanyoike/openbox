use super::{
    App, Message,
    preview::Event,
    view::{self, BG, BLUE, INK, LINE, MUTED, PANEL, action, panel, quiet},
    waveform::Wave,
};
use iced::{
    Alignment, Border, Color, Element, Font, Length, Theme,
    widget::{
        Space, canvas, column, container, progress_bar, row, rule::vertical, slider, text, tooltip,
        vertical_slider,
    },
};
use openbox_audio::{Band, DeckControl, DeckId};

const AMBER: Color = Color::from_rgb(0.95, 0.67, 0.29);

impl App {
    pub(super) fn players(&self) -> Element<'_, Message> {
        let mut content = column![
            row![
                self.deck(DeckId::A),
                vertical(1),
                self.mixer(),
                vertical(1),
                self.deck(DeckId::B)
            ]
            .spacing(12)
            .height(356)
            .align_y(Alignment::Start),
        ]
        .spacing(8);
        if let Some(error) = &self.preview.error {
            content = content.push(
                row![
                    text(error).size(11).color(AMBER).width(Length::Fill),
                    action(
                        "Retry audio",
                        Message::Preview(Event::Retry),
                        !self.preview.starting
                    )
                ]
                .align_y(Alignment::Center),
            );
        }
        container(content)
            .padding([10, 12])
            .style(|_| panel(PANEL))
            .into()
    }

    fn deck(&self, id: DeckId) -> Element<'_, Message> {
        let deck = &self.preview.decks[id.index()];
        let state = &self.preview.snapshot.decks[id.index()];
        let accent = if id == DeckId::A { BLUE } else { AMBER };
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
            waveform: deck.clip.as_ref().map(|c| &c.waveform),
            deck: id,
            position: state.position,
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
                        radius: 3.0.into()
                    },
                    ..panel(BG)
                }),
            view::cover(
                deck.track
                    .as_ref()
                    .and_then(|t| t.details.artwork.as_deref()),
                36
            ),
            column![
                tooltip(
                    text(title_short).size(14),
                    text(title).size(12),
                    tooltip::Position::Bottom
                ),
                text(artist.chars().take(45).collect::<String>())
                    .size(10)
                    .color(MUTED)
            ]
            .spacing(2)
            .width(Length::Fill),
            column![
                text(bpm).size(21).font(Font::MONOSPACE).color(accent),
                text("BPM").size(8).color(MUTED)
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
                .color(INK),
            text(format!(
                "-{}",
                time((state.duration - state.position).max(0.0))
            ))
            .font(Font::MONOSPACE)
            .size(11)
            .color(MUTED),
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
            .color(if state.playing { accent } else { MUTED }),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        let zoom = row![
            text("WAVEFORM").size(8).font(Font::MONOSPACE).color(MUTED),
            text("click to seek · drag to scrub").size(9).color(MUTED),
            Space::new().width(Length::Fill),
            action(
                "-",
                Message::Preview(Event::Zoom(id, deck.zoom * 2.0)),
                deck.zoom < 64.0
            )
            .style(quiet)
            .padding([0, 6]),
            text(format!("{:.0}s", deck.zoom)).size(9).color(MUTED),
            action(
                "+",
                Message::Preview(Event::Zoom(id, deck.zoom / 2.0)),
                deck.zoom > 4.0
            )
            .style(quiet)
            .padding([0, 6]),
        ]
        .spacing(5)
        .align_y(Alignment::Center);
        let transport = row![
            action(
                if state.playing { "Pause" } else { "Play" },
                control(DeckControl::PlayPause),
                enabled
            )
            .width(72)
            .style(move |theme, status| {
                let mut style = view::secondary(theme, status);
                if enabled {
                    style.background = Some(accent.into());
                    style.text_color = BG;
                    style.border.color = accent;
                }
                style
            }),
            action("Cue", control(DeckControl::Cue), enabled),
            action("Set cue", control(DeckControl::SetCue), enabled),
            Space::new().width(Length::Fill),
            tooltip(
                action(
                    "<",
                    control(DeckControl::Seek(state.position - 0.05)),
                    enabled
                ),
                text("Back 50 ms").size(11),
                tooltip::Position::Bottom
            ),
            tooltip(
                action(
                    ">",
                    control(DeckControl::Seek(state.position + 0.05)),
                    enabled
                ),
                text("Forward 50 ms").size(11),
                tooltip::Position::Bottom
            ),
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        let pitch = row![
            text("PITCH").size(9).font(Font::MONOSPACE).color(MUTED),
            slider(-8.0..=8.0, deck.pitch, move |pitch| control(
                DeckControl::Rate(1.0 + pitch / 100.0)
            ))
            .default(0.0)
            .step(0.05)
            .height(16)
            .style(move |theme, status| slider_style(theme, status, accent)),
            text(format!("{:+.2}%", deck.pitch))
                .size(11)
                .font(Font::MONOSPACE)
                .color(accent)
                .width(60),
            action("0", control(DeckControl::Rate(1.0)), enabled)
                .style(quiet)
                .padding([2, 6]),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        column![
            header,
            clock,
            canvas(wave(true)).width(Length::Fill).height(28),
            zoom,
            canvas(wave(false)).width(Length::Fill).height(174),
            transport,
            pitch
        ]
        .spacing(6)
        .width(Length::Fill)
        .into()
    }

    fn mixer(&self) -> Element<'_, Message> {
        let state = &self.preview.snapshot;
        let channel = |id: DeckId, accent| {
            let deck = &self.preview.decks[id.index()];
            let knob_hint = || text("Drag up/down · scroll · double-click to reset").size(10);
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
                    text("TRIM").size(8).color(MUTED).width(26),
                    tooltip(
                        super::knob::knob(0.0..=1.0, deck.gain, 0.8, 0.01, accent, move |value| {
                            Message::Preview(Event::Control(id, DeckControl::Gain(value)))
                        }),
                        knob_hint(),
                        tooltip::Position::Bottom
                    ),
                    text(format!("{:.0}", deck.gain * 100.0))
                        .size(9)
                        .font(Font::MONOSPACE)
                        .color(MUTED)
                        .width(22),
                ]
                .spacing(3)
                .align_y(Alignment::Center),
            ]
            .spacing(5);
            for (label, band) in [("HIGH", Band::High), ("MID", Band::Mid), ("LOW", Band::Low)] {
                let db = deck.eq[band.index()];
                controls = controls.push(
                    row![
                        text(label).size(8).color(MUTED).width(26),
                        tooltip(
                            super::knob::knob(-24.0..=6.0, db, 0.0, 0.5, accent, move |value| {
                                Message::Preview(Event::Control(id, DeckControl::Eq(band, value)))
                            }),
                            knob_hint(),
                            tooltip::Position::Bottom
                        ),
                        text(format!("{db:+.0}"))
                            .size(9)
                            .font(Font::MONOSPACE)
                            .color(MUTED)
                            .width(22),
                    ]
                    .spacing(3)
                    .align_y(Alignment::Center),
                );
            }
            controls.push(
                column![
                    row![
                        column((0..=5).rev().map(|n| text(format!("{}", n * 2)).size(8).color(MUTED).into())).spacing(10)
                            .width(15).height(100),
                        tooltip(
                            vertical_slider(0.0..=1.0, deck.fader, move |value| {
                                Message::Preview(Event::Control(id, DeckControl::Fader(value)))
                            })
                            .default(1.0).step(0.01).shift_step(0.001)
                            .width(30).height(100)
                            .style(move |theme, status| fader_style(theme, status, accent)),
                            text("Channel volume · Ctrl-click to reset · Shift for fine adjustment").size(10),
                            tooltip::Position::Bottom
                        ),
                        channel_meter(state.decks[id.index()].peak),
                    ].spacing(7).align_y(Alignment::Center),
                    text(format!("LEVEL {:3.0}%", deck.fader * 100.0))
                        .size(8).font(Font::MONOSPACE).color(MUTED),
                ].spacing(4).padding([5, 8])
            ).width(Length::Fill)
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
                .color(MUTED)
                .width(Length::Fill),
                action("Reset", Message::Preview(Event::ResetMixer), true)
                    .style(quiet)
                    .padding([0, 2])
            ]
            .align_y(Alignment::Center),
            row![channel(DeckId::A, BLUE), channel(DeckId::B, AMBER)].spacing(16),
            row![
                text("MASTER").size(8).color(MUTED),
                slider(0.0..=1.0, self.preview.master, |value| Message::Preview(
                    Event::Master(value)
                ))
                .default(0.8)
                .step(0.01)
                .height(14),
                text(if state.clipped { "CLIP" } else { "OUT" })
                    .size(8)
                    .color(if state.clipped { AMBER } else { MUTED })
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            meter(state.peak, if state.clipped { AMBER } else { BLUE }),
            row![
                text("A").size(9).color(BLUE),
                slider(0.0..=1.0, self.preview.crossfade, |value| Message::Preview(
                    Event::Crossfade(value)
                ))
                .default(0.5)
                .step(0.01)
                .height(20),
                text("B").size(9).color(AMBER)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(5)
        .width(230)
        .into()
    }
}

fn fader_style(theme: &Theme, status: slider::Status, accent: Color) -> slider::Style {
    let mut style = slider::default(theme, status);
    style.rail.backgrounds = (BG.into(), BG.into());
    style.rail.width = 5.0;
    style.handle.shape = slider::HandleShape::Rectangle {
        width: 9,
        border_radius: 2.0.into(),
    };
    style.handle.background = if status == slider::Status::Dragged {
        accent
    } else {
        INK
    }
    .into();
    style.handle.border_width = 1.0;
    style.handle.border_color = accent;
    style
}

fn channel_meter(peak: f32) -> Element<'static, Message> {
    let level = if peak > 0.0 {
        (1.0 + 20.0 * peak.log10() / 48.0).clamp(0.0, 1.0)
    } else {
        0.0
    };
    column((0..12).rev().map(|i| {
        let color = if i >= 10 {
            Color::from_rgb(0.9, 0.36, 0.38)
        } else if i >= 8 {
            AMBER
        } else {
            Color::from_rgb(0.3, 0.78, 0.5)
        };
        container(Space::new().width(7).height(6))
            .style(move |_| container::Style {
                background: Some(if level > i as f32 / 12.0 { color } else { LINE }.into()),
                ..Default::default()
            })
            .into()
    }))
    .spacing(2)
    .into()
}

fn slider_style(theme: &Theme, status: slider::Status, accent: Color) -> slider::Style {
    let mut style = slider::default(theme, status).with_circular_handle(4.0);
    style.rail.backgrounds = (accent.into(), LINE.into());
    style.rail.width = 2.0;
    style.handle.background = accent.into();
    style
}

fn meter(value: f32, accent: Color) -> Element<'static, Message> {
    progress_bar(0.0..=1.0, value)
        .girth(3)
        .style(move |_| progress_bar::Style {
            background: BG.into(),
            bar: accent.into(),
            border: Border::default(),
        })
        .into()
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
