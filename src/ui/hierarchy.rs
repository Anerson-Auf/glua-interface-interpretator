use eframe::egui;
use uuid::Uuid;

use crate::model::Project;

use super::theme::{self, card_frame};
use super::EditorUi;

pub fn show(ui: &mut egui::Ui, editor: &mut EditorUi, project: &mut Project) {
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut editor.hierarchy_search)
                .hint_text("Поиск в иерархии...")
                .desired_width(f32::INFINITY)
                .margin(egui::Margin::symmetric(8.0, 5.0)),
        );
        if ui
            .small_button("X")
            .on_hover_text("Очистить")
            .clicked()
        {
            editor.hierarchy_search.clear();
        }
    });
    ui.add_space(6.0);

    let filter = editor.hierarchy_search.to_lowercase();
    let mut drag_id = editor.hierarchy_drag_id;
    let mut drop_target: Option<(Uuid, bool)> = None;
    let pointer_released = ui.input(|i| i.pointer.any_released());

    card_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        show_node(
            ui,
            editor,
            project,
            project.root_id,
            0,
            &filter,
            &mut drag_id,
            &mut drop_target,
        );
    });

    editor.hierarchy_drag_id = drag_id;

    if pointer_released {
        if let (Some(dragged), Some((target, _before))) = (editor.hierarchy_drag_id, drop_target) {
            if dragged != target {
                editor.history.checkpoint(project);
                project.move_before_sibling(dragged, target);
            }
            editor.hierarchy_drag_id = None;
        } else if editor.hierarchy_drag_id.is_some() && drop_target.is_none() {
            editor.hierarchy_drag_id = None;
        }
    }
}

fn show_node(
    ui: &mut egui::Ui,
    editor: &mut EditorUi,
    project: &Project,
    id: Uuid,
    depth: usize,
    filter: &str,
    drag_id: &mut Option<Uuid>,
    drop_target: &mut Option<(Uuid, bool)>,
) {
    let Some(el) = project.element(id) else {
        return;
    };

    let label_lower = format!("{} {}", el.name, el.kind.label()).to_lowercase();
    let matches = filter.is_empty() || label_lower.contains(filter);
    if !matches {
        let children: Vec<Uuid> = project.children_ids(id);
        for child in children {
            show_node(ui, editor, project, child, depth, filter, drag_id, drop_target);
        }
        return;
    }

    let selected = editor.is_selected(id);
    let indent = depth as f32 * 14.0;
    let is_root = id == project.root_id;

    let row_response = ui.horizontal(|ui| {
        ui.add_space(indent);

        if !is_root {
            ui.spacing_mut().item_spacing.x = 2.0;
            let up = ui.small_button("^").on_hover_text("Выше").clicked();
            let down = ui.small_button("v").on_hover_text("Ниже").clicked();
            ui.spacing_mut().item_spacing.x = 8.0;
            if up || down {
                ui.ctx().data_mut(|d| {
                    d.insert_temp(egui::Id::new("hier_reorder"), (id, if up { -1 } else { 1 }));
                });
            }
        }

        let kind_short = el.kind.label().trim_start_matches('D');
        let name = if el.locked {
            format!("[L] {}", el.name)
        } else {
            el.name.clone()
        };
        let label = format!("{name}  {kind_short}");
        let resp = ui.selectable_label(selected, label);

        if !is_root {
            let drag_resp = ui
                .small_button("::")
                .on_hover_text("Перетащите для смены порядка");
            if drag_resp.drag_started() {
                *drag_id = Some(id);
            }
            if let Some(dragged) = *drag_id {
                if dragged != id {
                    let drop_rect = resp.rect;
                    if let Some(pointer) = ui.input(|i| i.pointer.interact_pos()) {
                        if drop_rect.contains(pointer) {
                            *drop_target = Some((id, true));
                            ui.painter().rect_stroke(
                                drop_rect,
                                4.0,
                                egui::Stroke::new(1.5, theme::ACCENT),
                            );
                        }
                    }
                }
            }
        }

        resp
    });

    if row_response.inner.clicked() {
        let ctrl = ui.input(|i| i.modifiers.ctrl);
        let shift = ui.input(|i| i.modifiers.shift);
        if ctrl {
            editor.toggle_selection(id);
        } else if shift {
            if let Some(last) = editor.selection.last().copied() {
                editor.select_range(project, last, id);
            } else {
                editor.select_single(id);
            }
        } else {
            editor.select_single(id);
        }
        editor.selected_sub_layer = None;
    }

    let children: Vec<Uuid> = project.children_ids(id);
    for child in children {
        show_node(
            ui,
            editor,
            project,
            child,
            depth + 1,
            filter,
            drag_id,
            drop_target,
        );
    }
}

pub fn apply_pending_reorder(ui: &egui::Ui, editor: &mut EditorUi, project: &mut Project) {
    let pending: Option<(Uuid, i32)> = ui.ctx().data_mut(|d| {
        d.get_temp::<(Uuid, i32)>(egui::Id::new("hier_reorder"))
            .and_then(|p| {
                d.remove::<(Uuid, i32)>(egui::Id::new("hier_reorder"));
                Some(p)
            })
    });
    if let Some((id, dir)) = pending {
        editor.history.checkpoint(project);
        project.reorder_sibling(id, dir);
    }
}
