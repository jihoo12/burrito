use std::collections::HashMap;

use iced::widget::{
    button, column, container, row, text, text_editor, Canvas, Column,
};
use iced::{
    border, Color, Element, Length, Point, Rectangle, Size, Theme,
};

use crate::types::*;

pub struct Whiteboard {
    pub elements: HashMap<Id, Item>,
    pub order: Vec<Id>,
    pub connections: Vec<Connection>,
    pub next_id: Id,
    pub selected: Option<Id>,
    pub drag: Option<(Id, f32, f32)>,
    pub pan_x: f32,
    pub pan_y: f32,
    pub pan_start: Option<Point>,
    pub connection_source: Option<Id>,
    pub connection_mode: bool,
    pub edit_content: Option<text_editor::Content>,
    pub resize: Option<Id>,
    pub selected_connection: Option<usize>,
}

impl Whiteboard {
    pub fn new() -> Self {
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
            edit_content: None,
            resize: None,
            selected_connection: None,
        };

        let n1 = app.add_node("Hello\nThis is a multiline\nnode".into(), 100.0, 100.0);
        let n2 = app.add_node("World\nDrag to move".into(), 400.0, 200.0);
        let n3 = app.add_node("Try editing me\nin the sidebar".into(), 400.0, 100.0);
        let g1 = app.add_group(80.0, 80.0);
        app.add_child_to_group(g1, n1);
        app.add_child_to_group(g1, n2);
        app.connections.push(Connection { from: n1, to: n2 });
        app.connections.push(Connection { from: n3, to: n2 });

        app
    }

    pub fn add_node(&mut self, text: String, x: f32, y: f32) -> Id {
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

    pub fn add_group(&mut self, x: f32, y: f32) -> Id {
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

    pub fn add_child_to_group(&mut self, group_id: Id, child_id: Id) {
        if let Some(Item::Group(g)) = self.elements.get_mut(&group_id) {
            g.children.push(child_id);
        }
    }

    pub fn find_element_at(&self, point: Point) -> Option<Id> {
        for id in self.order.iter().rev() {
            if let Some(e) = self.elements.get(id) {
                if matches!(e, Item::Node(_)) && e.contains(point) {
                    return Some(*id);
                }
            }
        }
        for id in self.order.iter().rev() {
            if let Some(e) = self.elements.get(id) {
                if matches!(e, Item::Group(_)) && e.contains(point) {
                    return Some(*id);
                }
            }
        }
        None
    }

    pub fn find_connection_at(&self, point: Point) -> Option<usize> {
        let threshold = 8.0;
        for (i, conn) in self.connections.iter().enumerate() {
            let from = self.elements.get(&conn.from);
            let to = self.elements.get(&conn.to);
            if let (Some(from_elem), Some(to_elem)) = (from, to) {
                let start = edge_point(from_elem.bounds(), to_elem.center());
                let end = edge_point(to_elem.bounds(), from_elem.center());
                if point_to_segment_distance(point, start, end) < threshold {
                    return Some(i);
                }
            }
        }
        None
    }

    pub fn move_element(&mut self, id: Id, dx: f32, dy: f32) {
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

    pub fn delete_element(&mut self, id: Id) {
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
            self.edit_content = None;
        }
    }
}

pub fn update(app: &mut Whiteboard, message: Message) {
    match message {
        Message::NewNode => {
            let x = 100.0 - app.pan_x;
            let y = 100.0 - app.pan_y;
            let id = app.add_node("New Node".into(), x, y);
            app.selected = Some(id);
            app.edit_content = Some(text_editor::Content::with_text("New Node"));
        }
        Message::NewGroup => {
            let x = 80.0 - app.pan_x;
            let y = 80.0 - app.pan_y;
            let id = app.add_group(x, y);
            app.selected = Some(id);
        }
        Message::DeleteSelected => {
            if let Some(idx) = app.selected_connection {
                app.connections.remove(idx);
                app.selected_connection = None;
            } else if let Some(id) = app.selected {
                app.delete_element(id);
            }
        }
        Message::SelectAndDrag(id, ox, oy) => {
            app.selected = Some(id);
            app.selected_connection = None;
            app.connection_source = None;
            app.drag = Some((id, ox, oy));
            if let Some(Item::Node(n)) = app.elements.get(&id) {
                app.edit_content = Some(text_editor::Content::with_text(&n.text));
            } else {
                app.edit_content = None;
            }
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
            if let Some((id, _, _)) = app.drag {
                // Always remove from any previous group
                for (_, item) in app.elements.iter_mut() {
                    if let Item::Group(grp) = item {
                        grp.children.retain(|c| *c != id);
                    }
                }
                // Re-add if dropped inside a group
                let center = app.elements.get(&id).map(|e| e.center());
                if let Some(center) = center {
                    for g_id in app.order.clone() {
                        if g_id == id {
                            continue;
                        }
                        if let Some(Item::Group(g)) = app.elements.get(&g_id) {
                            let g_bounds =
                                Rectangle::new(Point::new(g.x, g.y), Size::new(g.w, g.h));
                            if g_bounds.contains(center) {
                                if let Some(Item::Group(grp)) = app.elements.get_mut(&id) {
                                    grp.children.retain(|c| *c != g_id);
                                }
                                if let Some(Item::Group(grp)) = app.elements.get_mut(&g_id) {
                                    grp.children.push(id);
                                }
                                break;
                            }
                        }
                    }
                }
            }
            app.drag = None;
        }
        Message::EditNodeText(id, action) => {
            if let Some(content) = &mut app.edit_content {
                content.perform(action);
                let text = content.text();
                if let Some(Item::Node(n)) = app.elements.get_mut(&id) {
                    n.text = text;
                }
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
            app.selected_connection = None;
            app.pan_start = Some(pos);
        }
        Message::SelectConnection(idx) => {
            app.selected_connection = Some(idx);
            app.selected = None;
            app.edit_content = None;
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
        Message::ResizeStart(_id) => {
            app.resize = Some(_id);
        }
        Message::ResizeMove(new_w, new_h) => {
            if let Some(id) = app.resize {
                if let Some(elem) = app.elements.get_mut(&id) {
                    elem.set_size(new_w.max(50.0), new_h.max(30.0));
                }
            }
        }
        Message::ResizeEnd => {
            app.resize = None;
        }
    }
}

pub fn view(app: &Whiteboard) -> Element<'_, Message> {
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

            if let Item::Node(_) = elem {
                sidebar_children = sidebar_children.push(text("").size(5));
                sidebar_children = sidebar_children.push(text("Text:").size(12));
                if let Some(content) = &app.edit_content {
                    sidebar_children = sidebar_children.push(
                        text_editor::TextEditor::new(content)
                            .on_action(move |action| Message::EditNodeText(id, action))
                            .height(Length::Fixed(150.0)),
                    );
                }
            }
        }
    }

    sidebar_children = sidebar_children.push(column![].height(Length::Fill));

    let sidebar = container(sidebar_children)
        .width(SIDEBAR_W)
        .height(Length::Fill)
        .style(|_theme: &Theme| {
            container::Style::default()
                .background(Color::from_rgb(0.98, 0.98, 0.98))
                .border(border::color(Color::from_rgb(0.85, 0.85, 0.85)).width(1.0))
        });

    row![canvas, sidebar].into()
}
