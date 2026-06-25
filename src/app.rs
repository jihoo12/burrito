use std::collections::HashMap;
use std::fs;

use iced::widget::{
    button, column, container, row, text, text_editor, Canvas, Column,
};
use iced::widget::canvas::Cache;
use iced::{
    border, Color, Element, Length, Point, Rectangle, Size, Theme, Alignment,
};

use crate::types::*;

pub enum Phase {
    Lobby,
    Editor(Whiteboard),
}

pub struct App {
    pub phase: Phase,
}

impl App {
    pub fn new() -> Self {
        App {
            phase: Phase::Lobby,
        }
    }
}

pub struct Whiteboard {
    pub elements: HashMap<Id, Item>,
    pub order: Vec<Id>,
    pub connections: Vec<Connection>,
    pub next_id: Id,
    pub selected: Option<Id>,
    pub drag: Option<(Id, f32, f32)>,
    pub pan_x: f32,
    pub pan_y: f32,
    pub zoom: f32,
    pub pan_start: Option<Point>,
    pub connection_source: Option<Id>,
    pub connection_mode: bool,
    pub edit_content: Option<text_editor::Content>,
    pub resize: Option<Id>,
    pub selected_connection: Option<usize>,
    pub current_path: Option<String>,
    pub grid_cache: Cache,
}

impl Whiteboard {
    pub fn new() -> Self {
        Whiteboard {
            elements: HashMap::new(),
            order: Vec::new(),
            connections: Vec::new(),
            next_id: 0,
            selected: None,
            drag: None,
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
            pan_start: None,
            connection_source: None,
            connection_mode: false,
            edit_content: None,
            resize: None,
            selected_connection: None,
            current_path: None,
            grid_cache: Cache::new(),
        }
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
        let mut group_fallback = None;
        for id in self.order.iter().rev() {
            if let Some(e) = self.elements.get(id) {
                if e.contains(point) {
                    match e {
                        Item::Node(_) => return Some(*id),
                        Item::Group(_) => {
                            if group_fallback.is_none() {
                                group_fallback = Some(*id);
                            }
                        }
                    }
                }
            }
        }
        group_fallback
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

    pub fn to_data(&self) -> WhiteboardData {
        WhiteboardData {
            elements: self.elements.clone(),
            order: self.order.clone(),
            connections: self.connections.clone(),
            next_id: self.next_id,
        }
    }

    pub fn from_data(data: WhiteboardData) -> Self {
        Whiteboard {
            elements: data.elements,
            order: data.order,
            connections: data.connections,
            next_id: data.next_id,
            selected: None,
            drag: None,
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
            pan_start: None,
            connection_source: None,
            connection_mode: false,
            edit_content: None,
            resize: None,
            selected_connection: None,
            current_path: None,
            grid_cache: Cache::new(),
        }
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), String> {
        let data = self.to_data();
        let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
        fs::write(path, &json).map_err(|e| e.to_string())
    }

    pub fn load_from_file(&mut self, path: &str) -> Result<(), String> {
        let json = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let data: WhiteboardData = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        *self = Whiteboard::from_data(data);
        Ok(())
    }
}

pub fn update(app: &mut App, message: Message) {
    match message {
        Message::NewWhiteboard => {
            app.phase = Phase::Editor(Whiteboard::new());
        }
        Message::LoadFromDisk => {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Load Whiteboard")
                .add_filter("JSON", &["json"])
                .pick_file()
            {
                let mut wb = Whiteboard::new();
                let path_str = path.to_str().unwrap_or("").to_string();
                match wb.load_from_file(&path_str) {
                    Ok(_) => {
                        wb.current_path = Some(path_str);
                        app.phase = Phase::Editor(wb);
                    }
                    Err(e) => eprintln!("Load failed: {}", e),
                }
            }
        }
        Message::GoToLobby => {
            app.phase = Phase::Lobby;
        }
        _ => {
            if let Phase::Editor(ref mut wb) = app.phase {
                editor_update(wb, message);
            }
        }
    }
}

fn editor_update(wb: &mut Whiteboard, message: Message) {
    match message {
        Message::NewNode => {
            wb.connection_mode = false;
            wb.connection_source = None;
            let x = 100.0 - wb.pan_x;
            let y = 100.0 - wb.pan_y;
            let id = wb.add_node("New Node".into(), x, y);
            wb.selected = Some(id);
            wb.edit_content = Some(text_editor::Content::with_text("New Node"));
        }
        Message::NewGroup => {
            wb.connection_mode = false;
            wb.connection_source = None;
            let x = 80.0 - wb.pan_x;
            let y = 80.0 - wb.pan_y;
            let id = wb.add_group(x, y);
            wb.selected = Some(id);
        }
        Message::DeleteSelected => {
            wb.connection_mode = false;
            wb.connection_source = None;
            if let Some(idx) = wb.selected_connection {
                wb.connections.remove(idx);
                wb.selected_connection = None;
            } else if let Some(id) = wb.selected {
                wb.delete_element(id);
            }
        }
        Message::SelectAndDrag(id, ox, oy) => {
            wb.selected = Some(id);
            wb.selected_connection = None;
            wb.drag = Some((id, ox, oy));
            if let Some(Item::Node(n)) = wb.elements.get(&id) {
                wb.edit_content = Some(text_editor::Content::with_text(&n.text));
            } else {
                wb.edit_content = None;
            }
        }
        Message::DragMove(x, y) => {
            if let Some((id, _, _)) = wb.drag {
                if let Some(elem) = wb.elements.get(&id) {
                    let dx = x - elem.x();
                    let dy = y - elem.y();
                    wb.move_element(id, dx, dy);
                }
            }
        }
        Message::DragEnd => {
            if let Some((id, _, _)) = wb.drag {
                for (_, item) in wb.elements.iter_mut() {
                    if let Item::Group(grp) = item {
                        grp.children.retain(|c| *c != id);
                    }
                }
                let center = wb.elements.get(&id).map(|e| e.center());
                if let Some(center) = center {
                    for g_id in &wb.order {
                        if *g_id == id {
                            continue;
                        }
                        if let Some(Item::Group(g)) = wb.elements.get(g_id) {
                            let g_bounds =
                                Rectangle::new(Point::new(g.x, g.y), Size::new(g.w, g.h));
                            if g_bounds.contains(center) {
                                if let Some(Item::Group(grp)) = wb.elements.get_mut(&id) {
                                    grp.children.retain(|c| *c != *g_id);
                                }
                                if let Some(Item::Group(grp)) = wb.elements.get_mut(g_id) {
                                    grp.children.push(id);
                                }
                                break;
                            }
                        }
                    }
                }
            }
            wb.drag = None;
        }
        Message::EditNodeText(id, action) => {
            if let Some(content) = &mut wb.edit_content {
                content.perform(action);
                let text = content.text();
                if let Some(Item::Node(n)) = wb.elements.get_mut(&id) {
                    n.text = text;
                }
            }
        }
        Message::ToggleConnectionMode => {
            wb.connection_mode = !wb.connection_mode;
            wb.connection_source = None;
        }
        Message::StartConnection(id) => {
            if wb.connection_source == Some(id) {
                wb.connection_source = None;
            } else {
                wb.connection_source = Some(id);
            }
        }
        Message::EndConnection(id) => {
            if let Some(src) = wb.connection_source {
                if src != id {
                    wb.connections.push(Connection { from: src, to: id });
                    wb.connection_source = Some(id);
                }
            }
        }
        Message::PanStart(pos) => {
            wb.selected_connection = None;
            wb.pan_start = Some(pos);
        }
        Message::SelectConnection(idx) => {
            wb.selected_connection = Some(idx);
            wb.selected = None;
            wb.edit_content = None;
        }
        Message::PanMove(pos) => {
            if let Some(start) = wb.pan_start {
                let dx = pos.x - start.x;
                let dy = pos.y - start.y;
                wb.pan_x += dx;
                wb.pan_y += dy;
                wb.pan_start = Some(pos);
            }
        }
        Message::PanEnd => {
            wb.pan_start = None;
        }
        Message::ResizeStart(_id) => {
            wb.resize = Some(_id);
        }
        Message::ResizeMove(new_w, new_h) => {
            if let Some(id) = wb.resize {
                if let Some(elem) = wb.elements.get_mut(&id) {
                    elem.set_size(new_w.max(50.0), new_h.max(30.0));
                }
            }
        }
        Message::ResizeEnd => {
            wb.resize = None;
        }
        Message::ZoomIn => {
            wb.zoom = (wb.zoom * 1.2).min(5.0);
        }
        Message::ZoomOut => {
            wb.zoom = (wb.zoom / 1.2).max(0.1);
        }
        Message::ZoomReset => {
            wb.zoom = 1.0;
        }
        Message::Save => {
            if let Some(ref path) = wb.current_path.clone() {
                match wb.save_to_file(path) {
                    Ok(_) => {}
                    Err(e) => eprintln!("Save failed: {}", e),
                }
            } else if let Some(path) = rfd::FileDialog::new()
                .set_title("Save Whiteboard")
                .add_filter("JSON", &["json"])
                .set_file_name("whiteboard.json")
                .save_file()
            {
                let path_str = path.to_str().unwrap_or("").to_string();
                match wb.save_to_file(&path_str) {
                    Ok(_) => wb.current_path = Some(path_str),
                    Err(e) => eprintln!("Save failed: {}", e),
                }
            }
        }
        Message::Load => {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Load Whiteboard")
                .add_filter("JSON", &["json"])
                .pick_file()
            {
                let path_str = path.to_str().unwrap_or("").to_string();
                match wb.load_from_file(&path_str) {
                    Ok(_) => {
                        wb.current_path = Some(path_str);
                    }
                    Err(e) => eprintln!("Load failed: {}", e),
                }
            }
        }
        _ => {}
    }
}

pub fn view(app: &App) -> Element<'_, Message> {
    match &app.phase {
        Phase::Lobby => lobby_view(),
        Phase::Editor(wb) => editor_view(wb),
    }
}

fn lobby_view() -> Element<'static, Message> {
    container(
        column![
            text("Burrito Whiteboard").size(40),
            text("").size(30),
            button("+  Create New Whiteboard")
                .on_press(Message::NewWhiteboard)
                .padding(20),
            text("").size(15),
            button("📂  Load from Disk")
                .on_press(Message::LoadFromDisk)
                .padding(20),
        ]
        .spacing(5)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(Alignment::Center)
    .align_y(Alignment::Center)
    .into()
}

fn editor_view(wb: &Whiteboard) -> Element<'_, Message> {
    let toolbar = toolbar_view(wb);
    let canvas = Canvas::new(wb).width(Length::Fill).height(Length::Fill);
    let sidebar = sidebar_view(wb);
    let content = row![canvas, sidebar];
    column![toolbar, content].into()
}

fn toolbar_view(wb: &Whiteboard) -> Element<'_, Message> {
    row![
        button("+ Node").on_press(Message::NewNode),
        button("+ Group").on_press(Message::NewGroup),
        button(if wb.connection_mode { "➜ Exit" } else { "➜ Connect" })
            .on_press(Message::ToggleConnectionMode),
        button("Delete").on_press(Message::DeleteSelected),
        text(" | ").size(16),
        button("−").on_press(Message::ZoomOut),
        text(format!("{:.0}%", wb.zoom * 100.0)).size(14),
        button("+").on_press(Message::ZoomIn),
        button("⟲").on_press(Message::ZoomReset),
        text(" | ").size(16),
        button("💾 Save").on_press(Message::Save),
        button("📂 Load").on_press(Message::Load),
        text(" | ").size(16),
        button("🏠 Lobby").on_press(Message::GoToLobby),
    ]
    .spacing(4)
    .padding(6)
    .align_y(Alignment::Center)
    .into()
}

fn sidebar_view(wb: &Whiteboard) -> Element<'_, Message> {
    let mut children: Column<Message> = column![].spacing(5).padding(10);

    if let Some(id) = wb.selected {
        children = children.push(text("").size(5));
        children = children.push(text("Selected:").size(14));
        if let Some(elem) = wb.elements.get(&id) {
            let label = match elem {
                Item::Node(n) => format!("Node {}: {}", n.id, n.text),
                Item::Group(g) => {
                    format!("Group {} ({} children)", g.id, g.children.len())
                }
            };
            children = children.push(text(label).size(12));

            if let Item::Node(_) = elem {
                children = children.push(text("").size(5));
                children = children.push(text("Text:").size(12));
                if let Some(content) = &wb.edit_content {
                    children = children.push(
                        text_editor::TextEditor::new(content)
                            .on_action(move |action| Message::EditNodeText(id, action))
                            .height(Length::Fixed(150.0)),
                    );
                }
            }
        }
    }

    children = children.push(column![].height(Length::Fill));

    container(children)
        .width(SIDEBAR_W)
        .height(Length::Fill)
        .style(|_theme: &Theme| {
            container::Style::default()
                .background(Color::from_rgb(0.98, 0.98, 0.98))
                .border(border::color(Color::from_rgb(0.85, 0.85, 0.85)).width(1.0))
        })
        .into()
}
