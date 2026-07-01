use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumAxis {
    X,
    Y,
    W,
    H,
}

impl NumAxis {
    fn is_horizontal(self) -> bool {
        matches!(self, NumAxis::X | NumAxis::W)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NumExpr {
    Fixed(f32),
    ScrW,
    ScrH,
    ScrWPercent(f32),
    ScrHPercent(f32),
    ParentWPercent(f32),
    ParentHPercent(f32),
    Custom(String),
}

impl Default for NumExpr {
    fn default() -> Self {
        Self::Fixed(0.0)
    }
}

impl NumExpr {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Fixed(_) => "Число",
            Self::ScrW => "Screen Width",
            Self::ScrH => "Screen Height",
            Self::ScrWPercent(_) => "% от ширины экрана",
            Self::ScrHPercent(_) => "% от высоты экрана",
            Self::ParentWPercent(_) => "% от ширины родителя",
            Self::ParentHPercent(_) => "% от высоты родителя",
            Self::Custom(_) => "Выражение",
        }
    }

    pub fn label_in_parent(&self) -> &'static str {
        match self {
            Self::ScrWPercent(p) => Self::ParentWPercent(*p).label(),
            Self::ScrHPercent(p) => Self::ParentHPercent(*p).label(),
            other => other.label(),
        }
    }

    pub fn preview(&self, scr_w: f32, scr_h: f32) -> f32 {
        match self {
            Self::Fixed(v) => *v,
            Self::ScrW => scr_w,
            Self::ScrH => scr_h,
            Self::ScrWPercent(p) => scr_w * p / 100.0,
            Self::ScrHPercent(p) => scr_h * p / 100.0,
            Self::ParentWPercent(p) => scr_w * p / 100.0,
            Self::ParentHPercent(p) => scr_h * p / 100.0,
            Self::Custom(s) => parse_simple_expr(s, scr_w, scr_h).unwrap_or(0.0),
        }
    }

    /// Превью относительно размеров родительского элемента (для слоёв внутри панели).
    pub fn preview_in_parent(
        &self,
        parent_w: f32,
        parent_h: f32,
        scr_w: f32,
        scr_h: f32,
    ) -> f32 {
        match self {
            Self::ParentWPercent(p) | Self::ScrWPercent(p) => parent_w * p / 100.0,
            Self::ParentHPercent(p) | Self::ScrHPercent(p) => parent_h * p / 100.0,
            _ => self.preview(scr_w, scr_h),
        }
    }

    pub fn to_glua(&self) -> String {
        match self {
            Self::Fixed(v) => format_fixed(*v),
            Self::ScrW => "ScrW()".into(),
            Self::ScrH => "ScrH()".into(),
            Self::ScrWPercent(p) => format!("ScrW() * {:.4}", p / 100.0),
            Self::ScrHPercent(p) => format!("ScrH() * {:.4}", p / 100.0),
            Self::ParentWPercent(p) => format!("ScrW() * {:.4}", p / 100.0),
            Self::ParentHPercent(p) => format!("ScrH() * {:.4}", p / 100.0),
            Self::Custom(s) => s.clone(),
        }
    }

    /// GLua внутри `.Paint = function(self, w, h)` — проценты от `w`/`h` родителя.
    pub fn to_glua_in_parent(&self) -> String {
        match self {
            Self::Fixed(v) => format_fixed(*v),
            Self::ScrW => "ScrW()".into(),
            Self::ScrH => "ScrH()".into(),
            Self::ScrWPercent(p) | Self::ParentWPercent(p) => format!("w * {:.4}", p / 100.0),
            Self::ScrHPercent(p) | Self::ParentHPercent(p) => format!("h * {:.4}", p / 100.0),
            Self::Custom(s) => s.clone(),
        }
    }

    /// GLua для дочернего элемента: `parent:GetWide()` / `GetTall()`.
    pub fn to_glua_for_child(&self, parent_var: &str, axis: NumAxis) -> String {
        let wide = format!("{parent_var}:GetWide()");
        let tall = format!("{parent_var}:GetTall()");
        match (self, axis) {
            (Self::Fixed(v), _) => format_fixed(*v),
            (Self::ParentWPercent(p) | Self::ScrWPercent(p), NumAxis::X | NumAxis::W) => {
                format!("{wide} * {:.4}", p / 100.0)
            }
            (Self::ParentHPercent(p) | Self::ScrHPercent(p), NumAxis::Y | NumAxis::H) => {
                format!("{tall} * {:.4}", p / 100.0)
            }
            (Self::ScrW, NumAxis::X | NumAxis::W) => wide,
            (Self::ScrH, NumAxis::Y | NumAxis::H) => tall,
            (Self::ScrW, _) => "ScrW()".into(),
            (Self::ScrH, _) => "ScrH()".into(),
            (Self::Custom(s), _) => s.clone(),
            (Self::ParentWPercent(p) | Self::ScrWPercent(p), _) => {
                format!("{tall} * {:.4}", p / 100.0)
            }
            (Self::ParentHPercent(p) | Self::ScrHPercent(p), _) => {
                format!("{wide} * {:.4}", p / 100.0)
            }
        }
    }

    /// После drag/resize: сохранить тип (% / фикс), обновить значение.
    pub fn set_pixels_preserving_kind(
        &self,
        pixels: f32,
        parent_w: f32,
        parent_h: f32,
        scr_w: f32,
        scr_h: f32,
        axis: NumAxis,
        relative_to_parent: bool,
    ) -> NumExpr {
        match self {
            Self::Fixed(_) | Self::Custom(_) => NumExpr::Fixed(pixels),
            Self::ScrW if axis.is_horizontal() => NumExpr::Fixed(pixels),
            Self::ScrH if !axis.is_horizontal() => NumExpr::Fixed(pixels),
            Self::ParentWPercent(_) | Self::ScrWPercent(_) if axis.is_horizontal() => {
                let base = if relative_to_parent { parent_w } else { scr_w };
                percent_expr_horizontal(pixels, base, relative_to_parent)
            }
            Self::ParentHPercent(_) | Self::ScrHPercent(_) if !axis.is_horizontal() => {
                let base = if relative_to_parent { parent_h } else { scr_h };
                percent_expr_vertical(pixels, base, relative_to_parent)
            }
            _ => NumExpr::Fixed(pixels),
        }
    }
}

fn percent_expr_horizontal(pixels: f32, base: f32, parent: bool) -> NumExpr {
    if base <= f32::EPSILON {
        return NumExpr::Fixed(pixels);
    }
    let p = pixels / base * 100.0;
    if parent {
        NumExpr::ParentWPercent(p)
    } else {
        NumExpr::ScrWPercent(p)
    }
}

fn percent_expr_vertical(pixels: f32, base: f32, parent: bool) -> NumExpr {
    if base <= f32::EPSILON {
        return NumExpr::Fixed(pixels);
    }
    let p = pixels / base * 100.0;
    if parent {
        NumExpr::ParentHPercent(p)
    } else {
        NumExpr::ScrHPercent(p)
    }
}

fn format_fixed(v: f32) -> String {
    if v.fract().abs() < f32::EPSILON {
        format!("{}", v as i32)
    } else {
        format!("{v:.2}")
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StrExpr {
    Literal(String),
    PlayerName,
    SteamID,
    Health,
    Armor,
    Custom(String),
}

impl Default for StrExpr {
    fn default() -> Self {
        Self::Literal(String::new())
    }
}

impl StrExpr {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Literal(_) => "Текст",
            Self::PlayerName => "PlayerName",
            Self::SteamID => "SteamID",
            Self::Health => "Health",
            Self::Armor => "Armor",
            Self::Custom(_) => "Выражение",
        }
    }

    pub fn preview(&self) -> String {
        match self {
            Self::Literal(s) => s.clone(),
            Self::PlayerName => "PlayerName".into(),
            Self::SteamID => "STEAM_0:0:12345".into(),
            Self::Health => "100".into(),
            Self::Armor => "50".into(),
            Self::Custom(s) => s.clone(),
        }
    }

    pub fn to_glua(&self) -> String {
        match self {
            Self::Literal(s) => format_lua_string(s),
            Self::PlayerName => "LocalPlayer():Nick()".into(),
            Self::SteamID => "LocalPlayer():SteamID()".into(),
            Self::Health => "LocalPlayer():Health()".into(),
            Self::Armor => "LocalPlayer():Armor()".into(),
            Self::Custom(s) => s.clone(),
        }
    }
}

fn format_lua_string(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn parse_simple_expr(s: &str, scr_w: f32, scr_h: f32) -> Option<f32> {
    let t = s.trim();
    if t == "ScrW()" {
        return Some(scr_w);
    }
    if t == "ScrH()" {
        return Some(scr_h);
    }
    if let Ok(v) = t.parse::<f32>() {
        return Some(v);
    }
    if let Some(rest) = t.strip_prefix("ScrW() *") {
        return rest.trim().parse().ok().map(|m: f32| scr_w * m);
    }
    if let Some(rest) = t.strip_prefix("ScrH() *") {
        return rest.trim().parse().ok().map(|m: f32| scr_h * m);
    }
    None
}
