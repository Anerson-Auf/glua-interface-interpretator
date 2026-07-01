use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::expr::{NumExpr, StrExpr};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

impl Default for TextAlign {
    fn default() -> Self {
        Self::Left
    }
}

impl TextAlign {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Left => "Слева",
            Self::Center => "По центру",
            Self::Right => "Справа",
        }
    }

    pub fn glua_const(&self) -> &'static str {
        match self {
            Self::Left => "TEXT_ALIGN_LEFT",
            Self::Center => "TEXT_ALIGN_CENTER",
            Self::Right => "TEXT_ALIGN_RIGHT",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GradientDirection {
    Horizontal,
    Vertical,
}

impl Default for GradientDirection {
    fn default() -> Self {
        Self::Vertical
    }
}

impl GradientDirection {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Horizontal => "Горизонтальный",
            Self::Vertical => "Вертикальный",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GradientFill {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_grad_start")]
    pub color_start: [u8; 4],
    #[serde(default = "default_grad_end")]
    pub color_end: [u8; 4],
    #[serde(default)]
    pub direction: GradientDirection,
    /// Количество полос градиента (качество / плавность).
    #[serde(default = "default_grad_steps")]
    pub steps: u32,
}

fn default_grad_steps() -> u32 {
    32
}

fn default_grad_start() -> [u8; 4] {
    [40, 44, 52, 220]
}

fn default_grad_end() -> [u8; 4] {
    [20, 22, 28, 220]
}

impl Default for GradientFill {
    fn default() -> Self {
        Self {
            enabled: false,
            color_start: default_grad_start(),
            color_end: default_grad_end(),
            direction: GradientDirection::Vertical,
            steps: default_grad_steps(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextLayer {
    pub id: Uuid,
    pub name: String,
    pub x: NumExpr,
    pub y: NumExpr,
    pub text: StrExpr,
    pub font_name: String,
    pub font_size: u32,
    pub text_color: [u8; 4],
    pub align: TextAlign,
}

impl TextLayer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            x: NumExpr::Fixed(8.0),
            y: NumExpr::Fixed(8.0),
            text: StrExpr::Literal(String::new()),
            font_name: String::new(),
            font_size: 16,
            text_color: [255, 255, 255, 255],
            align: TextAlign::Left,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageLayer {
    pub id: Uuid,
    pub name: String,
    pub x: NumExpr,
    pub y: NumExpr,
    pub w: NumExpr,
    pub h: NumExpr,
    pub local_image_path: String,
    pub material_path: String,
    pub alpha: u8,
}

impl ImageLayer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            x: NumExpr::Fixed(0.0),
            y: NumExpr::Fixed(0.0),
            w: NumExpr::Fixed(64.0),
            h: NumExpr::Fixed(64.0),
            local_image_path: String::new(),
            material_path: String::new(),
            alpha: 255,
        }
    }

    pub fn assign_local_image(&mut self, path: String) {
        let name = std::path::Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("image.png");
        if self.material_path.is_empty() {
            self.material_path = format!("materials/{name}");
        }
        self.local_image_path = path;
    }

    pub fn export_material(&self) -> String {
        if !self.material_path.is_empty() {
            return self.material_path.clone();
        }
        if !self.local_image_path.is_empty() {
            let name = std::path::Path::new(&self.local_image_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("image.png");
            return format!("materials/{name}");
        }
        String::new()
    }
}

pub fn reorder_layer<T>(layers: &mut [T], idx: usize, up: bool) {
    if up {
        if idx > 0 {
            layers.swap(idx, idx - 1);
        }
    } else if idx + 1 < layers.len() {
        layers.swap(idx, idx + 1);
    }
}
