use crate::model::Project;

const MAX_HISTORY: usize = 50;

pub struct History {
    undo: Vec<Project>,
    redo: Vec<Project>,
}

impl Default for History {
    fn default() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }
}

impl History {
    pub fn checkpoint(&mut self, project: &Project) {
        self.undo.push(project.clone());
        if self.undo.len() > MAX_HISTORY {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self, project: &mut Project) -> bool {
        let Some(prev) = self.undo.pop() else {
            return false;
        };
        self.redo.push(project.clone());
        *project = prev;
        true
    }

    pub fn redo(&mut self, project: &mut Project) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(project.clone());
        *project = next;
        true
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

pub const SNAP_GRID: f32 = 20.0;

/// Позиция от точки старта drag + накопительная дельта (не от текущей каждый кадр).
pub fn apply_drag_snap(origin: f32, total_delta: f32, snap: bool) -> f32 {
    snap_coord(origin + total_delta, snap)
}

pub fn snap_coord(v: f32, enabled: bool) -> f32 {
    if !enabled {
        return v;
    }
    (v / SNAP_GRID).round() * SNAP_GRID
}

pub fn snap_size(v: f32, min: f32, snap: bool) -> f32 {
    let v = if snap { snap_coord(v, true) } else { v };
    v.max(min)
}
