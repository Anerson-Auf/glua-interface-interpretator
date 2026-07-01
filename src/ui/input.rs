use eframe::egui;
use uuid::Uuid;

use crate::model::{NumAxis, Project};

use super::history::snap_coord;
use super::EditorUi;

pub fn handle_shortcuts(ctx: &egui::Context, editor: &mut EditorUi, project: &mut Project) {
    let scr_w = project.screen_w as f32;
    let scr_h = project.screen_h as f32;

    ctx.input(|i| {
        if i.key_pressed(egui::Key::Z) && i.modifiers.ctrl && !i.modifiers.shift {
            if editor.history.undo(project) {
                editor.sync_selection_after_project_change(project);
            }
            return;
        }
        if (i.key_pressed(egui::Key::Y) && i.modifiers.ctrl)
            || (i.key_pressed(egui::Key::Z) && i.modifiers.ctrl && i.modifiers.shift)
        {
            if editor.history.redo(project) {
                editor.sync_selection_after_project_change(project);
            }
            return;
        }

        if i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace) {
            if i.modifiers.ctrl {
                return;
            }
            delete_selection(editor, project);
            return;
        }

        let step = if i.modifiers.shift { 10.0 } else { 1.0 };
        let dx = if i.key_pressed(egui::Key::ArrowLeft) {
            Some(-step)
        } else if i.key_pressed(egui::Key::ArrowRight) {
            Some(step)
        } else {
            None
        };
        let dy = if i.key_pressed(egui::Key::ArrowUp) {
            Some(-step)
        } else if i.key_pressed(egui::Key::ArrowDown) {
            Some(step)
        } else {
            None
        };

        if dx.is_some() || dy.is_some() {
            nudge_selection(editor, project, dx.unwrap_or(0.0), dy.unwrap_or(0.0), scr_w, scr_h);
        }
    });
}

fn delete_selection(editor: &mut EditorUi, project: &mut Project) {
    let ids: Vec<Uuid> = editor
        .selection
        .iter()
        .copied()
        .filter(|id| *id != project.root_id)
        .collect();
    if ids.is_empty() {
        return;
    }
    editor.history.checkpoint(project);
    for id in ids {
        project.remove_element(id);
    }
    editor.select_single(project.root_id);
}

fn nudge_selection(
    editor: &mut EditorUi,
    project: &mut Project,
    dx: f32,
    dy: f32,
    scr_w: f32,
    scr_h: f32,
) {
    if editor.selection.is_empty() {
        return;
    }
    editor.history.checkpoint(project);

    let ids: Vec<Uuid> = editor.selection.clone();
    for id in ids {
        let Some(el) = project.element(id) else { continue };
        if el.locked || el.dock != crate::model::DockMode::None {
            continue;
        }
        let (pw, ph, rel) = parent_dims(project, id, scr_w, scr_h);
        if let Some(el_mut) = project.element_mut(id) {
            let cx = el_mut.x.preview_in_parent(pw, ph, scr_w, scr_h);
            let cy = el_mut.y.preview_in_parent(pw, ph, scr_w, scr_h);
            let nx = snap_coord(cx + dx, editor.snap_to_grid);
            let ny = snap_coord(cy + dy, editor.snap_to_grid);
            el_mut.x = el_mut
                .x
                .set_pixels_preserving_kind(nx, pw, ph, scr_w, scr_h, NumAxis::X, rel);
            el_mut.y = el_mut
                .y
                .set_pixels_preserving_kind(ny, pw, ph, scr_w, scr_h, NumAxis::Y, rel);
        }
    }
}

fn parent_dims(project: &Project, id: Uuid, scr_w: f32, scr_h: f32) -> (f32, f32, bool) {
    let el = project.element(id).unwrap();
    if let Some(parent_id) = el.parent {
        let pw = project
            .element(parent_id)
            .map(|p| p.w.preview(scr_w, scr_h))
            .unwrap_or(scr_w);
        let ph = project
            .element(parent_id)
            .map(|p| p.h.preview(scr_w, scr_h))
            .unwrap_or(scr_h);
        (pw, ph, true)
    } else {
        (scr_w, scr_h, false)
    }
}
