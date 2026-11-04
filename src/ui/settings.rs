use super::{
    App, Message,
    appearance::{UiTheme, WaveformTheme},
    icons::{self, Icon},
    view, waveform,
};
use iced::{
    Alignment, Border, Color, Element, Font, Length,
    widget::{Column, Space, button, column, container, row, scrollable, text},
};

impl App {
    pub(super) fn settings(&self) -> Element<'_, Message> {
        let colors = self.preferences.ui_theme.palette();
        let heading = row![
            column![
                text("Settings").size(26),
                text("Appearance").size(12).color(colors.muted)
            ]
            .spacing(4),
            Space::new().width(Length::Fill),
            view::action("Restore defaults", Message::ResetAppearance, true),
            icons::hint(
                icons::action(Icon::Close, Message::CloseSettings, true),
                "Return to your library (Esc)"
            )
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        let mut interfaces = Column::new().spacing(7);
        for preset in UiTheme::ALL {
            let p = preset.palette();
            let selected = preset == self.preferences.ui_theme;
            let swatches = row([p.bg, p.raised, p.ink, p.accent, p.deck_b]
                .into_iter()
                .map(swatch))
            .spacing(4);
            interfaces = interfaces.push(
                button(
                    row![
                        swatches,
                        text(preset.name()).size(13).width(Length::Fill),
                        text(if selected { "ACTIVE" } else { "" })
                            .size(9)
                            .font(Font::MONOSPACE)
                            .color(colors.accent)
                    ]
                    .spacing(14)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::UiTheme(preset))
                .padding([13, 14])
                .width(Length::Fill)
                .style(move |theme, status| card_style(theme, status, selected)),
            );
        }
        let mut waveforms = Column::new().spacing(7);
        for preset in WaveformTheme::ALL {
            let selected = preset == self.preferences.waveform_theme;
            waveforms = waveforms.push(
                button(
                    row![
                        container(waveform::preview(preset)).width(168),
                        text(preset.name()).size(13).width(Length::Fill),
                        text(if selected { "ACTIVE" } else { "" })
                            .size(9)
                            .font(Font::MONOSPACE)
                            .color(colors.accent)
                    ]
                    .spacing(14)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::WaveformTheme(preset))
                .padding([6, 14])
                .width(Length::Fill)
                .style(move |theme, status| card_style(theme, status, selected)),
            );
        }
        let interface_panel = column![
            text("Interface").size(18),
            text("Ten palettes for panels, text, controls and deck accents.")
                .size(12)
                .color(colors.muted),
            interfaces
        ]
        .spacing(12)
        .width(Length::Fill);
        let waveform_panel = column![
            text("Waveforms").size(18),
            text("Choose independently. Colors represent low, mid and high energy.")
                .size(12)
                .color(colors.muted),
            waveforms
        ]
        .spacing(12)
        .width(Length::Fill);
        container(column![
            heading,
            text("Changes apply immediately and are saved automatically. Playback continues while Settings is open.").size(12).color(colors.muted),
            scrollable(row![interface_panel, waveform_panel].spacing(28)).height(Length::Fill)
        ].spacing(20)).padding(28).width(Length::Fill).height(Length::Fill).into()
    }
}

fn swatch(color: Color) -> Element<'static, Message> {
    container(Space::new().width(16).height(20))
        .style(move |theme| container::Style {
            background: Some(color.into()),
            border: Border {
                color: super::appearance::palette(theme).line,
                width: 1.0,
                radius: 3.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn card_style(theme: &iced::Theme, status: button::Status, selected: bool) -> button::Style {
    let mut style = view::secondary(theme, status);
    if selected {
        let p = super::appearance::palette(theme);
        style.border.color = p.accent;
        style.background = Some(p.selected.into());
    }
    style
}
