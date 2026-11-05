use super::Message;
use iced::{
    Color, Element, Point, Rectangle, Renderer, Size, Theme, keyboard, mouse,
    widget::{
        Action,
        canvas::{self, Frame, Geometry, Path, Stroke},
    },
};
use std::{
    ops::RangeInclusive,
    time::{Duration, Instant},
};

pub(super) fn knob<'a>(
    range: RangeInclusive<f32>,
    value: f32,
    default: f32,
    step: f32,
    accent: Color,
    on_change: impl Fn(f32) -> Message + 'a,
) -> Element<'a, Message> {
    canvas::Canvas::new(Knob {
        range,
        value,
        default,
        step,
        accent,
        on_change: Box::new(on_change),
    })
    .width(30)
    .height(30)
    .into()
}

struct Knob<'a> {
    range: RangeInclusive<f32>,
    value: f32,
    default: f32,
    step: f32,
    accent: Color,
    on_change: Box<dyn Fn(f32) -> Message + 'a>,
}

#[derive(Default)]
struct State {
    drag: Option<(f32, f32)>,
    last_click: Option<Instant>,
    modifiers: keyboard::Modifiers,
}

impl Knob<'_> {
    fn fraction(&self, value: f32) -> f32 {
        let low = *self.range.start();
        let high = *self.range.end();
        if low < self.default && self.default < high {
            if value <= self.default {
                0.5 * (value - low) / (self.default - low)
            } else {
                0.5 + 0.5 * (value - self.default) / (high - self.default)
            }
        } else {
            (value - low) / (high - low)
        }
    }

    fn value(&self, fraction: f32) -> f32 {
        let low = *self.range.start();
        let high = *self.range.end();
        let fraction = fraction.clamp(0.0, 1.0);
        let value = if low < self.default && self.default < high {
            if fraction <= 0.5 {
                low + fraction * 2.0 * (self.default - low)
            } else {
                self.default + (fraction - 0.5) * 2.0 * (high - self.default)
            }
        } else {
            low + fraction * (high - low)
        };
        ((value / self.step).round() * self.step).clamp(low, high)
    }
}

impl canvas::Program<Message> for Knob<'_> {
    type State = State;

    fn update(
        &self,
        state: &mut State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        let publish = |value| Action::publish((self.on_change)(value)).and_capture();
        match event {
            canvas::Event::Window(iced::window::Event::Unfocused) => {
                state.drag = None;
                state.last_click = None;
                state.modifiers = keyboard::Modifiers::default();
                Some(Action::request_redraw())
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if cursor.is_over(bounds) =>
            {
                let now = Instant::now();
                if state
                    .last_click
                    .is_some_and(|last| now.duration_since(last) < Duration::from_millis(300))
                {
                    state.last_click = None;
                    state.drag = None;
                    Some(publish(self.default))
                } else {
                    state.last_click = Some(now);
                    state.drag = Some((cursor.position()?.y, self.fraction(self.value)));
                    Some(Action::capture())
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))
                if cursor.is_over(bounds) =>
            {
                Some(publish(self.default))
            }
            canvas::Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
                None
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let (initial, fraction) = state.drag?;
                let distance = initial - cursor.position()?.y;
                if distance.abs() > 2.0 {
                    state.last_click = None;
                }
                let sensitivity = if state.modifiers.shift() {
                    600.0
                } else {
                    150.0
                };
                let value = self.value(fraction + distance / sensitivity);
                Some(if (value - self.value).abs() > self.step * 0.1 {
                    publish(value)
                } else {
                    Action::capture()
                })
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.drag.take().is_some() =>
            {
                Some(Action::capture())
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta })
                if cursor.is_over(bounds) =>
            {
                let y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 30.0,
                };
                Some(publish(
                    (self.value + y * self.step).clamp(*self.range.start(), *self.range.end()),
                ))
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let colors = super::appearance::palette(theme);
        let mut frame = Frame::new(renderer, bounds.size());
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let point = |fraction: f32, radius: f32| {
            let angle = (135.0 + fraction * 270.0).to_radians();
            Point::new(
                center.x + angle.cos() * radius,
                center.y + angle.sin() * radius,
            )
        };
        let arc = |start: f32, end: f32| {
            Path::new(|p| {
                p.move_to(point(start, 12.5));
                for i in 1..=48 {
                    p.line_to(point(start + (end - start) * i as f32 / 48.0, 12.5));
                }
            })
        };
        frame.stroke(
            &arc(0.0, 1.0),
            Stroke::default().with_width(2.0).with_color(colors.line),
        );
        let fraction = self.fraction(self.value);
        let neutral = self.fraction(self.default);
        frame.stroke(
            &arc(neutral.min(fraction), neutral.max(fraction)),
            Stroke::default().with_width(2.5).with_color(self.accent),
        );
        frame.fill(
            &Path::circle(center, 10.5),
            if state.drag.is_some() || cursor.is_over(bounds) {
                colors.line
            } else {
                colors.raised
            },
        );
        frame.stroke(
            &Path::line(point(fraction, 4.0), point(fraction, 9.0)),
            Stroke::default().with_width(2.0).with_color(self.accent),
        );
        frame.fill_rectangle(
            Point::new(center.x - 0.5, 0.0),
            Size::new(1.0, 2.0),
            colors.muted,
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
        } else if cursor.is_over(bounds) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canvas::Program;

    #[test]
    fn consecutive_drags_adjust_the_knob_without_becoming_a_double_click_reset() {
        let knob = Knob {
            range: 0.0..=1.0,
            value: 0.8,
            default: 0.8,
            step: 0.01,
            accent: Color::WHITE,
            on_change: Box::new(|value| {
                Message::Preview(super::super::preview::Event::Master(value))
            }),
        };
        let mut state = State::default();
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(30.0, 30.0));
        let cursor = |y| mouse::Cursor::Available(Point::new(15.0, y));
        let press = canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let release = canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
        assert!(
            knob.update(&mut state, &press, bounds, cursor(15.0))
                .unwrap()
                .into_inner()
                .0
                .is_none()
        );
        let movement = canvas::Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(15.0, 35.0),
        });
        assert!(
            knob.update(&mut state, &movement, bounds, cursor(35.0))
                .unwrap()
                .into_inner()
                .0
                .is_some()
        );
        let _ = knob.update(&mut state, &release, bounds, cursor(35.0));
        assert!(
            knob.update(&mut state, &press, bounds, cursor(15.0))
                .unwrap()
                .into_inner()
                .0
                .is_none()
        );
    }
    #[test]
    fn losing_window_focus_cancels_knob_drag_and_double_click_state() {
        let knob = Knob {
            range: 0.0..=1.0,
            value: 0.8,
            default: 0.5,
            step: 0.01,
            accent: Color::WHITE,
            on_change: Box::new(|value| {
                Message::Preview(super::super::preview::Event::Master(value))
            }),
        };
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(30.0, 30.0));
        let cursor = |y| mouse::Cursor::Available(Point::new(15.0, y));
        let press = canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let mut state = State::default();
        let _ = knob.update(&mut state, &press, bounds, cursor(15.0));
        let _ = knob.update(
            &mut state,
            &canvas::Event::Window(iced::window::Event::Unfocused),
            bounds,
            mouse::Cursor::Unavailable,
        );
        let moved = knob.update(
            &mut state,
            &canvas::Event::Mouse(mouse::Event::CursorMoved {
                position: Point::new(15.0, 70.0),
            }),
            bounds,
            cursor(70.0),
        );
        assert!(
            moved.and_then(|action| action.into_inner().0).is_none(),
            "Focus loss must cancel a knob gesture"
        );
        let next = knob.update(&mut state, &press, bounds, cursor(15.0));
        assert!(
            next.and_then(|action| action.into_inner().0).is_none(),
            "The first new press must start a drag, not reset the value"
        );
    }
}
