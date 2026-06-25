use std::collections::HashMap;

use iced::widget::text_editor;
use iced::{Color, Point, Rectangle, Size};
use serde::{Deserialize, Serialize};

pub type Id = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeData {
    pub id: Id,
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupData {
    pub id: Id,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub children: Vec<Id>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Item {
    Node(NodeData),
    Group(GroupData),
}

impl Item {
    #[allow(dead_code)]
    pub fn id(&self) -> Id {
        match self {
            Item::Node(n) => n.id,
            Item::Group(g) => g.id,
        }
    }

    pub fn x(&self) -> f32 {
        match self {
            Item::Node(n) => n.x,
            Item::Group(g) => g.x,
        }
    }

    pub fn y(&self) -> f32 {
        match self {
            Item::Node(n) => n.y,
            Item::Group(g) => g.y,
        }
    }

    pub fn w(&self) -> f32 {
        match self {
            Item::Node(n) => n.w,
            Item::Group(g) => g.w,
        }
    }

    pub fn h(&self) -> f32 {
        match self {
            Item::Node(n) => n.h,
            Item::Group(g) => g.h,
        }
    }

    pub fn bounds(&self) -> Rectangle {
        Rectangle::new(Point::new(self.x(), self.y()), Size::new(self.w(), self.h()))
    }

    pub fn center(&self) -> Point {
        Point::new(self.x() + self.w() / 2.0, self.y() + self.h() / 2.0)
    }

    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x()
            && point.x <= self.x() + self.w()
            && point.y >= self.y()
            && point.y <= self.y() + self.h()
    }

    pub fn set_size(&mut self, new_w: f32, new_h: f32) {
        match self {
            Item::Node(n) => {
                n.w = new_w;
                n.h = new_h;
            }
            Item::Group(g) => {
                g.w = new_w;
                g.h = new_h;
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub from: Id,
    pub to: Id,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhiteboardData {
    pub elements: HashMap<Id, Item>,
    pub order: Vec<Id>,
    pub connections: Vec<Connection>,
    pub next_id: Id,
}

#[derive(Debug, Clone)]
pub enum Message {
    NewNode,
    NewGroup,
    DeleteSelected,
    SelectAndDrag(Id, f32, f32),
    DragMove(f32, f32),
    DragEnd,
    EditNodeText(Id, text_editor::Action),
    ToggleConnectionMode,
    StartConnection(Id),
    EndConnection(Id),
    PanStart(Point),
    PanMove(Point),
    PanEnd,
    ResizeStart(Id),
    ResizeMove(f32, f32),
    ResizeEnd,
    SelectConnection(usize),
    Save,
    Load,
}

pub const NODE_W: f32 = 160.0;
pub const NODE_H: f32 = 120.0;
pub const GROUP_W: f32 = 300.0;
pub const GROUP_H: f32 = 200.0;
pub const SIDEBAR_W: f32 = 220.0;

pub const NODE_COLOR: Color = Color::WHITE;
pub const GROUP_COLOR: Color = Color::from_rgba(0.7, 0.85, 1.0, 0.3);
pub const GROUP_BORDER: Color = Color::from_rgb(0.3, 0.5, 0.8);
pub const SELECT_COLOR: Color = Color::from_rgb(0.2, 0.4, 0.9);
pub const CONNECTION_COLOR: Color = Color::from_rgb(0.3, 0.3, 0.3);
pub const TEXT_COLOR: Color = Color::from_rgb(0.1, 0.1, 0.1);
pub const CANVAS_BG: Color = Color::from_rgb(0.95, 0.95, 0.95);
pub const GRID_COLOR: Color = Color::from_rgba(0.8, 0.8, 0.8, 0.5);

pub fn point_to_segment_distance(p: Point, a: Point, b: Point) -> f32 {
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let len_sq = abx * abx + aby * aby;
    if len_sq < 0.0001 {
        return ((p.x - a.x).powi(2) + (p.y - a.y).powi(2)).sqrt();
    }
    let t = ((p.x - a.x) * abx + (p.y - a.y) * aby) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let cx = a.x + t * abx;
    let cy = a.y + t * aby;
    ((p.x - cx).powi(2) + (p.y - cy).powi(2)).sqrt()
}

pub fn edge_point(rect: Rectangle, target: Point) -> Point {
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
