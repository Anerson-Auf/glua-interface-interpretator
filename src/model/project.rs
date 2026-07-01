use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::element::{ElementKind, UiElement};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub screen_w: u32,
    pub screen_h: u32,
    pub root_id: Uuid,
    pub elements: Vec<UiElement>,
    #[serde(default = "default_color_palette")]
    pub color_palette: Vec<[u8; 4]>,
}

fn default_color_palette() -> Vec<[u8; 4]> {
    vec![
        [40, 44, 52, 220],
        [30, 32, 38, 245],
        [55, 120, 200, 255],
        [75, 140, 220, 255],
        [255, 255, 255, 255],
        [200, 200, 200, 255],
        [100, 220, 120, 255],
        [220, 80, 80, 255],
    ]
}

impl Default for Project {
    fn default() -> Self {
        let mut root = UiElement::new(ElementKind::EditablePanel, "RootPanel");
        root.x = super::expr::NumExpr::Fixed(0.0);
        root.y = super::expr::NumExpr::Fixed(0.0);
        root.w = super::expr::NumExpr::ScrW;
        root.h = super::expr::NumExpr::ScrH;
        root.corner_radius = 0.0;
        root.paint_background = false;
        root.text = super::expr::StrExpr::Literal(String::new());
        let root_id = root.id;

        Self {
            name: "Новый интерфейс".into(),
            screen_w: 1920,
            screen_h: 1080,
            root_id,
            elements: vec![root],
            color_palette: default_color_palette(),
        }
    }
}

impl Project {
    pub fn element(&self, id: Uuid) -> Option<&UiElement> {
        self.elements.iter().find(|e| e.id == id)
    }

    pub fn element_mut(&mut self, id: Uuid) -> Option<&mut UiElement> {
        self.elements.iter_mut().find(|e| e.id == id)
    }

    pub fn children_of(&self, parent: Uuid) -> Vec<&UiElement> {
        self.elements
            .iter()
            .filter(|e| e.parent == Some(parent))
            .collect()
    }

    pub fn children_ids(&self, parent: Uuid) -> Vec<Uuid> {
        self.elements
            .iter()
            .filter(|e| e.parent == Some(parent))
            .map(|e| e.id)
            .collect()
    }

    /// Переместить элемент среди соседей (порядок отрисовки / z-order).
    pub fn reorder_sibling(&mut self, id: Uuid, direction: i32) -> bool {
        let parent = match self.element(id).and_then(|e| e.parent) {
            Some(p) => p,
            None => return false,
        };
        let siblings: Vec<Uuid> = self.children_ids(parent);
        let Some(pos) = siblings.iter().position(|&s| s == id) else {
            return false;
        };
        let new_pos = (pos as i32 + direction).clamp(0, siblings.len() as i32 - 1) as usize;
        if new_pos == pos {
            return false;
        }
        let target = siblings[new_pos];
        let idx_a = self.elements.iter().position(|e| e.id == id).unwrap();
        let idx_b = self.elements.iter().position(|e| e.id == target).unwrap();
        self.elements.swap(idx_a, idx_b);
        true
    }

    pub fn move_before_sibling(&mut self, id: Uuid, before_id: Uuid) -> bool {
        if id == before_id {
            return false;
        }
        let parent = match self.element(id).and_then(|e| e.parent) {
            Some(p) => p,
            None => return false,
        };
        let before_parent = self.element(before_id).and_then(|e| e.parent);
        if before_parent != Some(parent) {
            return false;
        }
        let idx = match self.elements.iter().position(|e| e.id == id) {
            Some(i) => i,
            None => return false,
        };
        let el = self.elements.remove(idx);
        let insert_at = self
            .elements
            .iter()
            .position(|e| e.id == before_id)
            .unwrap_or(self.elements.len());
        self.elements.insert(insert_at, el);
        true
    }

    pub fn add_close_button(&mut self, parent_id: Uuid) -> Option<Uuid> {
        let parent_w = self
            .element(parent_id)
            .map(|e| e.w.preview(1920.0, 1080.0))
            .unwrap_or(400.0);
        let n = self.elements.len();
        let el = UiElement::new_close_button(format!("CloseBtn_{n}"), parent_w);
        Some(self.add_element(el, parent_id))
    }

    pub fn add_element(&mut self, mut el: UiElement, parent: Uuid) -> Uuid {
        el.parent = Some(parent);
        let id = el.id;
        self.elements.push(el);
        id
    }

    pub fn remove_element(&mut self, id: Uuid) -> bool {
        if id == self.root_id {
            return false;
        }
        let to_remove = collect_descendants(self, id);
        let before = self.elements.len();
        self.elements.retain(|e| !to_remove.contains(&e.id));
        self.elements.len() < before
    }

    pub fn duplicate_element(&mut self, id: Uuid) -> Option<Uuid> {
        let el = self.element(id)?.clone();
        let parent = el.parent?;
        let mut copy = el;
        copy.id = Uuid::new_v4();
        copy.name = format!("{}_copy", copy.name);
        copy.x = super::expr::NumExpr::Fixed(copy.x.preview(1920.0, 1080.0) + 10.0);
        for layer in &mut copy.text_layers {
            layer.id = Uuid::new_v4();
        }
        for layer in &mut copy.image_layers {
            layer.id = Uuid::new_v4();
        }
        Some(self.add_element(copy, parent))
    }
}

fn collect_descendants(project: &Project, id: Uuid) -> Vec<Uuid> {
    let mut result = vec![id];
    let mut i = 0;
    while i < result.len() {
        let current = result[i];
        for child in project.children_of(current) {
            result.push(child.id);
        }
        i += 1;
    }
    result
}
