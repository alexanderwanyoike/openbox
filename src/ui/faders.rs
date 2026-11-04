use super::{Message, appearance, surfaces};
use iced::{Background, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, gradient, mouse, widget::{canvas, slider, stack}};

#[derive(Clone, Copy)]
pub(super) enum Kind { Pitch, Master, Crossfade }

pub(super) fn horizontal(kind: Kind, value: f64, accent: Color, change: impl Fn(f64) -> Message + 'static) -> Element<'static, Message> {
    let (range, default, step, height) = match kind {
        Kind::Pitch => (-8.0..=8.0, 0.0, 0.05, 24.0),
        Kind::Master => (0.0..=1.0, 0.8, 0.01, 20.0),
        Kind::Crossfade => (0.0..=1.0, 0.5, 0.01, 20.0),
    };
    let neutral = (default - range.start()) / (range.end() - range.start());
    stack![
        canvas(Rail { neutral, accent }).width(Length::Fill).height(height),
        slider(range, value, change).default(default).step(step).shift_step(step / 10.0).height(height)
            .style(move |theme, status| style(theme, status, accent, false))
    ].width(Length::Fill).height(height).into()
}

pub(super) fn vertical_style(theme: &Theme, status: slider::Status, accent: Color) -> slider::Style { style(theme, status, accent, true) }

fn style(theme: &Theme, status: slider::Status, accent: Color, vertical: bool) -> slider::Style {
    let p = appearance::palette(theme);
    let face = surfaces::blend(p.raised, p.ink, 0.10);
    let top = surfaces::blend(face, p.ink, 0.12);
    let bottom = surfaces::blend(face, p.bg, 0.25);
    let grip = surfaces::blend(face, p.ink, 0.6);
    let background = gradient::Linear::new(iced::Radians(if vertical { std::f32::consts::FRAC_PI_2 } else { 0.0 }))
        .add_stop(0.0, top).add_stop(0.20, face).add_stop(0.43, face)
        .add_stop(0.47, grip).add_stop(0.53, grip).add_stop(0.57, face).add_stop(0.80, face).add_stop(1.0, bottom);
    let mut style = slider::default(theme, status);
    style.rail.backgrounds = (p.bg.into(), p.bg.into());
    style.rail.width = if vertical { 5.0 } else { 3.0 };
    style.rail.border = iced::Border { color: p.line, width: 1.0, radius: 2.0.into() };
    style.handle.shape = slider::HandleShape::Rectangle { width: if vertical { 10 } else { 12 }, border_radius: 3.0.into() };
    style.handle.background = Background::Gradient(background.into());
    style.handle.border_width = 1.0;
    style.handle.border_color = if matches!(status, slider::Status::Dragged | slider::Status::Hovered) { accent } else { p.line };
    style
}

struct Rail { neutral: f64, accent: Color }
impl canvas::Program<Message> for Rail {
    type State = ();
    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let p = appearance::palette(theme);
        let center = bounds.height / 2.0;
        for n in 0..=8 {
            let x = 6.0 + (bounds.width - 12.0) * n as f32 / 8.0;
            frame.fill_rectangle(Point::new(x, center - 5.0), Size::new(1.0,10.0), p.line);
        }
        let x = 6.0 + (bounds.width - 12.0) * self.neutral as f32;
        frame.fill_rectangle(Point::new(x - 0.5, center - 7.0), Size::new(1.0,14.0), self.accent);
        vec![frame.into_geometry()]
    }
}
