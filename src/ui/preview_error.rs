use super::{
    App, Message,
    icons::{self, Icon},
    preview::Event,
    view,
};
use iced::{
    Alignment, Border, Element, Font, Length,
    widget::{Space, column, container, row, scrollable, text},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Track,
    Output,
}
#[derive(Debug, Clone)]
pub(super) struct Failure {
    pub kind: Kind,
    pub diagnostics: String,
}
impl Failure {
    pub fn track(diagnostics: String) -> Self {
        Self {
            kind: Kind::Track,
            diagnostics,
        }
    }
    pub fn output(diagnostics: String) -> Self {
        Self {
            kind: Kind::Output,
            diagnostics,
        }
    }
}
impl App {
    pub(super) fn preview_error(&self, failure: &Failure) -> Element<'_, Message> {
        let p = self.preferences.palette();
        let (title, advice) = match failure.kind {
            Kind::Track => (
                "Track could not be loaded",
                "Check that its file is available and readable, then load the track again.",
            ),
            Kind::Output => (
                "Audio output is unavailable",
                "Check the system audio output, then retry audio.",
            ),
        };
        let mut actions = row![
            view::action(
                if self.preview.error_details {
                    "Hide details"
                } else {
                    "Details"
                },
                Message::Preview(Event::ToggleErrorDetails),
                true
            )
            .style(view::quiet),
            view::action("Dismiss", Message::Preview(Event::DismissError), true).style(view::quiet),
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        if failure.kind == Kind::Output {
            actions = actions.push(view::action(
                "Retry audio",
                Message::Preview(Event::Retry),
                !self.preview.starting,
            ));
        }
        let mut body = column![
            row![
                icons::themed(Icon::Warning, true),
                column![text(title).size(13), text(advice).size(11).color(p.muted)].spacing(4),
                Space::new().width(Length::Fill),
                actions
            ]
            .spacing(10)
            .align_y(Alignment::Center)
        ]
        .spacing(10);
        if self.preview.error_details {
            body = body.push(
                container(
                    scrollable(
                        text(failure.diagnostics.clone())
                            .font(Font::MONOSPACE)
                            .size(11),
                    )
                    .height(88),
                )
                .padding(8)
                .style(move |_| view::panel(p.bg)),
            );
        }
        container(body)
            .padding(12)
            .width(Length::Fill)
            .style(move |_| iced::widget::container::Style {
                background: Some(super::surfaces::blend(p.panel, p.danger, 0.04).into()),
                border: Border {
                    color: iced::Color { a: 0.4, ..p.danger },
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            })
            .into()
    }
}
