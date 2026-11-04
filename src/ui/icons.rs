use super::{Message, view};
use iced::{
    Alignment, Element, Point, Rectangle, Renderer, Theme, mouse,
    widget::{
        Button, Tooltip, button,
        canvas::{self, Frame, Geometry, Path, Stroke},
        container, row, text, tooltip,
    },
};
use std::time::Duration;

#[derive(Clone, Copy)]
pub(super) enum Icon {
    Brand,
    Library,
    Playlist,
    Minimize,
    Maximize,
    Resize,
    Play,
    Pause,
    Back,
    Forward,
    ZoomIn,
    ZoomOut,
    Reset,
    Folder,
    Import,
    Usb,
    Players,
    Analyze,
    Refresh,
    Settings,
    Close,
}

pub(super) fn action(icon: Icon, message: Message, enabled: bool) -> Button<'static, Message> {
    button(container(themed(icon, enabled)).align_x(Alignment::Center))
        .width(30)
        .height(28)
        .padding([6, 7])
        .style(view::quiet)
        .on_press_maybe(enabled.then_some(message))
}

pub(super) fn labeled(
    icon: Icon,
    label: &'static str,
    message: Message,
    enabled: bool,
    width: u32,
) -> Button<'static, Message> {
    button(
        row![themed(icon, enabled), text(label).size(12)]
            .spacing(6)
            .align_y(Alignment::Center),
    )
    .padding([5, 10])
    .width(width)
    .style(view::secondary)
    .on_press_maybe(enabled.then_some(message))
}

pub(super) fn hint<'a>(
    content: impl Into<Element<'a, Message>>,
    label: impl Into<String>,
) -> Tooltip<'a, Message> {
    tooltip(
        content,
        container(text(label.into()).size(11).line_height(1.4)).max_width(280),
        tooltip::Position::Bottom,
    )
    .style(|theme| {
        let p = super::appearance::palette(theme);
        container::Style { background: Some(p.raised.into()), text_color: Some(p.ink), border: iced::Border { color: p.line, width: 1.0, radius: 6.0.into() }, ..Default::default() }
    })
    .delay(Duration::from_millis(500))
    .gap(8)
    .padding(8)
    .snap_within_viewport(true)
}

pub(super) fn themed(icon: Icon, enabled: bool) -> Element<'static, Message> {
    canvas::Canvas::new(Symbol { icon, enabled })
        .width(16)
        .height(16)
        .into()
}

struct Symbol {
    icon: Icon,
    enabled: bool,
}
impl canvas::Program<Message> for Symbol {
    type State = ();
    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let colors = super::appearance::palette(theme);
        let color = if matches!(self.icon, Icon::Brand) {
            colors.accent
        } else if self.enabled {
            colors.ink
        } else {
            colors.muted
        };
        let mut frame = Frame::new(renderer, bounds.size());
        let line = |frame: &mut Frame, points: &[(f32, f32)]| {
            let path = Path::new(|p| {
                p.move_to(Point::new(points[0].0, points[0].1));
                for &(x, y) in &points[1..] {
                    p.line_to(Point::new(x, y));
                }
            });
            frame.stroke(&path, Stroke::default().with_width(1.5).with_color(color));
        };
        match self.icon {
            Icon::Brand => {
                frame.stroke(
                    &Path::circle(Point::new(8.0, 6.0), 4.0),
                    Stroke::default().with_width(1.5).with_color(color),
                );
                line(
                    &mut frame,
                    &[(2.0, 7.0), (2.0, 14.0), (14.0, 14.0), (14.0, 7.0)],
                );
                line(&mut frame, &[(5.0, 10.0), (11.0, 10.0)]);
            }
            Icon::Library => {
                for x in [3.0, 7.0, 12.0] {
                    line(&mut frame, &[(x, 3.0), (x, 13.0)]);
                }
                line(&mut frame, &[(1.0, 14.0), (15.0, 14.0)]);
            }
            Icon::Playlist => {
                for y in [4.0, 8.0, 12.0] {
                    line(&mut frame, &[(3.0, y), (13.0, y)]);
                }
            }
            Icon::Minimize => line(&mut frame, &[(3.0, 11.0), (13.0, 11.0)]),
            Icon::Maximize => line(
                &mut frame,
                &[
                    (3.0, 3.0),
                    (13.0, 3.0),
                    (13.0, 13.0),
                    (3.0, 13.0),
                    (3.0, 3.0),
                ],
            ),
            Icon::Resize => {
                line(&mut frame, &[(5.0, 14.0), (14.0, 5.0)]);
                line(&mut frame, &[(10.0, 14.0), (14.0, 10.0)]);
            }

            Icon::Close => {
                line(&mut frame, &[(3.0, 3.0), (13.0, 13.0)]);
                line(&mut frame, &[(13.0, 3.0), (3.0, 13.0)]);
            }
            Icon::Settings => {
                for n in 0..8 {
                    let angle = n as f32 * std::f32::consts::TAU / 8.0;
                    line(
                        &mut frame,
                        &[
                            (8.0 + angle.cos() * 5.0, 8.0 + angle.sin() * 5.0),
                            (8.0 + angle.cos() * 7.0, 8.0 + angle.sin() * 7.0),
                        ],
                    );
                }
                frame.stroke(
                    &Path::circle(Point::new(8.0, 8.0), 5.0),
                    Stroke::default().with_width(1.5).with_color(color),
                );
                frame.stroke(
                    &Path::circle(Point::new(8.0, 8.0), 2.0),
                    Stroke::default().with_width(1.5).with_color(color),
                );
            }
            Icon::Play => frame.fill(
                &Path::new(|p| {
                    p.move_to(Point::new(4.0, 2.0));
                    p.line_to(Point::new(13.0, 8.0));
                    p.line_to(Point::new(4.0, 14.0));
                    p.close();
                }),
                color,
            ),
            Icon::Pause => {
                for x in [4.0, 10.0] {
                    frame.fill_rectangle(Point::new(x, 3.0), iced::Size::new(3.0, 10.0), color);
                }
            }
            Icon::Back => line(&mut frame, &[(10.0, 3.0), (5.0, 8.0), (10.0, 13.0)]),
            Icon::Forward => line(&mut frame, &[(6.0, 3.0), (11.0, 8.0), (6.0, 13.0)]),
            Icon::ZoomIn | Icon::ZoomOut => {
                frame.stroke(
                    &Path::circle(Point::new(6.5, 6.5), 4.5),
                    Stroke::default().with_width(1.5).with_color(color),
                );
                line(&mut frame, &[(10.0, 10.0), (14.0, 14.0)]);
                line(&mut frame, &[(4.0, 6.5), (9.0, 6.5)]);
                if matches!(self.icon, Icon::ZoomIn) {
                    line(&mut frame, &[(6.5, 4.0), (6.5, 9.0)]);
                }
            }
            Icon::Reset | Icon::Refresh => {
                line(
                    &mut frame,
                    &[
                        (3.0, 6.0),
                        (4.0, 3.0),
                        (9.0, 2.0),
                        (13.0, 5.0),
                        (13.0, 10.0),
                        (10.0, 13.0),
                        (5.0, 13.0),
                        (2.0, 10.0),
                    ],
                );
                line(&mut frame, &[(2.0, 2.0), (2.0, 6.0), (6.0, 6.0)]);
            }
            Icon::Folder => line(
                &mut frame,
                &[
                    (2.0, 13.0),
                    (2.0, 3.0),
                    (6.0, 3.0),
                    (8.0, 5.0),
                    (14.0, 5.0),
                    (14.0, 13.0),
                    (2.0, 13.0),
                ],
            ),
            Icon::Import => {
                line(
                    &mut frame,
                    &[(3.0, 10.0), (3.0, 14.0), (13.0, 14.0), (13.0, 10.0)],
                );
                line(&mut frame, &[(8.0, 1.0), (8.0, 10.0)]);
                line(&mut frame, &[(4.0, 6.0), (8.0, 10.0), (12.0, 6.0)]);
            }
            Icon::Usb => {
                line(
                    &mut frame,
                    &[(5.0, 5.0), (5.0, 1.0), (11.0, 1.0), (11.0, 5.0)],
                );
                line(
                    &mut frame,
                    &[
                        (4.0, 5.0),
                        (12.0, 5.0),
                        (12.0, 14.0),
                        (4.0, 14.0),
                        (4.0, 5.0),
                    ],
                );
            }
            Icon::Players => {
                line(
                    &mut frame,
                    &[
                        (2.0, 12.0),
                        (2.0, 6.0),
                        (5.0, 2.0),
                        (11.0, 2.0),
                        (14.0, 6.0),
                        (14.0, 12.0),
                    ],
                );
                for x in [2.0, 12.0] {
                    frame.stroke_rectangle(
                        Point::new(x, 8.0),
                        iced::Size::new(2.0, 6.0),
                        Stroke::default().with_width(1.5).with_color(color),
                    );
                }
            }
            Icon::Analyze => {
                for (x, height) in [
                    (2.0, 4.0),
                    (5.0, 10.0),
                    (8.0, 14.0),
                    (11.0, 7.0),
                    (14.0, 3.0),
                ] {
                    line(
                        &mut frame,
                        &[(x, 8.0 - height / 2.0), (x, 8.0 + height / 2.0)],
                    );
                }
            }
        }
        vec![frame.into_geometry()]
    }
}
