use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 255 };
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 255 };

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LayerKind {
    Rect { corner_radius: f32 },
    Ellipse,
    /// Raw RGBA8 pixels, row-major, `width * height * 4` bytes.
    Image { width: u32, height: u32, rgba: Vec<u8> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub kind: LayerKind,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// Rotation in degrees around the layer centre.
    pub rotation: f32,
    pub opacity: f32,
    pub fill: Color,
    pub visible: bool,
}

impl Layer {
    pub fn rect(name: &str, x: f32, y: f32, w: f32, h: f32, fill: Color) -> Self {
        Self::new(name, LayerKind::Rect { corner_radius: 0.0 }, x, y, w, h, fill)
    }

    pub fn ellipse(name: &str, x: f32, y: f32, w: f32, h: f32, fill: Color) -> Self {
        Self::new(name, LayerKind::Ellipse, x, y, w, h, fill)
    }

    pub fn image(name: &str, x: f32, y: f32, width: u32, height: u32, rgba: Vec<u8>) -> Self {
        let kind = LayerKind::Image { width, height, rgba };
        Self::new(name, kind, x, y, width as f32, height as f32, Color::WHITE)
    }

    fn new(name: &str, kind: LayerKind, x: f32, y: f32, width: f32, height: f32, fill: Color) -> Self {
        Layer {
            name: name.to_string(),
            kind,
            x,
            y,
            width,
            height,
            rotation: 0.0,
            opacity: 1.0,
            fill,
            visible: true,
        }
    }

    pub fn contains(&self, px: f32, py: f32) -> bool {
        let (cx, cy) = (self.x + self.width / 2.0, self.y + self.height / 2.0);
        let a = (-self.rotation).to_radians();
        let (dx, dy) = (px - cx, py - cy);
        let (lx, ly) = (dx * a.cos() - dy * a.sin(), dx * a.sin() + dy * a.cos());
        lx.abs() <= self.width / 2.0 && ly.abs() <= self.height / 2.0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub width: u32,
    pub height: u32,
    pub background: Color,
    /// Bottom-to-top draw order.
    pub layers: Vec<Layer>,
}

impl Page {
    pub fn new(width: u32, height: u32) -> Self {
        Page { width, height, background: Color::WHITE, layers: Vec::new() }
    }

    /// Topmost visible layer under the point, if any.
    pub fn hit_test(&self, x: f32, y: f32) -> Option<usize> {
        self.layers.iter().rposition(|l| l.visible && l.contains(x, y))
    }

    pub fn move_layer(&mut self, from: usize, to: usize) {
        if from < self.layers.len() && to < self.layers.len() {
            let l = self.layers.remove(from);
            self.layers.insert(to, l);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub title: String,
    pub pages: Vec<Page>,
}

impl Document {
    pub fn new(title: &str, width: u32, height: u32) -> Self {
        Document { title: title.to_string(), pages: vec![Page::new(width, height)] }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}
