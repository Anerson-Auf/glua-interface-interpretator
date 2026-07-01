use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::expr::{NumExpr, StrExpr};
use super::layer::{GradientFill, ImageLayer, TextLayer};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementKind {
    Panel,
    Frame,
    Button,
    #[serde(alias = "Label")]
    EditablePanel,
    EditablePanelImaged,
    TextEntry,
    Image,
}

impl ElementKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Panel => "DPanel",
            Self::Frame => "DFrame",
            Self::Button => "DButton",
            Self::EditablePanel => "EditablePanel",
            Self::EditablePanelImaged => "EditablePanel (imaged)",
            Self::TextEntry => "DTextEntry",
            Self::Image => "DImage",
        }
    }

    pub fn vgui_class(&self) -> &'static str {
        match self {
            Self::EditablePanel | Self::EditablePanelImaged => "EditablePanel",
            other => other.label(),
        }
    }

    pub fn accepts_image_drop(self) -> bool {
        matches!(self, Self::Image | Self::EditablePanelImaged) || self.supports_layers()
    }

    pub fn supports_layers(self) -> bool {
        matches!(
            self,
            Self::Panel
                | Self::EditablePanel
                | Self::EditablePanelImaged
                | Self::Button
                | Self::Frame
        )
    }

    pub fn supports_text_layers(self) -> bool {
        self.supports_layers()
    }

    pub fn supports_gradient(self) -> bool {
        self.supports_layers()
    }

    pub fn supports_image_layers(self) -> bool {
        self.supports_layers()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BgImageMode {
    Stretch,
    Tile,
    Cover,
}

impl Default for BgImageMode {
    fn default() -> Self {
        Self::Stretch
    }
}

impl BgImageMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Stretch => "Растянуть",
            Self::Tile => "Замостить",
            Self::Cover => "Cover",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UiElement {
    pub id: Uuid,
    pub name: String,
    pub kind: ElementKind,
    pub parent: Option<Uuid>,
    pub x: NumExpr,
    pub y: NumExpr,
    pub w: NumExpr,
    pub h: NumExpr,
    pub visible: bool,
    pub corner_radius: f32,
    pub bg_color: [u8; 4],
    pub text_color: [u8; 4],
    pub text: StrExpr,
    pub font_name: String,
    pub font_size: u32,
    pub image_path: String,
    pub image_url: String,
    pub local_image_path: String,
    pub bg_image_material: String,
    pub bg_image_mode: BgImageMode,
    pub bg_image_alpha: u8,
    pub bg_image_tile_size: u32,
    pub dock: DockMode,
    pub paint_background: bool,
    pub clickable: bool,
    pub text_entry_multiline: bool,
    pub text_entry_draw_background: bool,
    pub text_entry_draw_border: bool,
    pub text_entry_numeric: bool,
    pub text_entry_editable: bool,
    #[serde(default)]
    pub bg_gradient: GradientFill,
    #[serde(default)]
    pub text_layers: Vec<TextLayer>,
    #[serde(default)]
    pub image_layers: Vec<ImageLayer>,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub make_popup: bool,
    #[serde(default)]
    pub close_on_escape: bool,
    #[serde(default)]
    pub close_button: CloseButtonConfig,
    #[serde(default)]
    pub button_hover_enabled: bool,
    #[serde(default = "default_button_hover_bg")]
    pub button_hover_bg: [u8; 4],
    #[serde(default)]
    pub button_pressed_enabled: bool,
    #[serde(default = "default_button_pressed_bg")]
    pub button_pressed_bg: [u8; 4],
    #[serde(default = "default_button_hover_text")]
    pub button_hover_text_color: [u8; 4],
    #[serde(default = "default_button_pressed_text")]
    pub button_pressed_text_color: [u8; 4],
    #[serde(default)]
    pub button_disabled: bool,
    #[serde(default)]
    pub lua_do_click: String,
    #[serde(default)]
    pub lua_on_hover: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloseButtonConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_close_size")]
    pub w: f32,
    #[serde(default = "default_close_size")]
    pub h: f32,
    #[serde(default)]
    pub x: NumExpr,
    #[serde(default)]
    pub y: NumExpr,
    #[serde(default = "default_close_text")]
    pub label: String,
    #[serde(default)]
    pub lua_do_click: String,
}

fn default_close_size() -> f32 {
    24.0
}

fn default_close_text() -> String {
    "x".into()
}

fn default_button_hover_bg() -> [u8; 4] {
    [75, 140, 220, 255]
}

fn default_button_pressed_bg() -> [u8; 4] {
    [40, 90, 160, 255]
}

fn default_button_hover_text() -> [u8; 4] {
    [255, 255, 255, 255]
}

fn default_button_pressed_text() -> [u8; 4] {
    [220, 220, 220, 255]
}

        impl Default for CloseButtonConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            w: default_close_size(),
            h: default_close_size(),
            x: NumExpr::ParentWPercent(92.0),
            y: NumExpr::Fixed(4.0),
            label: default_close_text(),
            lua_do_click: String::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockMode {
    None,
    Fill,
    Top,
    Bottom,
    Left,
    Right,
}

impl Default for DockMode {
    fn default() -> Self {
        Self::None
    }
}

impl UiElement {
    pub fn new(kind: ElementKind, name: impl Into<String>) -> Self {
        let (w, h) = match kind {
            ElementKind::Panel => (NumExpr::Fixed(200.0), NumExpr::Fixed(150.0)),
            ElementKind::Frame => (NumExpr::Fixed(400.0), NumExpr::Fixed(300.0)),
            ElementKind::Button => (NumExpr::Fixed(120.0), NumExpr::Fixed(32.0)),
            ElementKind::EditablePanel => (NumExpr::Fixed(150.0), NumExpr::Fixed(24.0)),
            ElementKind::EditablePanelImaged => (NumExpr::Fixed(200.0), NumExpr::Fixed(80.0)),
            ElementKind::TextEntry => (NumExpr::Fixed(200.0), NumExpr::Fixed(28.0)),
            ElementKind::Image => (NumExpr::Fixed(128.0), NumExpr::Fixed(128.0)),
        };

        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            kind,
            parent: None,
            x: NumExpr::Fixed(10.0),
            y: NumExpr::Fixed(10.0),
            w,
            h,
            visible: true,
            corner_radius: 0.0,
            bg_color: default_bg(kind),
            text_color: [255, 255, 255, 255],
            text: StrExpr::Literal(String::new()),
            font_name: String::new(),
            font_size: 16,
            image_path: String::new(),
            image_url: String::new(),
            local_image_path: String::new(),
            bg_image_material: String::new(),
            bg_image_mode: BgImageMode::Stretch,
            bg_image_alpha: 255,
            bg_image_tile_size: 64,
            dock: DockMode::None,
            paint_background: default_paint_background(kind),
            clickable: matches!(kind, ElementKind::Button | ElementKind::Frame),
            text_entry_multiline: false,
            text_entry_draw_background: true,
            text_entry_draw_border: true,
            text_entry_numeric: false,
            text_entry_editable: true,
            bg_gradient: GradientFill::default(),
            text_layers: Vec::new(),
            image_layers: Vec::new(),
            locked: false,
            notes: String::new(),
            make_popup: false,
            close_on_escape: false,
            close_button: CloseButtonConfig::default(),
            button_hover_enabled: false,
            button_hover_bg: default_button_hover_bg(),
            button_pressed_enabled: false,
            button_pressed_bg: default_button_pressed_bg(),
            button_hover_text_color: default_button_hover_text(),
            button_pressed_text_color: default_button_pressed_text(),
            button_disabled: false,
            lua_do_click: String::new(),
            lua_on_hover: String::new(),
        }
    }

    pub fn new_close_button(name: impl Into<String>, parent_w: f32) -> Self {
        let mut el = Self::new(ElementKind::Button, name);
        el.w = NumExpr::Fixed(24.0);
        el.h = NumExpr::Fixed(24.0);
        el.x = NumExpr::Fixed((parent_w - 28.0).max(0.0));
        el.y = NumExpr::Fixed(4.0);
        el.text = StrExpr::Literal(default_close_text());
        el.font_size = 18;
        el.lua_do_click = "self:GetParent():Remove()".into();
        el.button_hover_enabled = true;
        el.button_pressed_enabled = true;
        el
    }

    pub fn uses_custom_paint(&self) -> bool {
        let button_states = self.kind == ElementKind::Button
            && (self.button_hover_enabled || self.button_pressed_enabled);
        !self.text_layers.is_empty()
            || !self.image_layers.is_empty()
            || self.bg_gradient.enabled
            || button_states
            || (self.kind == ElementKind::EditablePanelImaged && !self.export_material_path().is_empty())
            || (self.paint_background
                && self.corner_radius > 0.0
                && self.bg_color[3] > 0
                && !self.bg_gradient.enabled)
    }

    pub fn export_material_path(&self) -> String {
        if self.kind == ElementKind::EditablePanelImaged {
            if !self.bg_image_material.is_empty() {
                return self.bg_image_material.clone();
            }
        }
        if !self.image_path.is_empty() {
            return self.image_path.clone();
        }
        if !self.image_url.is_empty() {
            return self.image_url.clone();
        }
        String::new()
    }

    pub fn preview_image_path(&self) -> &str {
        if !self.local_image_path.is_empty() {
            return &self.local_image_path;
        }
        ""
    }

    pub fn assign_local_image(&mut self, path: String) {
        let name = std::path::Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("image.png");
        let suggested = format!("materials/{name}");
        self.local_image_path = path;
        match self.kind {
            ElementKind::EditablePanelImaged if self.bg_image_material.is_empty() => {
                self.bg_image_material = suggested;
            }
            ElementKind::Image if self.image_path.is_empty() => {
                self.image_path = suggested;
            }
            _ => {}
        }
    }
}

fn default_paint_background(kind: ElementKind) -> bool {
    match kind {
        ElementKind::EditablePanel
        | ElementKind::EditablePanelImaged
        | ElementKind::TextEntry => false,
        _ => true,
    }
}

fn default_bg(kind: ElementKind) -> [u8; 4] {
    match kind {
        ElementKind::Panel => [40, 44, 52, 220],
        ElementKind::Frame => [30, 32, 38, 245],
        ElementKind::Button => [55, 120, 200, 255],
        ElementKind::EditablePanel => [0, 0, 0, 0],
        ElementKind::EditablePanelImaged => [0, 0, 0, 0],
        ElementKind::TextEntry => [30, 32, 38, 200],
        ElementKind::Image => [255, 255, 255, 255],
    }
}
