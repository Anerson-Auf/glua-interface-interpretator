use eframe::egui;

use crate::codegen::generate_glua;
use crate::model::Project;

use super::theme::{self, primary_button, tool_button, vertical_separator};
use super::EditorUi;

pub fn show(ctx: &egui::Context, editor: &mut EditorUi, project: &mut Project) {
    egui::TopBottomPanel::top("toolbar")
            .frame(
            egui::Frame::none()
                .fill(crate::ui::theme::SURFACE_RAISED)
                .stroke(egui::Stroke::new(1.0, crate::ui::theme::BORDER))
                .inner_margin(egui::Margin::symmetric(14.0, 10.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(theme::app_title());
                vertical_separator(ui);

                ui.label(theme::hint("Проект"));
                ui.add(
                    egui::TextEdit::singleline(&mut project.name)
                        .desired_width(160.0)
                        .margin(egui::Margin::symmetric(8.0, 4.0)),
                );

                vertical_separator(ui);

                ui.label(theme::hint("Экран"));
                ui.add(egui::DragValue::new(&mut project.screen_w).speed(1));
                ui.label(theme::hint("x"));
                ui.add(egui::DragValue::new(&mut project.screen_h).speed(1));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if tool_button(ui, "Новый", "Новый проект").clicked() {
                        *project = Project::default();
                        editor.history.clear();
                        editor.select_single(project.root_id);
                        editor.selected_sub_layer = None;
                    }

                    vertical_separator(ui);

                    if ui
                        .add_enabled(editor.history.can_redo(), egui::Button::new("Redo"))
                        .on_hover_text("Ctrl+Y")
                        .clicked()
                    {
                        editor.history.redo(project);
                        editor.sync_selection_after_project_change(project);
                    }
                    if ui
                        .add_enabled(editor.history.can_undo(), egui::Button::new("Undo"))
                        .on_hover_text("Ctrl+Z")
                        .clicked()
                    {
                        editor.history.undo(project);
                        editor.sync_selection_after_project_change(project);
                    }

                    vertical_separator(ui);

                    ui.menu_button("Файл", |ui| {
                        if ui.button("Сохранить...").clicked() {
                            save_project_dialog(project, editor, ctx);
                            ui.close_menu();
                        }
                        if ui.button("Открыть...").clicked() {
                            load_project_dialog(project, editor, ctx);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("JSON в буфер").clicked() {
                            save_project_to_clipboard(project, editor, ctx);
                            ui.close_menu();
                        }
                        if ui.button("JSON из буфера").clicked() {
                            load_project_from_clipboard(project, editor, ctx);
                            ui.close_menu();
                        }
                    });

                    vertical_separator(ui);

                    if ui.button("Код").on_hover_text("Показать сгенерированный Lua").clicked()
                    {
                        editor.code_preview = generate_glua(project);
                        editor.show_code_window = true;
                        editor.set_status(
                            ctx,
                            format!("Сгенерировано {} символов", editor.code_preview.len()),
                        );
                    }

                    if ui.add(primary_button("Экспорт GLua")).clicked() {
                        let code = generate_glua(project);
                        match arboard::Clipboard::new().and_then(|mut c| c.set_text(code)) {
                            Ok(()) => {
                                editor.set_status(ctx, "GLua код скопирован в буфер обмена")
                            }
                            Err(e) => editor.set_status(ctx, format!("Ошибка буфера: {e}")),
                        }
                    }
                });
            });
        });
}

fn save_project_to_clipboard(project: &Project, editor: &mut EditorUi, ctx: &egui::Context) {
    match serde_json::to_string_pretty(project) {
        Ok(json) => match arboard::Clipboard::new().and_then(|mut c| c.set_text(json)) {
            Ok(()) => editor.set_status(ctx, "Проект (JSON) скопирован в буфер"),
            Err(e) => editor.set_status(ctx, format!("Ошибка буфера: {e}")),
        },
        Err(e) => editor.set_status(ctx, format!("Ошибка сериализации: {e}")),
    }
}

fn save_project_dialog(project: &Project, editor: &mut EditorUi, ctx: &egui::Context) {
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(format!("{}.json", sanitize_filename(&project.name)))
        .add_filter("JSON проект", &["json"])
        .save_file()
    else {
        return;
    };

    match serde_json::to_string_pretty(project) {
        Ok(json) => match std::fs::write(&path, json) {
            Ok(()) => editor.set_status(ctx, format!("Сохранено: {}", path.display())),
            Err(e) => editor.set_status(ctx, format!("Ошибка сохранения: {e}")),
        },
        Err(e) => editor.set_status(ctx, format!("Ошибка сериализации: {e}")),
    }
}

fn load_project_dialog(project: &mut Project, editor: &mut EditorUi, ctx: &egui::Context) {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("JSON проект", &["json"])
        .pick_file()
    else {
        return;
    };

    match std::fs::read_to_string(&path) {
        Ok(s) => match serde_json::from_str::<Project>(&s) {
            Ok(loaded) => {
                *project = loaded;
                editor.history.clear();
                editor.select_single(project.root_id);
                editor.image_cache = super::image_cache::ImageCache::default();
                editor.selected_sub_layer = None;
                editor.set_status(ctx, format!("Загружено: {}", path.display()));
            }
            Err(e) => editor.set_status(ctx, format!("Некорректный JSON: {e}")),
        },
        Err(e) => editor.set_status(ctx, format!("Ошибка чтения файла: {e}")),
    }
}

fn load_project_from_clipboard(project: &mut Project, editor: &mut EditorUi, ctx: &egui::Context) {
    let text = match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
        Ok(t) => t,
        Err(e) => {
            editor.set_status(ctx, format!("Не удалось прочитать буфер: {e}"));
            return;
        }
    };

    match serde_json::from_str::<Project>(&text) {
        Ok(loaded) => {
            *project = loaded;
            editor.history.clear();
            editor.select_single(project.root_id);
            editor.image_cache = super::image_cache::ImageCache::default();
            editor.selected_sub_layer = None;
            editor.set_status(ctx, "Проект загружен из буфера обмена");
        }
        Err(e) => editor.set_status(ctx, format!("Некорректный JSON: {e}")),
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}