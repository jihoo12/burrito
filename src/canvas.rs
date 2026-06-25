use iced::border;
use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path, Program};
use iced::{Color, Font, Point, Rectangle, Size, Theme, Vector};

use crate::app::Whiteboard;
use crate::types::*;

fn draw_connection(frame: &mut Frame, from: Point, to: Point) {
    let len = ((to.x - from.x).powi(2) + (to.y - from.y).powi(2)).sqrt();
    if len < 1.0 {
        return;
    }

    let path = Path::line(from, to);
    frame.stroke(
        &path,
        canvas::Stroke::default()
            .with_width(2.0)
            .with_color(CONNECTION_COLOR),
    );

    let nx = (to.x - from.x) / len;
    let ny = (to.y - from.y) / len;
    let arrow_len = 12.0;
    let arrow_w = 6.0;
    let px = -ny;
    let py = nx;

    let base_x = to.x - nx * arrow_len;
    let base_y = to.y - ny * arrow_len;
    let p1 = Point::new(base_x + px * arrow_w, base_y + py * arrow_w);
    let p2 = Point::new(base_x - px * arrow_w, base_y - py * arrow_w);

    let arrow = Path::new(|b| {
        b.move_to(to);
        b.line_to(p1);
        b.line_to(p2);
        b.close();
    });
    frame.fill(&arrow, CONNECTION_COLOR);
}

impl Program<Message> for Whiteboard {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let grid = self.grid_cache.draw(renderer, bounds.size(), |frame| {
            frame.fill_rectangle(Point::ORIGIN, bounds.size(), CANVAS_BG);

            let path = Path::new(|b| {
                for x in (0..(bounds.width as i32)).step_by(40) {
                    for y in (0..(bounds.height as i32)).step_by(40) {
                        b.rectangle(
                            Point::new(x as f32, y as f32),
                            Size::new(1.0, 1.0),
                        );
                    }
                }
            });
            frame.fill(&path, GRID_COLOR);
        });

        let mut frame = Frame::new(renderer, bounds.size());

        frame.translate(Vector::new(self.pan_x, self.pan_y));
        frame.scale(self.zoom);

        for (i, conn) in self.connections.iter().enumerate() {
            let from = self.elements.get(&conn.from);
            let to = self.elements.get(&conn.to);
            if let (Some(from_elem), Some(to_elem)) = (from, to) {
                let start = edge_point(from_elem.bounds(), to_elem.center());
                let end = edge_point(to_elem.bounds(), from_elem.center());
                if self.selected_connection == Some(i) {
                    let path = Path::line(start, end);
                    frame.stroke(
                        &path,
                        canvas::Stroke::default()
                            .with_width(3.0)
                            .with_color(SELECT_COLOR),
                    );
                } else {
                    draw_connection(&mut frame, start, end);
                }
            }
        }

        for id in &self.order {
            if let Some(Item::Group(g)) = self.elements.get(id) {
                let radius = border::Radius::new(8.0);
                let path = Path::rounded_rectangle(
                    Point::new(g.x, g.y),
                    Size::new(g.w, g.h),
                    radius,
                );
                frame.fill(&path, GROUP_COLOR);
                frame.stroke(
                    &path,
                    canvas::Stroke::default()
                        .with_width(2.0)
                        .with_color(GROUP_BORDER),
                );

                frame.fill_text(canvas::Text {
                    content: format!("Group {}", g.id),
                    position: Point::new(g.x + 10.0, g.y + 8.0),
                    color: GROUP_BORDER,
                    size: iced::Pixels(13.0),
                    font: Font::default(),
                    ..Default::default()
                });
            }
        }

        for id in &self.order {
            if let Some(Item::Node(n)) = self.elements.get(id) {
                let radius = border::Radius::new(6.0);
                let path = Path::rounded_rectangle(
                    Point::new(n.x, n.y),
                    Size::new(n.w, n.h),
                    radius,
                );
                frame.fill(&path, NODE_COLOR);
                frame.stroke(
                    &path,
                    canvas::Stroke::default()
                        .with_width(1.5)
                        .with_color(Color::from_rgb(0.6, 0.6, 0.6)),
                );

                frame.fill_text(canvas::Text {
                    content: n.text.clone(),
                    position: Point::new(n.x + 10.0, n.y + 8.0),
                    color: TEXT_COLOR,
                    size: iced::Pixels(14.0),
                    max_width: n.w - 20.0,
                    font: Font::default(),
                    ..Default::default()
                });
            }
        }

        if let Some(id) = self.selected {
            if let Some(elem) = self.elements.get(&id) {
                let rect = elem.bounds();
                let radius = border::Radius::new(match elem {
                    Item::Node(_) => 6.0,
                    Item::Group(_) => 8.0,
                });
                let path = Path::rounded_rectangle(
                    Point::new(rect.x - 2.0, rect.y - 2.0),
                    Size::new(rect.width + 4.0, rect.height + 4.0),
                    radius,
                );
                frame.stroke(
                    &path,
                    canvas::Stroke::default()
                        .with_width(2.5)
                        .with_color(SELECT_COLOR),
                );

                let handle = Path::rectangle(
                    Point::new(rect.x + rect.width - 12.0, rect.y + rect.height - 12.0),
                    Size::new(12.0, 12.0),
                );
                frame.fill(&handle, Color::WHITE);
                frame.stroke(
                    &handle,
                    canvas::Stroke::default()
                        .with_width(1.5)
                        .with_color(SELECT_COLOR),
                );
            }
        }

        vec![grid, frame.into_geometry()]
    }

    fn update(
        &self,
        _state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let cursor_pos = cursor.position_in(bounds);

        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let screen_pos = cursor_pos?;

                if self.drag.is_some() || self.pan_start.is_some() {
                    return None;
                }

                let world_pos = Point::new(
                    (screen_pos.x - self.pan_x) / self.zoom,
                    (screen_pos.y - self.pan_y) / self.zoom,
                );

                if self.connection_mode {
                    if let Some(id) = self.find_element_at(world_pos) {
                        if self.connection_source.is_none() || self.connection_source == Some(id) {
                            return Some(canvas::Action::publish(Message::StartConnection(id)));
                        } else {
                            return Some(canvas::Action::publish(Message::EndConnection(id)));
                        }
                    }
                    return Some(canvas::Action::capture());
                }

                if let Some(id) = self.selected {
                    if let Some(elem) = self.elements.get(&id) {
                        let hit_size = 14.0;
                        let handle_rect = Rectangle::new(
                            Point::new(
                                elem.x() + elem.w() - hit_size,
                                elem.y() + elem.h() - hit_size,
                            ),
                            Size::new(hit_size, hit_size),
                        );
                        if handle_rect.contains(world_pos) {
                            return Some(canvas::Action::publish(Message::ResizeStart(id)));
                        }
                    }
                }

                if let Some(id) = self.find_element_at(world_pos) {
                    if let Some(elem) = self.elements.get(&id) {
                        let ox = world_pos.x - elem.x();
                        let oy = world_pos.y - elem.y();
                        return Some(canvas::Action::publish(Message::SelectAndDrag(
                            id, ox, oy,
                        )));
                    }
                }

                if let Some(idx) = self.find_connection_at(world_pos) {
                    return Some(canvas::Action::publish(Message::SelectConnection(idx)));
                }

                Some(canvas::Action::publish(Message::PanStart(screen_pos)))
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let screen_pos = cursor_pos?;

                if let Some(id) = self.resize {
                    if let Some(elem) = self.elements.get(&id) {
                        let new_w = ((screen_pos.x - self.pan_x) / self.zoom - elem.x()).max(50.0);
                        let new_h = ((screen_pos.y - self.pan_y) / self.zoom - elem.y()).max(30.0);
                        return Some(canvas::Action::publish(Message::ResizeMove(new_w, new_h)));
                    }
                }

                if self.drag.is_some() {
                    if let Some((_id, ox, oy)) = self.drag {
                        let new_x = (screen_pos.x - self.pan_x) / self.zoom - ox;
                        let new_y = (screen_pos.y - self.pan_y) / self.zoom - oy;
                        return Some(canvas::Action::publish(Message::DragMove(
                            new_x, new_y,
                        )));
                    }
                }

                if self.pan_start.is_some() {
                    return Some(canvas::Action::publish(Message::PanMove(screen_pos)));
                }

                None
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if self.resize.is_some() {
                    return Some(canvas::Action::publish(Message::ResizeEnd));
                }
                if self.drag.is_some() {
                    return Some(canvas::Action::publish(Message::DragEnd));
                }
                if self.pan_start.is_some() {
                    return Some(canvas::Action::publish(Message::PanEnd));
                }
                None
            }
            _ => None,
        }
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if let Some(pos) = cursor.position_in(_bounds) {
            let world = Point::new(
                (pos.x - self.pan_x) / self.zoom,
                (pos.y - self.pan_y) / self.zoom,
            );

            if let Some(id) = self.selected {
                if let Some(elem) = self.elements.get(&id) {
                    let hit_size = 14.0;
                    let handle_rect = Rectangle::new(
                        Point::new(
                            elem.x() + elem.w() - hit_size,
                            elem.y() + elem.h() - hit_size,
                        ),
                        Size::new(hit_size, hit_size),
                    );
                    if handle_rect.contains(world) {
                        return mouse::Interaction::ResizingDiagonallyUp;
                    }
                }
            }

            if self.find_element_at(world).is_some() {
                return mouse::Interaction::Pointer;
            }
        }
        mouse::Interaction::default()
    }
}
