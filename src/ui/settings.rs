use super::{
    App, Message,
    appearance::{Accent, Material, Mode, WaveformTheme},
    icons::{self, Icon},
    view, waveform,
};
use iced::{
    Alignment, Border, Color, Element, Length,
    widget::{Column, Row, Space, button, column, container, row, scrollable, text},
};

impl App {
    pub(super) fn settings(&self) -> Element<'_, Message> {
        let p = self.preferences.palette();
        let heading = row![
            text("Settings").size(15),
            Space::new().width(Length::Fill),
            view::action("Restore defaults", Message::ResetAppearance, true),
            icons::hint(
                icons::action(Icon::Close, Message::CloseSettings, true),
                "Return to library (Esc)"
            )
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        let modes = row(Mode::ALL.into_iter().map(|mode| {
            choice(
                text(mode.to_string()).size(13),
                Message::Mode(mode),
                self.preferences.mode == mode,
            )
            .width(110)
            .into()
        }))
        .spacing(10);
        let mut materials = Column::new().spacing(10);
        for presets in Material::ALL.chunks(4) {
            let mut cards = Row::new().spacing(10);
            for &material in presets {
                let colors = super::palette::compose(
                    self.preferences.mode,
                    material,
                    self.preferences.accent,
                );
                let sample = container(
                    column![
                        container(Space::new().height(5))
                            .style(move |_| view::panel(colors.raised)),
                        row![
                            container(Space::new().width(20).height(26))
                                .style(move |_| view::panel(colors.raised)),
                            container(Space::new().width(Length::Fill).height(26))
                                .style(move |_| view::panel(colors.panel))
                        ]
                        .spacing(3)
                    ]
                    .spacing(3),
                )
                .padding(5)
                .style(move |_| view::panel(colors.bg));
                cards = cards.push(
                    choice(
                        column![sample, text(material.name()).size(11)].spacing(8),
                        Message::Material(material),
                        material == self.preferences.material,
                    )
                    .width(Length::Fill),
                );
            }
            for _ in presets.len()..4 {
                cards = cards.push(Space::new().width(Length::Fill));
            }
            materials = materials.push(cards);
        }
        let accents = row(Accent::ALL.into_iter().map(|accent| {
            let color = accent.color(self.preferences.mode);
            choice(
                column![swatch(color), text(accent.to_string()).size(10)]
                    .spacing(7)
                    .align_x(Alignment::Center)
                    .width(Length::Fill),
                Message::Accent(accent),
                self.preferences.accent == accent,
            )
            .width(Length::Fill)
            .into()
        }))
        .spacing(7);
        let appearance = column![
            text("Appearance").size(14),
            text("Mode").size(12).color(p.muted),
            modes,
            text("Material").size(12).color(p.muted),
            materials,
            text("Accent").size(12).color(p.muted),
            accents
        ]
        .spacing(18)
        .width(Length::Fill);
        let mut waves = Column::new().spacing(10);
        for presets in WaveformTheme::ALL.chunks(2) {
            waves = waves.push(
                row(presets.iter().map(|&preset| {
                    choice(
                        column![waveform::preview(preset), text(preset.name()).size(11)].spacing(8),
                        Message::WaveformTheme(preset),
                        preset == self.preferences.waveform_theme,
                    )
                    .width(Length::Fill)
                    .into()
                }))
                .spacing(10),
            );
        }
        let waveforms = column![text("Waveform").size(14), waves]
            .spacing(18)
            .width(Length::Fill);
        let navigation = container(column![
            container(text("Appearance").size(12))
                .padding([9, 10])
                .width(Length::Fill)
                .style(move |_| view::panel(p.selected))
        ])
        .padding(12)
        .width(146)
        .height(Length::Fill)
        .style(move |_| view::panel(super::surfaces::blend(p.panel, p.bg, 0.15)));
        let content =
            container(scrollable(row![appearance, waveforms].spacing(24)).height(Length::Fill))
                .padding(20)
                .width(Length::Fill);
        container(column![
            container(heading)
                .padding([10, 16])
                .height(48)
                .style(move |_| super::surfaces::chrome(p)),
            row![navigation, content].height(Length::Fill)
        ])
        .width(Length::Fill)
        .max_width(1180)
        .height(Length::Fill)
        .max_height(680)
        .style(move |_| super::surfaces::card(p))
        .into()
    }
}

fn choice<'a>(
    content: impl Into<Element<'a, Message>>,
    message: Message,
    selected: bool,
) -> iced::widget::Button<'a, Message> {
    button(content)
        .on_press(message)
        .padding(10)
        .style(move |theme, status| {
            let mut style = view::secondary(theme, status);
            style.border.radius = 8.0.into();
            if selected {
                let p = super::appearance::palette(theme);
                style.border.color = p.accent;
                style.border.width = 2.0;
                style.background = Some(p.selected.into());
            }
            style
        })
}

fn swatch(color: Color) -> Element<'static, Message> {
    container(Space::new().width(24).height(24))
        .style(move |_| container::Style {
            background: Some(color.into()),
            border: Border {
                radius: 12.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}
