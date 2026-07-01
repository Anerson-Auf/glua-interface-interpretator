use eframe::egui;
use uuid::Uuid;

use crate::model::Project;

use super::EditorUi;

pub fn show(ctx: &egui::Context, editor: &mut EditorUi, project: &mut Project) {
    let Some(id) = editor.scripts_target else {
        editor.show_scripts_window = false;
        return;
    };

    if editor.scripts_checkpointed != Some(id) {
        editor.history.checkpoint(project);
        editor.scripts_checkpointed = Some(id);
    }

    let Some(el) = project.element_mut(id) else {
        editor.show_scripts_window = false;
        editor.scripts_target = None;
        editor.scripts_checkpointed = None;
        return;
    };

    let title = format!("Скрипты - {}", el.name);
    let close_btn_enabled = el.close_button.enabled;
    let mut open = editor.show_scripts_window;

    egui::Window::new(title)
        .default_size([520.0, 420.0])
        .open(&mut open)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new("Lua-код вставляется в экспорт как есть (с отступом).")
                    .small()
                    .weak(),
            );
            ui.separator();

            ui.heading("DoClick");
            ui.label(
                egui::RichText::new("Для DButton и кликабельных элементов.")
                    .small()
                    .weak(),
            );
            ui.add(
                egui::TextEdit::multiline(&mut el.lua_do_click)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .desired_rows(6)
                    .hint_text("-- print(\"clicked\")"),
            );

            ui.separator();
            ui.heading("OnHover (Think + IsHovered)");
            ui.label(
                egui::RichText::new("Выполняется в Think, когда курсор над панелью.")
                    .small()
                    .weak(),
            );
            ui.add(
                egui::TextEdit::multiline(&mut el.lua_on_hover)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .desired_rows(6)
                    .hint_text("-- print(\"hover\")"),
            );

            if close_btn_enabled {
                ui.separator();
                ui.heading("CloseButton - DoClick");
                ui.add(
                    egui::TextEdit::multiline(&mut el.close_button.lua_do_click)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(4)
                        .hint_text("self:GetParent():Remove()"),
                );
            }
        });

    editor.show_scripts_window = open;
    if !open {
        editor.scripts_target = None;
        editor.scripts_checkpointed = None;
    }
}

pub fn open_for(editor: &mut EditorUi, id: Uuid) {
    editor.scripts_checkpointed = None;
    editor.scripts_target = Some(id);
    editor.show_scripts_window = true;
}
