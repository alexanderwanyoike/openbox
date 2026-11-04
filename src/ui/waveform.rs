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
    fn switching_waveform_themes_recolors_cached_overviews_without_reloading_audio() {
        use super::super::appearance::WaveformTheme;
        use canvas::Program;
        use iced_graphics::{core::Renderer as _, geometry::Renderer as _};
        let samples = (0..44100)
            .flat_map(|n| {
                let value =
                    ((n as f64 * 80.0 * std::f64::consts::TAU / 44100.0).sin() * 25000.0) as i16;
                [value, value]
            })
            .collect();
        let clip = openbox_audio::Clip::from_pcm(samples, 44100).unwrap();
        let mut renderer = Renderer::new(iced::Font::DEFAULT, 16.0.into());
        let state = State::default();
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(320.0, 80.0));
        let mut rendered = Vec::new();
        for theme in WaveformTheme::ALL {
            let wave = Wave {
                waveform: Some(&clip.waveform),
                deck: DeckId::A,
                position: 0.5,
                playing: false,
                rate: 1.0,
                sampled_at: Instant::now(),
                cue: 0.0,
                span: 1.0,
                overview: true,
                color: Color::WHITE,
                beats: &[],
                theme,
            };
            renderer.reset(bounds);
            for geometry in wave.draw(
                &state,
                &renderer,
                &Theme::Dark,
                bounds,
                mouse::Cursor::Unavailable,
            ) {
                renderer.draw_geometry(geometry);
            }
            let mut pixels = tiny_skia::Pixmap::new(320, 80).unwrap();
            renderer.draw(
                &mut pixels.as_mut(),
                &mut tiny_skia::Mask::new(320, 80).unwrap(),
                &iced_graphics::Viewport::with_physical_size(iced::Size::new(320, 80), 1.0),
                &[bounds],
                Color::BLACK,
            );
            let pixels = pixels.data().to_vec();
            assert!(
                !rendered.contains(&pixels),
                "{} reused another theme's cached pixels",
                theme
            );
            rendered.push(pixels);
        }
    }

    #[test]
    fn bass_stays_blue_ahead_of_the_playhead_and_warm_behind_it() {
        use canvas::Program;
        use iced_graphics::{core::Renderer as _, geometry::Renderer as _};
        let samples = (0..44100 * 4)
            .flat_map(|n| {
                let value =
                    ((n as f64 * 80.0 * std::f64::consts::TAU / 44100.0).sin() * 25000.0) as i16;
                [value, value]
            })
            .collect();
        let clip = openbox_audio::Clip::from_pcm(samples, 44100).unwrap();
        let wave = Wave {
            waveform: Some(&clip.waveform),
            deck: DeckId::A,
            position: 2.0,
            cue: 0.0,
            playing: false,
            rate: 1.0,
            sampled_at: Instant::now(),
            span: 4.0,
            overview: false,
            color: Color::WHITE,
            beats: &[],
            theme: WaveformTheme::Classic,
        };
        let mut renderer = Renderer::new(iced::Font::DEFAULT, 16.0.into());
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(570.0, 174.0));
        renderer.reset(bounds);
        for geometry in wave.draw(
            &State::default(),
            &renderer,
            &Theme::Dark,
            bounds,
            mouse::Cursor::Unavailable,
        ) {
            renderer.draw_geometry(geometry);
        }
        let mut pixels = tiny_skia::Pixmap::new(570, 174).unwrap();
        let mut mask = tiny_skia::Mask::new(570, 174).unwrap();
        renderer.draw(
            &mut pixels.as_mut(),
            &mut mask,
            &iced_graphics::Viewport::with_physical_size(iced::Size::new(570, 174), 1.0),
            &[bounds],
            Color::BLACK,
        );
        let mut future_blue = 0;
        let mut past_warm = 0;
        // The software renderer writes the native window surface in BGRA order.
        for (i, pixel) in pixels.data().chunks_exact(4).enumerate() {
            let x = i % 570;
            if x > 300 && pixel[0] > 180 && pixel[2] < 50 {
                future_blue += 1;
            }
            if x < 270 && pixel[2] > 180 && pixel[1] > 100 && pixel[0] < 50 {
                past_warm += 1;
            }
        }
        assert!(
            future_blue > 100,
            "Upcoming bass was overpainted: {future_blue} blue pixels"
        );
        assert!(past_warm > 100, "Played waveform lost its warm color");
    }

    #[test]
    #[ignore = "manual software raster timing harness"]
    fn profile_waveform_raster() {
        use canvas::Program;
        use std::{hint::black_box, time::Instant};
        let samples = (0..44100 * 120 * 2)
            .map(|n| {
                let t = (n / 2) as f32 / 44100.0;
                ((t * 440.0 * std::f32::consts::TAU).sin()
                    * (0.15 + 0.7 * (-20.0 * (t % 0.5)).exp())
                    * 32767.0) as i16
            })
            .collect();
        let clip = openbox_audio::Clip::from_pcm(samples, 44100).unwrap();
        let mut renderer = Renderer::new(iced::Font::DEFAULT, 16.0.into());
        let mut pixels = tiny_skia::Pixmap::new(570, 174).unwrap();
        let mut mask = tiny_skia::Mask::new(570, 174).unwrap();
        let viewport = iced_graphics::Viewport::with_physical_size(iced::Size::new(570, 174), 1.0);
        use iced_graphics::core::Renderer as _;
        use iced_graphics::geometry::Renderer as _;
        for overview in [true, false] {
            let state = State::default();
            let mut timings = Vec::new();
            for n in 0..240 {
                let wave = Wave {
                    waveform: Some(&clip.waveform),
                    deck: DeckId::A,
                    position: 20.0 + n as f64 / 60.0,
                    cue: 0.0,
                    playing: false,
                    rate: 1.0,
                    sampled_at: Instant::now(),
                    span: 16.0,
                    overview,
                    color: Color::from_rgb(0.25, 0.65, 0.98),
                    beats: &[],
                    theme: WaveformTheme::Classic,
                };
                let start = Instant::now();
                let bounds = Rectangle::new(
                    Point::ORIGIN,
                    Size::new(570.0, if overview { 28.0 } else { 174.0 }),
                );
                renderer.reset(bounds);
                for geometry in wave.draw(
                    &state,
                    &renderer,
                    &Theme::Dark,
                    bounds,
                    mouse::Cursor::Unavailable,
                ) {
                    renderer.draw_geometry(geometry);
                }
                renderer.draw(
                    &mut pixels.as_mut(),
                    &mut mask,
                    &viewport,
                    &[bounds],
                    Color::BLACK,
                );
                black_box(pixels.data());
                timings.push(start.elapsed().as_secs_f64() * 1000.0);
            }
            timings.sort_by(f64::total_cmp);
            println!(
                "{overview:?}: raster p50={:.3}ms p95={:.3}ms",
                timings[120], timings[228]
            );
        }
    }

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
