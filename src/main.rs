use std::collections::HashMap;

use iced::border;
use iced::widget::{
    button, column, container, row, text, text_input, Canvas, Column,
};
use iced::{
    Color, Element, Font, Length, Point, Rectangle, Size, Theme, Vector,
};
use iced::widget::canvas::{self, Frame, Geometry, Path, Program};
use iced::mouse;

type Id = u64;

#[derive(Debug, Clone)]
struct NodeData {
    id: Id,
    text: String,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Debug, Clone)]
struct GroupData {
    id: Id,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    children: Vec<Id>,
}

#[derive(Debug, Clone)]
enum Item {
    Node(NodeData),
    Group(GroupData),
}

impl Item {
    #[allow(dead_code)]
    fn id(&self) -> Id {
        match self {
            Item::Node(n) => n.id,
            Item::Group(g) => g.id,
        }
    }

    fn x(&self) -> f32 {
        match self {
            Item::Node(n) => n.x,
            Item::Group(g) => g.x,
        }
    }

    fn y(&self) -> f32 {
        match self {
            Item::Node(n) => n.y,
            Item::Group(g) => g.y,
        }
    }

    fn w(&self) -> f32 {
        match self {
            Item::Node(n) => n.w,
            Item::Group(g) => g.w,
        }
    }

    fn h(&self) -> f32 {
        match self {
            Item::Node(n) => n.h,
            Item::Group(g) => g.h,
        }
    }

    fn bounds(&self) -> Rectangle {
        Rectangle::new(Point::new(self.x(), self.y()), Size::new(self.w(), self.h()))
    }

    fn center(&self) -> Point {
        Point::new(self.x() + self.w() / 2.0, self.y() + self.h() / 2.0)
    }

    fn contains(&self, point: Point) -> bool {
        point.x >= self.x()
            && point.x <= self.x() + self.w()
            && point.y >= self.y()
            && point.y <= self.y() + self.h()
    }
}

#[derive(Debug, Clone)]
struct Connection {
    from: Id,
    to: Id,
}

#[derive(Debug, Clone)]
enum Message {
    NewNode,
    NewGroup,
    DeleteSelected,
    SelectAndDrag(Id, f32, f32),
    DragMove(f32, f32),
    DragEnd,
    EditNodeText(Id, String),
    ToggleConnectionMode,
    StartConnection(Id),
    EndConnection(Id),
    PanStart(Point),
    PanMove(Point),
    PanEnd,
}

const NODE_W: f32 = 160.0;
const NODE_H: f32 = 80.0;
const GROUP_W: f32 = 300.0;
const GROUP_H: f32 = 200.0;
const SIDEBAR_W: f32 = 220.0;

const NODE_COLOR: Color = Color::WHITE;
const GROUP_COLOR: Color = Color::from_rgba(0.7, 0.85, 1.0, 0.3);
const GROUP_BORDER: Color = Color::from_rgb(0.3, 0.5, 0.8);
const SELECT_COLOR: Color = Color::from_rgb(0.2, 0.4, 0.9);
const CONNECTION_COLOR: Color = Color::from_rgb(0.3, 0.3, 0.3);
const TEXT_COLOR: Color = Color::from_rgb(0.1, 0.1, 0.1);
const CANVAS_BG: Color = Color::from_rgb(0.95, 0.95, 0.95);
const GRID_COLOR: Color = Color::from_rgba(0.8, 0.8, 0.8, 0.5);

struct Whiteboard {
    elements: HashMap<Id, Item>,
    order: Vec<Id>,
    connections: Vec<Connection>,
    next_id: Id,
    selected: Option<Id>,
    drag: Option<(Id, f32, f32)>,
    pan_x: f32,
    pan_y: f32,
    pan_start: Option<Point>,
    connection_source: Option<Id>,
    connection_mode: bool,
}

impl Whiteboard {
    fn new() -> Self {
        let mut app = Whiteboard {
            elements: HashMap::new(),
            order: Vec::new(),
            connections: Vec::new(),
            next_id: 0,
            selected: None,
            drag: None,
            pan_x: 0.0,
            pan_y: 0.0,
            pan_start: None,
            connection_source: None,
            connection_mode: false,
        };

        let n1 = app.add_node("Hello".into(), 100.0, 100.0);
        let n2 = app.add_node("World".into(), 400.0, 200.0);
        let n3 = app.add_node("Node 3".into(), 400.0, 100.0);
        let g1 = app.add_group(80.0, 80.0);
        app.add_child_to_group(g1, n1);
        app.add_child_to_group(g1, n2);
        app.connections.push(Connection { from: n1, to: n2 });
        app.connections.push(Connection { from: n3, to: n2 });

        app
    }

    fn add_node(&mut self, text: String, x: f32, y: f32) -> Id {
        let id = self.next_id;
        self.next_id += 1;
        self.elements.insert(
            id,
            Item::Node(NodeData {
                id,
                text,
                x,
                y,
                w: NODE_W,
                h: NODE_H,
            }),
        );
        self.order.push(id);
        id
    }

    fn add_group(&mut self, x: f32, y: f32) -> Id {
        let id = self.next_id;
        self.next_id += 1;
        self.elements.insert(
            id,
            Item::Group(GroupData {
                id,
                x,
                y,
                w: GROUP_W,
                h: GROUP_H,
                children: Vec::new(),
            }),
        );
        self.order.push(id);
        id
    }

    fn add_child_to_group(&mut self, group_id: Id, child_id: Id) {
        if let Some(Item::Group(g)) = self.elements.get_mut(&group_id) {
            g.children.push(child_id);
        }
    }

    fn find_element_at(&self, point: Point) -> Option<Id> {
        self.order.iter().rev().find_map(|id| {
            self.elements.get(id).and_then(|e| {
                if e.contains(point) { Some(*id) } else { None }
            })
        })
    }

    fn move_element(&mut self, id: Id, dx: f32, dy: f32) {
        if let Some(elem) = self.elements.get_mut(&id) {
            match elem {
                Item::Node(n) => {
                    n.x += dx;
                    n.y += dy;
                }
                Item::Group(g) => {
                    g.x += dx;
                    g.y += dy;
                    for child in &g.children.clone() {
                        self.move_element(*child, dx, dy);
                    }
                }
            }
        }
    }

    fn delete_element(&mut self, id: Id) {
        self.order.retain(|o| *o != id);
        self.elements.remove(&id);
        self.connections.retain(|c| c.from != id && c.to != id);
        for elem in self.elements.values_mut() {
            if let Item::Group(g) = elem {
                g.children.retain(|c| *c != id);
            }
        }
        if self.selected == Some(id) {
            self.selected = None;
        }
    }

    fn edge_point(rect: Rectangle, target: Point) -> Point {
        let cx = rect.x + rect.width / 2.0;
        let cy = rect.y + rect.height / 2.0;
        let dx = target.x - cx;
        let dy = target.y - cy;

        if dx == 0.0 && dy == 0.0 {
            return Point::new(cx, cy);
        }

        let half_w = rect.width / 2.0;
        let half_h = rect.height / 2.0;

        if dx.abs() * half_h > dy.abs() * half_w {
            let x = if dx > 0.0 { rect.x + rect.width } else { rect.x };
            let y = cy + dy * (x - cx) / dx;
            Point::new(x, y)
        } else {
            let y = if dy > 0.0 { rect.y + rect.height } else { rect.y };
            let x = cx + dx * (y - cy) / dy;
            Point::new(x, y)
        }
    }
}

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
        let mut frame = Frame::new(renderer, bounds.size());

        frame.fill_rectangle(Point::ORIGIN, bounds.size(), CANVAS_BG);

        for x in (0..(bounds.width as i32)).step_by(40) {
            for y in (0..(bounds.height as i32)).step_by(40) {
                frame.fill_rectangle(
                    Point::new(x as f32, y as f32),
                    Size::new(1.0, 1.0),
                    GRID_COLOR,
                );
            }
        }

        frame.translate(Vector::new(self.pan_x, self.pan_y));

        for conn in &self.connections {
            let from = self.elements.get(&conn.from);
            let to = self.elements.get(&conn.to);
            if let (Some(from_elem), Some(to_elem)) = (from, to) {
                let start = Whiteboard::edge_point(from_elem.bounds(), to_elem.center());
                let end = Whiteboard::edge_point(to_elem.bounds(), from_elem.center());
                draw_connection(&mut frame, start, end);
            }
        }

        if self.connection_source.is_some() {
            if let mouse::Cursor::Available(cursor_pos) = _cursor {
                let world_pos = Point::new(
                    cursor_pos.x - bounds.x - self.pan_x,
                    cursor_pos.y - bounds.y - self.pan_y,
                );
                if let Some(src) = self.elements.get(&self.connection_source.unwrap()) {
                    let start = src.center();
                    let dashed = Path::line(start, world_pos);
                    frame.stroke(
                        &dashed,
                        canvas::Stroke::default()
                            .with_width(1.5)
                            .with_color(Color::from_rgba(0.3, 0.3, 0.3, 0.6)),
                    );
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
            }
        }

        vec![frame.into_geometry()]
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

                let world_pos =
                    Point::new(screen_pos.x - self.pan_x, screen_pos.y - self.pan_y);

                if self.connection_mode {
                    if let Some(id) = self.find_element_at(world_pos) {
                        if self.connection_source.is_none() {
                            return Some(canvas::Action::publish(Message::StartConnection(id)));
                        } else if self.connection_source == Some(id) {
                            return Some(canvas::Action::publish(Message::ToggleConnectionMode));
                        } else {
                            return Some(canvas::Action::publish(Message::EndConnection(id)));
                        }
                    }
                    return Some(canvas::Action::capture());
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

                Some(canvas::Action::publish(Message::PanStart(screen_pos)))
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let screen_pos = cursor_pos?;

                if self.drag.is_some() {
                    if let Some((_id, ox, oy)) = self.drag {
                        let new_x = screen_pos.x - self.pan_x - ox;
                        let new_y = screen_pos.y - self.pan_y - oy;
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
            let world = Point::new(pos.x - self.pan_x, pos.y - self.pan_y);
            if self.find_element_at(world).is_some() {
                return mouse::Interaction::Pointer;
            }
        }
        mouse::Interaction::default()
    }
}

fn new() -> Whiteboard {
    Whiteboard::new()
}

fn update(app: &mut Whiteboard, message: Message) {
    match message {
        Message::NewNode => {
            let x = 100.0 - app.pan_x;
            let y = 100.0 - app.pan_y;
            let id = app.add_node("New Node".into(), x, y);
            app.selected = Some(id);
        }
        Message::NewGroup => {
            let x = 80.0 - app.pan_x;
            let y = 80.0 - app.pan_y;
            let id = app.add_group(x, y);
            app.selected = Some(id);
        }
        Message::DeleteSelected => {
            if let Some(id) = app.selected {
                app.delete_element(id);
            }
        }
        Message::SelectAndDrag(id, ox, oy) => {
            app.selected = Some(id);
            app.connection_source = None;
            app.drag = Some((id, ox, oy));
        }
        Message::DragMove(x, y) => {
            if let Some((id, _, _)) = app.drag {
                if let Some(elem) = app.elements.get(&id) {
                    let dx = x - elem.x();
                    let dy = y - elem.y();
                    app.move_element(id, dx, dy);
                }
            }
        }
        Message::DragEnd => {
            app.drag = None;
        }
        Message::EditNodeText(id, text) => {
            if let Some(Item::Node(n)) = app.elements.get_mut(&id) {
                n.text = text;
            }
        }
        Message::ToggleConnectionMode => {
            app.connection_mode = !app.connection_mode;
            app.connection_source = None;
        }
        Message::StartConnection(id) => {
            app.connection_source = Some(id);
        }
        Message::EndConnection(id) => {
            if let Some(src) = app.connection_source {
                if src != id {
                    app.connections.push(Connection { from: src, to: id });
                }
            }
            app.connection_source = None;
        }
        Message::PanStart(pos) => {
            app.pan_start = Some(pos);
        }
        Message::PanMove(pos) => {
            if let Some(start) = app.pan_start {
                let dx = pos.x - start.x;
                let dy = pos.y - start.y;
                app.pan_x += dx;
                app.pan_y += dy;
                app.pan_start = Some(pos);
            }
        }
        Message::PanEnd => {
            app.pan_start = None;
        }
    }
}

fn view(app: &Whiteboard) -> Element<'_, Message> {
    let canvas = Canvas::new(app).width(Length::Fill).height(Length::Fill);

    let mut sidebar_children: Column<Message> = column![
        row![
            button("+ Node").on_press(Message::NewNode),
            button("+ Group").on_press(Message::NewGroup),
        ]
        .spacing(5),
        row![
            button(if app.connection_mode { "➜ On" } else { "➜ Connect" })
                .on_press(Message::ToggleConnectionMode),
            button("Delete").on_press(Message::DeleteSelected),
        ]
        .spacing(5),
        text("").size(10),
    ]
    .spacing(5)
    .padding(10);

    if let Some(id) = app.selected {
        sidebar_children = sidebar_children.push(text("").size(5));
        sidebar_children = sidebar_children.push(text("Selected:").size(14));
        if let Some(elem) = app.elements.get(&id) {
            let label = match elem {
                Item::Node(n) => format!("Node {}: {}", n.id, n.text),
                Item::Group(g) => {
                    format!("Group {} ({} children)", g.id, g.children.len())
                }
            };
            sidebar_children = sidebar_children.push(text(label).size(12));

            if let Item::Node(n) = elem {
                sidebar_children = sidebar_children.push(text("").size(5));
                sidebar_children = sidebar_children.push(text("Text:").size(12));
                sidebar_children =
                    sidebar_children.push(text_input::TextInput::new(
                        "Type here...",
                        &n.text,
                    )
                    .on_input(move |s| Message::EditNodeText(id, s))
                    .size(14));
            }
        }
    }

    sidebar_children = sidebar_children.push(
        column![].height(Length::Fill),
    );

    let sidebar = container(sidebar_children)
        .width(SIDEBAR_W)
        .height(Length::Fill)
        .style(|_theme: &Theme| {
            container::Style::default()
                .background(Color::from_rgb(0.98, 0.98, 0.98))
                .border(
                    border::color(Color::from_rgb(0.85, 0.85, 0.85))
                        .width(1.0),
                )
        });

    row![canvas, sidebar].into()
}

fn theme(_app: &Whiteboard) -> Theme {
    Theme::Light
}

fn main() -> iced::Result {
    iced::application(new, update, view)
        .theme(theme)
        .window_size(iced::Size::new(1200.0, 800.0))
        .centered()
        .run()
}
