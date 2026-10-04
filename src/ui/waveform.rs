use super::{Message, preview};
use iced::{
    Color, Point, Rectangle, Renderer, Size, Theme, mouse,
    widget::{
        Action,
        canvas::{self, Frame, Geometry, Path, Stroke},
    },
};
use openbox_audio::{DeckControl, DeckId, Waveform};

#[derive(Clone, Copy)]
struct Viewport {
    start: f64,
    span: f64,
    duration: f64,
}

impl Viewport {
    fn new(position: f64, duration: f64, span: f64, overview: bool) -> Self {
        Self {
            start: if overview { 0.0 } else { position - span / 2.0 },
            span: if overview { duration.max(0.001) } else { span },
            duration,
        }
    }
    fn seek_at(self, fraction: f64) -> f64 {
        (self.start + fraction * self.span).clamp(0.0, self.duration)
    }
    fn drag_seek(self, initial: f64, delta: f64) -> f64 {
        (initial - delta * self.span).clamp(0.0, self.duration)
    }
}

pub(super) struct Wave<'a> {
    pub waveform: Option<&'a Waveform>,
    pub deck: DeckId,
    pub position: f64,
    pub cue: f64,
    pub span: f64,
    pub overview: bool,
    pub color: Color,
    pub beats: &'a [openbox_domain::Beat],
}

struct Drag {
    x: f32,
    position: f64,
    viewport: Viewport,
    moved: bool,
}

#[derive(Default)]
pub(super) struct State {
    drag: Option<Drag>,
}

impl canvas::Program<Message> for Wave<'_> {
    type State = State;

    fn update(
        &self,
        state: &mut State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        let waveform = self.waveform?;
        let seek = |seconds| {
            Action::publish(Message::Preview(preview::Event::Control(
                self.deck,
                DeckControl::Seek(seconds),
            )))
            .and_capture()
        };
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let point = cursor.position_in(bounds)?;
                let viewport =
                    Viewport::new(self.position, waveform.duration, self.span, self.overview);
                state.drag = Some(Drag {
                    x: point.x,
                    position: self.position,
                    viewport,
                    moved: false,
                });
                if self.overview {
                    Some(seek(viewport.seek_at(f64::from(point.x / bounds.width))))
                } else {
                    Some(Action::capture())
                }
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let drag = state.drag.as_mut()?;
                let point = cursor.position()?;
                let x = point.x - bounds.x;
                drag.moved |= (x - drag.x).abs() > 3.0;
                if self.overview {
                    Some(seek(drag.viewport.seek_at(f64::from(x / bounds.width))))
                } else if drag.moved {
                    Some(seek(drag.viewport.drag_seek(
                        drag.position,
                        f64::from((x - drag.x) / bounds.width),
                    )))
                } else {
                    Some(Action::capture())
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let drag = state.drag.take()?;
                if !drag.moved && !self.overview {
                    Some(seek(
                        drag.viewport.seek_at(f64::from(drag.x / bounds.width)),
                    ))
                } else {
                    Some(Action::capture())
                }
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            Color::from_rgb(0.057, 0.067, 0.081),
        );
        let middle = bounds.height / 2.0;
        frame.fill_rectangle(
            Point::new(0.0, middle),
            Size::new(bounds.width, 1.0),
            Color::from_rgb(0.15, 0.18, 0.21),
        );
        let Some(waveform) = self.waveform else {
            frame.fill_text(canvas::Text {
                content: if self.overview {
                    "Full track overview"
                } else {
                    "Select a track below and load this deck"
                }
                .into(),
                position: Point::new(10.0, middle - 6.0),
                max_width: (bounds.width - 20.0).max(1.0),
                align_x: iced::widget::text::Alignment::Left,
                color: Color::from_rgb(0.4, 0.46, 0.52),
                size: 11.0.into(),
                ..Default::default()
            });
            return vec![frame.into_geometry()];
        };
        let viewport = Viewport::new(self.position, waveform.duration, self.span, self.overview);
        if self.overview {
            let played = bounds.width * (self.position / waveform.duration.max(0.001)) as f32;
            frame.fill_rectangle(
                Point::ORIGIN,
                Size::new(played, bounds.height),
                Color {
                    a: 0.08,
                    ..self.color
                },
            );
            let left = ((self.position - self.span / 2.0) / waveform.duration).clamp(0.0, 1.0)
                as f32
                * bounds.width;
            let right = ((self.position + self.span / 2.0) / waveform.duration).clamp(0.0, 1.0)
                as f32
                * bounds.width;
            frame.stroke_rectangle(
                Point::new(left, 1.0),
                Size::new(right - left, bounds.height - 2.0),
                Stroke::default().with_color(Color {
                    a: 0.35,
                    ..self.color
                }),
            );
        } else {
            let step = if self.span <= 8.0 {
                1.0
            } else if self.span <= 32.0 {
                2.0
            } else {
                4.0
            };
            let mut time = (viewport.start / step).ceil() * step;
            while time < viewport.start + viewport.span {
                if time >= 0.0 && time <= waveform.duration {
                    let x = ((time - viewport.start) / viewport.span) as f32 * bounds.width;
                    frame.fill_rectangle(
                        Point::new(x, 0.0),
                        Size::new(1.0, bounds.height),
                        Color::from_rgb(0.12, 0.14, 0.17),
                    );
                    frame.fill_text(canvas::Text {
                        content: format!("{}:{:02}", time as u32 / 60, time as u32 % 60),
                        position: Point::new(x + 4.0, 3.0),
                        size: 9.0.into(),
                        color: Color::from_rgb(0.36, 0.42, 0.48),
                        ..Default::default()
                    });
                }
                time += step;
            }
        }
        let amplitude = middle - if self.overview { 3.0 } else { 12.0 };
        if !self.overview {
            for beat in self.beats.iter().filter(|b| {
                b.time_seconds >= viewport.start && b.time_seconds <= viewport.start + viewport.span
            }) {
                let x =
                    ((beat.time_seconds - viewport.start) / viewport.span) as f32 * bounds.width;
                let downbeat = beat.number == 1;
                frame.fill_rectangle(
                    Point::new(x, 15.0),
                    Size::new(if downbeat { 1.5 } else { 1.0 }, bounds.height - 15.0),
                    Color {
                        a: if downbeat { 0.38 } else { 0.13 },
                        ..self.color
                    },
                );
            }
        }
        for pixel in (0..bounds.width as usize).step_by(2) {
            let from = viewport.start + pixel as f64 / f64::from(bounds.width) * viewport.span;
            let to = viewport.start + (pixel + 2) as f64 / f64::from(bounds.width) * viewport.span;
            if to <= 0.0 || from >= waveform.duration {
                continue;
            }
            let start = (from.max(0.0) * waveform.bins_per_second).floor() as usize;
            let end = ((to * waveform.bins_per_second).ceil() as usize).min(waveform.peaks.len());
            if start >= end {
                continue;
            }
            let (peak, rms) = waveform.peaks[start..end]
                .iter()
                .fold((0.0_f32, 0.0_f32), |(peak, rms), p| {
                    (peak.max(p.peak), rms.max(p.rms))
                });
            let height = (peak.sqrt() * amplitude).max(1.0);
            let body = rms.sqrt() * amplitude;
            let alpha = if from < self.position { 0.42 } else { 0.85 };
            frame.fill_rectangle(
                Point::new(pixel as f32, middle - height),
                Size::new(1.5, height * 2.0),
                Color {
                    a: alpha,
                    ..self.color
                },
            );
            frame.fill_rectangle(
                Point::new(pixel as f32, middle - body),
                Size::new(1.5, body * 2.0),
                Color {
                    a: 0.85,
                    ..self.color
                },
            );
        }
        let cue_x = ((self.cue - viewport.start) / viewport.span) as f32 * bounds.width;
        if (0.0..bounds.width).contains(&cue_x) {
            frame.stroke(
                &Path::line(Point::new(cue_x, 0.0), Point::new(cue_x, bounds.height)),
                Stroke::default().with_color(self.color).with_width(1.0),
            );
        }
        let playhead = ((self.position - viewport.start) / viewport.span) as f32 * bounds.width;
        frame.fill_rectangle(
            Point::new(playhead - 1.0, 0.0),
            Size::new(2.0, bounds.height),
            Color::from_rgb(0.9, 0.94, 0.98),
        );
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.drag.is_some() {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) && self.waveform.is_some() {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overview_clicks_and_zoomed_waveform_drags_seek_the_correct_audio_time() {
        let overview = Viewport::new(20.0, 100.0, 16.0, true);
        assert_eq!(overview.seek_at(0.75), 75.0);
        let detail = Viewport::new(20.0, 100.0, 16.0, false);
        assert_eq!(detail.seek_at(0.75), 24.0);
        assert_eq!(detail.drag_seek(20.0, -0.25), 24.0);
        assert_eq!(detail.drag_seek(20.0, 2.0), 0.0);
        assert_eq!(detail.seek_at(50.0), 100.0);
    }
}
