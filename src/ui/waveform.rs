use super::{
    Message,
    appearance::{WavePalette, WaveformTheme},
    preview,
};
use iced::{
    Color, Point, Rectangle, Renderer, Size, Theme, mouse,
    widget::{
        Action,
        canvas::{self, Frame, Geometry, Path, Stroke},
    },
};
use openbox_audio::{DeckControl, DeckId, Waveform};
use std::{
    cell::RefCell,
    sync::Arc,
    time::{Duration, Instant},
};

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
    pub theme: WaveformTheme,
    pub waveform: Option<&'a Waveform>,
    pub deck: DeckId,
    pub position: f64,
    pub playing: bool,
    pub rate: f64,
    pub sampled_at: Instant,
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
    overview: canvas::Cache,
    source: RefCell<Option<(Waveform, WaveformTheme)>>,
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
        if let canvas::Event::Window(iced::window::Event::RedrawRequested(at)) = event
            && self.playing
            && self.position_at(*at) < self.waveform.map_or(0.0, |w| w.duration)
        {
            return Some(Action::request_redraw_at(*at + Duration::from_millis(16)));
        }
        let waveform = self.waveform?;
        let position = self.position_at(Instant::now());
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
                let viewport = Viewport::new(position, waveform.duration, self.span, self.overview);
                state.drag = Some(Drag {
                    x: point.x,
                    position,
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
        state: &State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let colors = self.theme.palette();
        let background = colors.bg;
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), background);
        let Some(waveform) = self.waveform else {
            frame.fill_text(canvas::Text {
                content: if self.overview {
                    "Full track overview"
                } else {
                    "Select a track below and load this deck"
                }
                .into(),
                position: Point::new(10.0, bounds.height / 2.0 - 6.0),
                max_width: (bounds.width - 20.0).max(1.0),
                color: colors.text,
                size: 11.0.into(),
                ..Default::default()
            });
            return vec![frame.into_geometry()];
        };
        let position = self.position_at(Instant::now());
        let viewport = Viewport::new(position, waveform.duration, self.span, self.overview);
        let mut geometries = Vec::with_capacity(3);
        if self.overview {
            let mut source = state.source.borrow_mut();
            if source.as_ref().is_none_or(|(old, theme)| {
                *theme != self.theme || !Arc::ptr_eq(&old.peaks, &waveform.peaks)
            }) {
                state.overview.clear();
                *source = Some((waveform.clone(), self.theme));
            }
            geometries.push(state.overview.draw(renderer, bounds.size(), |cached| {
                cached.fill_rectangle(Point::ORIGIN, bounds.size(), background);
                draw_envelope(cached, waveform, viewport, true, 0.0, colors);
            }));
            frame = Frame::new(renderer, bounds.size());
            let played = bounds.width * (position / waveform.duration.max(0.001)) as f32;
            frame.fill_rectangle(
                Point::ORIGIN,
                Size::new(played, bounds.height),
                Color {
                    a: 0.10,
                    ..self.color
                },
            );
            let left = ((position - self.span / 2.0) / waveform.duration).clamp(0.0, 1.0) as f32
                * bounds.width;
            let right = ((position + self.span / 2.0) / waveform.duration).clamp(0.0, 1.0) as f32
                * bounds.width;
            frame.stroke_rectangle(
                Point::new(left, 1.0),
                Size::new(right - left, bounds.height - 2.0),
                Stroke::default().with_color(Color {
                    a: 0.6,
                    ..self.color
                }),
            );
        } else {
            let ticks = super::ruler::beats(
                self.beats,
                viewport.start,
                viewport.span,
                waveform.duration,
                bounds.width,
            );
            for tick in &ticks {
                if tick.major {
                    frame.fill_rectangle(
                        Point::new(tick.x, 18.0),
                        Size::new(1.0, bounds.height - 38.0),
                        Color {
                            a: 0.12,
                            ..colors.bar
                        },
                    );
                }
                frame.fill_rectangle(
                    Point::new(tick.x, 0.0),
                    Size::new(1.0, if tick.major { 17.0 } else { 7.0 }),
                    if tick.major { colors.bar } else { colors.tick },
                );
                if let Some(label) = &tick.label {
                    outline_label(
                        &mut frame,
                        canvas::Text {
                            content: label.clone(),
                            position: Point::new(tick.x + 4.0, 4.0),
                            size: 10.0.into(),
                            color: colors.text,
                            ..Default::default()
                        },
                    );
                }
            }
            frame.fill_rectangle(
                Point::new(0.0, 18.0),
                Size::new(bounds.width, 1.0),
                Color {
                    a: 0.25,
                    ..colors.tick
                },
            );
            frame.fill_rectangle(
                Point::new(0.0, bounds.height - 20.0),
                Size::new(bounds.width, 1.0),
                Color {
                    a: 0.25,
                    ..colors.tick
                },
            );
            for tick in super::ruler::time(
                viewport.start,
                viewport.span,
                waveform.duration,
                bounds.width,
            ) {
                frame.fill_rectangle(
                    Point::new(tick.x, bounds.height - 20.0),
                    Size::new(1.0, if tick.major { 7.0 } else { 3.0 }),
                    colors.tick,
                );
                if let Some(label) = tick.label {
                    outline_label(
                        &mut frame,
                        canvas::Text {
                            content: label,
                            position: Point::new(tick.x + 4.0, bounds.height - 12.0),
                            size: 9.0.into(),
                            color: colors.text,
                            ..Default::default()
                        },
                    );
                }
            }
            draw_envelope(
                &mut frame,
                waveform,
                viewport,
                false,
                bounds.width / 2.0,
                colors,
            );
        }
        let cue_x = ((self.cue - viewport.start) / viewport.span) as f32 * bounds.width;
        if (0.0..bounds.width).contains(&cue_x) {
            frame.fill_rectangle(
                Point::new(cue_x, 18.0),
                Size::new(1.0, bounds.height - 36.0),
                Color {
                    a: 0.6,
                    ..self.color
                },
            );
        }
        let playhead = ((position - viewport.start) / viewport.span) as f32 * bounds.width;
        frame.fill_rectangle(
            Point::new(playhead - 1.0, 0.0),
            Size::new(2.0, bounds.height),
            colors.playhead,
        );
        if !self.overview {
            frame.fill(
                &Path::new(|p| {
                    p.move_to(Point::new(playhead, bounds.height - 8.0));
                    p.line_to(Point::new(playhead - 6.0, bounds.height));
                    p.line_to(Point::new(playhead + 6.0, bounds.height));
                    p.close();
                }),
                colors.marker,
            );
        }
        geometries.push(frame.into_geometry());
        geometries
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

impl Wave<'_> {
    fn position_at(&self, now: Instant) -> f64 {
        preview::position_at(
            openbox_audio::DeckSnapshot {
                position: self.position,
                duration: self.waveform.map_or(0.0, |w| w.duration),
                playing: self.playing,
                rate: self.rate,
                ..Default::default()
            },
            self.sampled_at,
            now,
        )
    }
}

fn outline_label(frame: &mut Frame, label: canvas::Text) {
    // Tiny-skia gives cached canvas text unbounded damage rectangles.
    // Outlines keep moving ruler labels from invalidating the library below.
    label.draw_with(|path, color| frame.fill(&path, color));
}

fn draw_envelope(
    frame: &mut Frame,
    waveform: &Waveform,
    viewport: Viewport,
    overview: bool,
    playhead: f32,
    colors: WavePalette,
) {
    let size = frame.size();
    let middle = size.height / 2.0;
    let amplitude = middle
        - if overview {
            3.0
        } else {
            25.0_f32.min(middle * 0.3)
        };
    let resolution = viewport.span / f64::from(size.width.max(1.0));
    let points: Vec<_> = (0..=size.width.ceil() as usize)
        .map(|pixel| {
            let peak = waveform.peak_at(viewport.start + pixel as f64 * resolution, resolution);
            let energy = peak.bands.iter().sum::<f32>().max(0.000001);
            let outer = peak.peak * amplitude;
            let mid = outer * ((peak.bands[1] + peak.bands[2]) / energy).min(1.0);
            let core = (outer * peak.bands[2] / energy + peak.rms * amplitude * 0.18).min(mid);
            [outer, mid, core]
        })
        .collect();
    for (layer, color) in colors.bands.into_iter().enumerate() {
        let shape = |values: &[[f32; 3]]| {
            Path::new(|p| {
                p.move_to(Point::new(0.0, middle - values[0][layer]));
                for (x, point) in values.iter().enumerate().skip(1) {
                    p.line_to(Point::new(x as f32, middle - point[layer]));
                }
                for (x, point) in values.iter().enumerate().rev() {
                    p.line_to(Point::new(x as f32, middle + point[layer]));
                }
                p.close();
            })
        };
        let path = shape(&points);
        frame.fill(
            &path,
            if overview && layer == 0 {
                colors.past
            } else {
                color
            },
        );
        if !overview && layer == 0 {
            let past_end = (playhead.floor() as usize + 1).min(points.len());
            frame.fill(&shape(&points[..past_end]), colors.past);
        }
    }
}

pub(super) fn preview(theme: WaveformTheme) -> iced::Element<'static, Message> {
    canvas::Canvas::new(Preview(theme))
        .width(iced::Length::Fill)
        .height(40)
        .into()
}

struct Preview(WaveformTheme);

impl canvas::Program<Message> for Preview {
    type State = canvas::Cache;

    fn draw(
        &self,
        cache: &canvas::Cache,
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        static CLIP: std::sync::OnceLock<openbox_audio::Clip> = std::sync::OnceLock::new();
        let clip = CLIP.get_or_init(|| {
            let samples = (0..44100 * 4)
                .flat_map(|n| {
                    let t = n as f64 / 44100.0;
                    let kick = (-12.0 * (t % 0.5)).exp();
                    let phase = std::f64::consts::TAU * t;
                    let value = ((0.55 * kick * (phase * 80.0).sin()
                        + 0.15 * (phase * 900.0).sin()
                        + 0.1 * (phase * 6000.0).sin())
                        * 32767.0) as i16;
                    [value, value]
                })
                .collect();
            openbox_audio::Clip::from_pcm(samples, 44100).unwrap()
        });
        let colors = self.0.palette();
        vec![cache.draw(renderer, bounds.size(), |frame| {
            frame.fill_rectangle(Point::ORIGIN, bounds.size(), colors.bg);
            draw_envelope(
                frame,
                &clip.waveform,
                Viewport::new(2.0, 4.0, 4.0, false),
                false,
                bounds.width / 2.0,
                colors,
            );
            frame.fill_rectangle(
                Point::new(bounds.width / 2.0, 0.0),
                Size::new(1.0, bounds.height),
                colors.playhead,
            );
        })]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_ruler_labels_only_damage_the_waveform_viewport() {
        use canvas::Program;
        use iced_graphics::{core::Renderer as _, geometry::Renderer as _};
        let clip = openbox_audio::Clip::from_pcm(vec![1000; 44100 * 2 * 40], 44100).unwrap();
        let mut renderer = crate::ui::rendering::new_renderer();
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(570.0, 174.0));
        let state = State::default();
        let mut previous = None;
        for position in [20.0, 20.01] {
            let wave = Wave {
                waveform: Some(&clip.waveform),
                theme: WaveformTheme::Classic,
                deck: DeckId::A,
                position,
                playing: false,
                rate: 1.0,
                sampled_at: Instant::now(),
                cue: 0.0,
                span: 4.0,
                overview: false,
                color: Color::WHITE,
                beats: &[],
            };
            renderer.reset(Rectangle::new(Point::ORIGIN, Size::new(1440.0, 900.0)));
            for geometry in wave.draw(
                &state,
                &renderer,
                &Theme::Dark,
                bounds,
                mouse::Cursor::Unavailable,
            ) {
                renderer.draw_geometry(geometry);
            }
            let current = crate::ui::rendering::backend(&mut renderer).layers().to_vec();
            if let Some(previous) = previous {
                for (before, after) in std::iter::zip(&previous, &current) {
                    for damage in iced_tiny_skia::Layer::damage(before, after) {
                        assert!(damage.width.is_finite() && damage.height.is_finite());
                        assert!(
                            damage.x + damage.width <= 572.0 && damage.y + damage.height <= 176.0,
                            "Ruler repaint escaped the deck: {damage:?}"
                        );
                    }
                }
            }
            previous = Some(current);
        }
    }

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
        let mut renderer = crate::ui::rendering::new_renderer();
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
            crate::ui::rendering::backend(&mut renderer).draw(
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
        let mut renderer = crate::ui::rendering::new_renderer();
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
        crate::ui::rendering::backend(&mut renderer).draw(
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
        let mut renderer = crate::ui::rendering::new_renderer();
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
                crate::ui::rendering::backend(&mut renderer).draw(
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
