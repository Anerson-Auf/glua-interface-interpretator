use eframe::egui;
use uuid::Uuid;

use crate::model::{
    reorder_layer, BgImageMode, DockMode, ElementKind, GMOD_FONTS, GradientDirection, ImageLayer,
    NumExpr, Project, StrExpr, TextAlign, TextLayer,
};

use super::image_cache::{pick_image_file, suggest_material_path};
use super::scripts;
use super::theme::{self, card_frame, tool_button};
use super::{EditorUi, SubLayerRef};

pub fn show(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    editor: &mut EditorUi,
    project: &mut Project,
) {
    if let Some(color) = ctx.data_mut(|d| {
        d.get_temp::<[u8; 4]>(egui::Id::new("palette_apply"))
            .map(|c| {
                d.remove::<[u8; 4]>(egui::Id::new("palette_apply"));
                c
            })
    }) {
        if let Some(sel) = editor.primary() {
            editor.history.checkpoint(project);
            if let Some(el) = project.element_mut(sel) {
                el.bg_color = color;
            }
        }
    }

    let Some(sel) = editor.primary() else {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(theme::hint("Выберите элемент на холсте или в иерархии"));
        });
        return;
    };

    let is_root = sel == project.root_id;
    let kind = project.element(sel).map(|e| e.kind);
    let el_name = project.element(sel).map(|e| e.name.clone()).unwrap_or_default();
    let el_kind = kind.map(|k| k.label()).unwrap_or_default();

    card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(&el_name)
                        .strong()
                        .size(15.0),
                );
                ui.label(theme::hint(&el_kind));
            });
            if editor.selection.len() > 1 {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        theme::hint(&format!("+ ещё {}", editor.selection.len() - 1)),
                    );
                });
            }
        });
    });
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        if tool_button(ui, "Удалить", "Delete").clicked() && !is_root {
            editor.history.checkpoint(project);
            for id in editor.selection.clone() {
                if id != project.root_id {
                    project.remove_element(id);
                }
            }
            editor.select_single(project.root_id);
            editor.selected_sub_layer = None;
            return;
        }
        if tool_button(ui, "Копия", "Дублировать элемент").clicked() && !is_root {
            editor.history.checkpoint(project);
            if let Some(new_id) = project.duplicate_element(sel) {
                editor.select_single(new_id);
                editor.selected_sub_layer = None;
            }
        }
        if tool_button(ui, "Lua", "Скрипты DoClick / OnHover").clicked() {
            scripts::open_for(editor, sel);
        }
    });
    ui.add_space(8.0);

    let supports_layers = kind.map(|k| k.supports_layers()).unwrap_or(false);
    let supports_grad = kind.map(|k| k.supports_gradient()).unwrap_or(false);
    let mut add_close_child = false;
    let mut defer_checkpoint = false;

    {
        let Some(el) = project.element_mut(sel) else {
            return;
        };

        ui.label(theme::hint("Имя"));
        let name_resp = ui.add(
            egui::TextEdit::singleline(&mut el.name)
                .desired_width(f32::INFINITY)
                .margin(egui::Margin::symmetric(8.0, 5.0)),
        );
        if name_resp.gained_focus() {
            defer_checkpoint = true;
        }

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.checkbox(&mut el.visible, "Видимый");
            ui.checkbox(&mut el.locked, "Заблокирован");
        });

        ui.add_space(4.0);
        ui.label(theme::hint("Заметки"));
        let notes_resp = ui.add(
            egui::TextEdit::multiline(&mut el.notes)
                .desired_width(f32::INFINITY)
                .desired_rows(2)
                .hint_text("Комментарий дизайнера (не экспортируется)"),
        );
        if notes_resp.gained_focus() {
            defer_checkpoint = true;
        }

        egui::CollapsingHeader::new("Поведение и закрытие")
            .default_open(true)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Экспорт в GLua")
                        .small()
                        .weak(),
                );
                ui.checkbox(&mut el.make_popup, "MakePopup()");
                ui.checkbox(&mut el.close_on_escape, "Закрывать по KEY_ESCAPE");
                if el.close_on_escape {
                    ui.label(
                        egui::RichText::new(
                            "-> OnKeyCodeReleased: if key == KEY_ESCAPE then self:Remove() end",
                        )
                        .small()
                        .color(egui::Color32::from_rgb(120, 180, 255)),
                    );
                }
                ui.separator();
                ui.checkbox(&mut el.close_button.enabled, "CloseButton (DButton)");
                if el.close_button.enabled {
                    ui.label("Текст");
                    ui.text_edit_singleline(&mut el.close_button.label);
                    ui.add(egui::DragValue::new(&mut el.close_button.w).speed(1.0).prefix("W "));
                    ui.add(egui::DragValue::new(&mut el.close_button.h).speed(1.0).prefix("H "));
                    num_expr_ui(ui, "X", &mut el.close_button.x, NumExprScope::Parent);
                    num_expr_ui(ui, "Y", &mut el.close_button.y, NumExprScope::Parent);
                    if el.close_button.lua_do_click.is_empty() {
                        el.close_button.lua_do_click = "self:GetParent():Remove()".into();
                    }
                }
                if ui.button("+ Добавить CloseButton как элемент").clicked() {
                    add_close_child = true;
                }
            });

        egui::CollapsingHeader::new("Позиция и размер")
            .default_open(true)
            .show(ui, |ui| {
                let pos_scope = if is_root {
                    NumExprScope::Screen
                } else {
                    NumExprScope::Parent
                };
                num_expr_ui(ui, "X", &mut el.x, pos_scope);
                num_expr_ui(ui, "Y", &mut el.y, pos_scope);
                num_expr_ui(ui, "Ширина", &mut el.w, pos_scope);
                num_expr_ui(ui, "Высота", &mut el.h, pos_scope);
                ui.label("Dock");
                egui::ComboBox::from_id_salt(format!("dock_{sel}"))
                    .selected_text(dock_label(el.dock))
                    .show_ui(ui, |ui| {
                        for mode in [
                            DockMode::None,
                            DockMode::Fill,
                            DockMode::Top,
                            DockMode::Bottom,
                            DockMode::Left,
                            DockMode::Right,
                        ] {
                            ui.selectable_value(&mut el.dock, mode, dock_label(mode));
                        }
                    });
            });

        egui::CollapsingHeader::new("Внешний вид")
            .default_open(true)
            .show(ui, |ui| {
                if kind != Some(ElementKind::EditablePanelImaged) {
                    ui.checkbox(&mut el.paint_background, "Рисовать фон");
                }
                ui.add(egui::Slider::new(&mut el.corner_radius, 0.0..=64.0).text("Скругление"));
                if !el.bg_gradient.enabled {
                    ui.color_edit_button_srgba_unmultiplied(&mut el.bg_color);
                } else {
                    ui.label(
                        egui::RichText::new("Сплошной фон отключен - используется градиент")
                            .small()
                            .weak(),
                    );
                }

                if kind == Some(ElementKind::TextEntry) {
                    ui.separator();
                    ui.checkbox(&mut el.text_entry_draw_background, "Рисовать фон");
                    ui.checkbox(&mut el.text_entry_draw_border, "Рисовать рамку");
                    ui.checkbox(&mut el.text_entry_multiline, "Многострочный");
                    ui.checkbox(&mut el.text_entry_numeric, "Только числа");
                    ui.checkbox(&mut el.text_entry_editable, "Редактируемый");
                }

                if kind == Some(ElementKind::Image) {
                    ui.separator();
                    image_picker_ui(ui, ctx, editor, el, ImageTarget::DImage);
                }

                if kind == Some(ElementKind::EditablePanelImaged) {
                    ui.separator();
                    ui.label("Фоновое изображение (legacy)");
                    image_picker_ui(ui, ctx, editor, el, ImageTarget::BgImage);
                    ui.text_edit_singleline(&mut el.bg_image_material);
                    egui::ComboBox::from_id_salt(format!("bgmode_{sel}"))
                        .selected_text(el.bg_image_mode.label())
                        .show_ui(ui, |ui| {
                            for mode in
                                [BgImageMode::Stretch, BgImageMode::Tile, BgImageMode::Cover]
                            {
                                ui.selectable_value(&mut el.bg_image_mode, mode, mode.label());
                            }
                        });
                    ui.add(egui::Slider::new(&mut el.bg_image_alpha, 0..=255).text("Прозрачность"));
                }

                if kind == Some(ElementKind::Button) {
                    ui.checkbox(&mut el.clickable, "Кликабельный");
                    ui.checkbox(&mut el.button_disabled, "Отключена (SetEnabled false)");
                    ui.separator();
                    ui.checkbox(&mut el.button_hover_enabled, "Цвет фона при наведении");
                    if el.button_hover_enabled {
                        ui.label("Фон");
                        ui.color_edit_button_srgba_unmultiplied(&mut el.button_hover_bg);
                        ui.label("Текст");
                        ui.color_edit_button_srgba_unmultiplied(&mut el.button_hover_text_color);
                    }
                    ui.checkbox(&mut el.button_pressed_enabled, "Цвет фона при нажатии");
                    if el.button_pressed_enabled {
                        ui.label("Фон");
                        ui.color_edit_button_srgba_unmultiplied(&mut el.button_pressed_bg);
                        ui.label("Текст");
                        ui.color_edit_button_srgba_unmultiplied(&mut el.button_pressed_text_color);
                    }
                }
            });

        if supports_layers {
            let mut add_text = false;
            let mut add_image = false;

            egui::CollapsingHeader::new("Текстовые слои")
                .default_open(true)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Текст рисуется через draw.SimpleText в Paint")
                            .small()
                            .weak(),
                    );
                    if ui.button("+ Добавить текстовый слой").clicked() {
                        add_text = true;
                    }
                });

            egui::CollapsingHeader::new("Слои изображений")
                .default_open(true)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Нижний слой -> верхний. Перетаскивайте на холсте.")
                            .small()
                            .weak(),
                    );
                    if ui.button("+ Добавить слой картинки").clicked() {
                        add_image = true;
                    }
                });

            if add_text {
                let n = el.text_layers.len() + 1;
                el.text_layers.push(TextLayer::new(format!("Текст_{n}")));
            }
            if add_image {
                let n = el.image_layers.len() + 1;
                el.image_layers.push(ImageLayer::new(format!("Картинка_{n}")));
            }
        }
    }

    if defer_checkpoint {
        editor.history.checkpoint(project);
    }

    if add_close_child {
        editor.history.checkpoint(project);
        if let Some(id) = project.add_close_button(sel) {
            editor.select_single(id);
        }
    }

    if supports_layers {
        text_layers_editor(ui, project, sel, editor);
        image_layers_editor(ui, ctx, project, sel, editor);
    }

    if supports_grad {
        gradient_editor(ui, project, sel, editor);
    }

    if !is_root {
        ui.separator();
        ui.heading("Родитель");
        let mut parent = project.element(sel).and_then(|e| e.parent);
        parent_selector(ui, project, sel, &mut parent);
        if let Some(el) = project.element_mut(sel) {
            el.parent = parent;
        }
    }
}

enum ImageTarget {
    DImage,
    BgImage,
}

fn image_picker_ui(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    editor: &mut EditorUi,
    el: &mut crate::model::UiElement,
    target: ImageTarget,
) {
    ui.horizontal(|ui| {
        if ui.button("Выбрать файл").clicked() {
            if let Some(path) = pick_image_file() {
                let old = el.local_image_path.clone();
                el.assign_local_image(path);
                if !old.is_empty() {
                    editor.image_cache.invalidate(&old);
                }
                editor.image_cache.invalidate(&el.local_image_path);
            }
        }
        if ui.button("X").on_hover_text("Очистить").clicked() {
            let old = el.local_image_path.clone();
            el.local_image_path.clear();
            editor.image_cache.invalidate(&old);
        }
    });

    if !el.local_image_path.is_empty() {
        if let Some(tex) = editor.image_cache.texture_id(ctx, &el.local_image_path) {
            ui.image((tex, egui::vec2(120.0, 90.0)));
        }
    }

    match target {
        ImageTarget::DImage => {
            ui.label("Материал (GLua)");
            ui.text_edit_singleline(&mut el.image_path);
            ui.text_edit_singleline(&mut el.image_url);
        }
        ImageTarget::BgImage => {
            if ui.button("Путь материала из файла").clicked() {
                if !el.local_image_path.is_empty() {
                    el.bg_image_material = suggest_material_path(&el.local_image_path);
                }
            }
        }
    }
}

fn image_layer_picker(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    editor: &mut EditorUi,
    layer: &mut ImageLayer,
) {
    ui.horizontal(|ui| {
        if ui.button("Файл").clicked() {
            if let Some(path) = pick_image_file() {
                let old = layer.local_image_path.clone();
                layer.assign_local_image(path);
                if !old.is_empty() {
                    editor.image_cache.invalidate(&old);
                }
                editor.image_cache.invalidate(&layer.local_image_path);
            }
        }
        if ui.button("X").on_hover_text("Очистить").clicked() {
            let old = layer.local_image_path.clone();
            layer.local_image_path.clear();
            editor.image_cache.invalidate(&old);
        }
    });
    if !layer.local_image_path.is_empty() {
        ui.label(egui::RichText::new(&layer.local_image_path).small());
        if let Some(tex) = editor.image_cache.texture_id(ctx, &layer.local_image_path) {
            ui.image((tex, egui::vec2(80.0, 80.0)));
        }
    }
    ui.label("Материал (GLua)");
    ui.text_edit_singleline(&mut layer.material_path);
}

fn font_picker_ui(ui: &mut egui::Ui, font_name: &mut String, id: Uuid) {
    ui.horizontal(|ui| {
        ui.label("Шрифт");
        let preview = if font_name.is_empty() {
            "(по умолчанию)"
        } else {
            font_name.as_str()
        };
        egui::ComboBox::from_id_salt(format!("font_{id}"))
            .selected_text(preview)
            .show_ui(ui, |ui| {
                if ui.selectable_label(font_name.is_empty(), "(пусто / DermaDefault)").clicked() {
                    *font_name = String::new();
                }
                for &font in GMOD_FONTS {
                    ui.selectable_value(font_name, font.to_string(), font);
                }
            });
    });
    ui.text_edit_singleline(font_name);
}

fn gradient_editor(ui: &mut egui::Ui, project: &mut Project, sel: Uuid, _editor: &mut EditorUi) {
    let Some(el) = project.element_mut(sel) else {
        return;
    };
    let enabled = el.bg_gradient.enabled;

    egui::CollapsingHeader::new("Градиент фона")
        .default_open(enabled)
        .show(ui, |ui| {
            ui.checkbox(&mut el.bg_gradient.enabled, "Включить градиент");
            if !el.bg_gradient.enabled {
                return;
            }

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label("Начало");
                    ui.color_edit_button_srgba_unmultiplied(&mut el.bg_gradient.color_start);
                });
                ui.vertical(|ui| {
                    ui.label("Конец");
                    ui.color_edit_button_srgba_unmultiplied(&mut el.bg_gradient.color_end);
                });
                ui.vertical(|ui| {
                    ui.add_space(18.0);
                    if ui.button("<->").on_hover_text("Поменять цвета местами").clicked() {
                        std::mem::swap(
                            &mut el.bg_gradient.color_start,
                            &mut el.bg_gradient.color_end,
                        );
                    }
                });
            });

            ui.add(
                egui::Slider::new(&mut el.bg_gradient.steps, 4..=128)
                    .logarithmic(true)
                    .text("Плавность (шаги)"),
            );

            ui.horizontal(|ui| {
                ui.label("Направление:");
                egui::ComboBox::from_id_salt(format!("grad_dir_{sel}"))
                    .selected_text(el.bg_gradient.direction.label())
                    .show_ui(ui, |ui| {
                        for dir in
                            [GradientDirection::Vertical, GradientDirection::Horizontal]
                        {
                            ui.selectable_value(
                                &mut el.bg_gradient.direction,
                                dir,
                                dir.label(),
                            );
                        }
                    });
            });

            ui.separator();
            ui.label(egui::RichText::new("Превью").small().weak());
            let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 28.0), egui::Sense::hover());
            if rect.is_positive() {
                draw_gradient_preview(
                    &ui.painter_at(rect),
                    rect,
                    &el.bg_gradient,
                );
            }

            ui.label(
                egui::RichText::new("В GLua: self._UIGradStart / _UIGradEnd, шаги в _UIGradSteps")
                    .small()
                    .weak(),
            );
        });
}

fn draw_gradient_preview(
    painter: &egui::Painter,
    rect: egui::Rect,
    grad: &crate::model::GradientFill,
) {
    let steps = grad.steps.clamp(4, 128) as usize;
    for i in 0..steps {
        let t0 = i as f32 / steps as f32;
        let t1 = (i + 1) as f32 / steps as f32;
        let t = (t0 + t1) * 0.5;
        let r = grad.color_start[0] as f32
            + (grad.color_end[0] as f32 - grad.color_start[0] as f32) * t;
        let g = grad.color_start[1] as f32
            + (grad.color_end[1] as f32 - grad.color_start[1] as f32) * t;
        let b = grad.color_start[2] as f32
            + (grad.color_end[2] as f32 - grad.color_start[2] as f32) * t;
        let a = grad.color_start[3] as f32
            + (grad.color_end[3] as f32 - grad.color_start[3] as f32) * t;
        let color = egui::Color32::from_rgba_unmultiplied(r as u8, g as u8, b as u8, a as u8);
        let strip = match grad.direction {
            GradientDirection::Horizontal => egui::Rect::from_min_max(
                egui::pos2(rect.left() + rect.width() * t0, rect.top()),
                egui::pos2(rect.left() + rect.width() * t1, rect.bottom()),
            ),
            GradientDirection::Vertical => egui::Rect::from_min_max(
                egui::pos2(rect.left(), rect.top() + rect.height() * t0),
                egui::pos2(rect.right(), rect.top() + rect.height() * t1),
            ),
        };
        painter.rect_filled(strip, 2.0, color);
    }
    painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, egui::Color32::from_gray(80)));
}

fn dock_label(mode: DockMode) -> &'static str {
    match mode {
        DockMode::None => "Нет",
        DockMode::Fill => "Fill",
        DockMode::Top => "Top",
        DockMode::Bottom => "Bottom",
        DockMode::Left => "Left",
        DockMode::Right => "Right",
    }
}

fn num_expr_ui(ui: &mut egui::Ui, label: &str, expr: &mut NumExpr, scope: NumExprScope) {
    ui.horizontal(|ui| {
        ui.label(label);
        let selected = match scope {
            NumExprScope::Screen => expr.label(),
            NumExprScope::Parent => expr.label_in_parent(),
        };
        egui::ComboBox::from_id_salt(format!("{label}_{scope:?}_{:?}", std::ptr::from_ref(expr)))
            .selected_text(selected)
            .show_ui(ui, |ui| {
                ui.selectable_value(expr, NumExpr::Fixed(0.0), "Число");
                match scope {
                    NumExprScope::Screen => {
                        ui.selectable_value(expr, NumExpr::ScrW, "Screen Width");
                        ui.selectable_value(expr, NumExpr::ScrH, "Screen Height");
                        ui.selectable_value(expr, NumExpr::ScrWPercent(50.0), "% от ширины экрана");
                        ui.selectable_value(expr, NumExpr::ScrHPercent(50.0), "% от высоты экрана");
                    }
                    NumExprScope::Parent => {
                        ui.selectable_value(expr, NumExpr::ParentWPercent(50.0), "% от ширины родителя");
                        ui.selectable_value(expr, NumExpr::ParentHPercent(50.0), "% от высоты родителя");
                    }
                }
                ui.selectable_value(expr, NumExpr::Custom(String::new()), "Выражение");
            });
    });

    match expr {
        NumExpr::Fixed(v) => {
            ui.add(egui::DragValue::new(v).speed(1.0));
        }
        NumExpr::ScrWPercent(p) | NumExpr::ScrHPercent(p) | NumExpr::ParentWPercent(p)
        | NumExpr::ParentHPercent(p) => {
            ui.add(egui::Slider::new(p, 0.0..=100.0).suffix("%"));
        }
        NumExpr::Custom(s) => {
            ui.text_edit_singleline(s);
            if scope == NumExprScope::Parent {
                ui.label("Напр.: w * 0.5");
            } else {
                ui.label("Напр.: ScrW() * 0.5");
            }
        }
        _ => {}
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NumExprScope {
    Screen,
    Parent,
}

fn str_expr_ui(ui: &mut egui::Ui, expr: &mut StrExpr, id: Uuid) {
    egui::ComboBox::from_id_salt(format!("str_{id}"))
        .selected_text(expr.label())
        .show_ui(ui, |ui| {
            ui.selectable_value(expr, StrExpr::Literal(String::new()), "Текст");
            ui.selectable_value(expr, StrExpr::PlayerName, "PlayerName");
            ui.selectable_value(expr, StrExpr::SteamID, "SteamID");
            ui.selectable_value(expr, StrExpr::Health, "Health");
            ui.selectable_value(expr, StrExpr::Armor, "Armor");
            ui.selectable_value(expr, StrExpr::Custom(String::new()), "Выражение");
        });

    match expr {
        StrExpr::Literal(s) => {
            ui.text_edit_singleline(s);
        }
        StrExpr::Custom(s) => {
            ui.text_edit_singleline(s);
        }
        _ => {
            ui.label(format!("Превью: {}", expr.preview()));
        }
    }
}

fn align_picker(ui: &mut egui::Ui, align: &mut TextAlign, id: Uuid) {
    egui::ComboBox::from_id_salt(format!("align_{id}"))
        .selected_text(align.label())
        .show_ui(ui, |ui| {
            for a in [TextAlign::Left, TextAlign::Center, TextAlign::Right] {
                ui.selectable_value(align, a, a.label());
            }
        });
}

fn parent_selector(ui: &mut egui::Ui, project: &Project, sel: Uuid, parent: &mut Option<Uuid>) {
    let current = parent.unwrap_or(project.root_id);
    let current_name = project
        .element(current)
        .map(|e| e.name.as_str())
        .unwrap_or("?");

    egui::ComboBox::from_id_salt(format!("parent_{sel}"))
        .selected_text(current_name)
        .show_ui(ui, |ui| {
            for el in &project.elements {
                if el.id == sel {
                    continue;
                }
                let name = format!("{} ({})", el.name, el.kind.label());
                ui.selectable_value(parent, Some(el.id), name);
            }
        });
}

fn text_layers_editor(ui: &mut egui::Ui, project: &mut Project, sel: Uuid, editor: &mut EditorUi) {
    let Some(el) = project.element_mut(sel) else {
        return;
    };

    let mut remove_idx: Option<usize> = None;
    let mut reorder: Option<(usize, bool)> = None;

    for i in 0..el.text_layers.len() {
        let layer_id = el.text_layers[i].id;
        let header_name = el.text_layers[i].name.clone();
        let selected = editor.selected_sub_layer == Some(SubLayerRef::Text(layer_id));

        egui::CollapsingHeader::new(header_name)
            .id_salt(layer_id)
            .show(ui, |ui| {
            if selected {
                ui.colored_label(egui::Color32::LIGHT_BLUE, "Выбран на холсте");
            }
            let layer = &mut el.text_layers[i];
            ui.text_edit_singleline(&mut layer.name);
            num_expr_ui(ui, "X", &mut layer.x, NumExprScope::Parent);
            num_expr_ui(ui, "Y", &mut layer.y, NumExprScope::Parent);
            str_expr_ui(ui, &mut layer.text, layer_id);
            font_picker_ui(ui, &mut layer.font_name, layer_id);
            ui.color_edit_button_srgba_unmultiplied(&mut layer.text_color);
            ui.add(egui::Slider::new(&mut layer.font_size, 8..=48).text("Размер (превью)"));
            ui.label("Выравнивание");
            align_picker(ui, &mut layer.align, layer_id);
            ui.horizontal(|ui| {
                if ui.button("^").on_hover_text("Выше").clicked() {
                    reorder = Some((i, true));
                }
                if ui.button("v").on_hover_text("Ниже").clicked() {
                    reorder = Some((i, false));
                }
                if ui.button("Удалить").clicked() {
                    remove_idx = Some(i);
                }
            });
        });
    }

    if let Some((idx, up)) = reorder {
        reorder_layer(&mut el.text_layers, idx, up);
    }
    if let Some(idx) = remove_idx {
        el.text_layers.remove(idx);
        editor.selected_sub_layer = None;
    }
}

fn image_layers_editor(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    project: &mut Project,
    sel: Uuid,
    editor: &mut EditorUi,
) {
    let Some(el) = project.element_mut(sel) else {
        return;
    };

    let mut remove_idx: Option<usize> = None;
    let mut reorder: Option<(usize, bool)> = None;

    for i in 0..el.image_layers.len() {
        let layer_id = el.image_layers[i].id;
        let header_name = el.image_layers[i].name.clone();
        let selected = editor.selected_sub_layer == Some(SubLayerRef::Image(layer_id));

        egui::CollapsingHeader::new(format!("{header_name} (#{})", i + 1))
            .id_salt(layer_id)
            .show(ui, |ui| {
            if selected {
                ui.colored_label(egui::Color32::LIGHT_BLUE, "Выбран на холсте");
            }
            let layer = &mut el.image_layers[i];
            ui.text_edit_singleline(&mut layer.name);
            num_expr_ui(ui, "X", &mut layer.x, NumExprScope::Parent);
            num_expr_ui(ui, "Y", &mut layer.y, NumExprScope::Parent);
            num_expr_ui(ui, "Ширина", &mut layer.w, NumExprScope::Parent);
            num_expr_ui(ui, "Высота", &mut layer.h, NumExprScope::Parent);
            ui.add(egui::Slider::new(&mut layer.alpha, 0..=255).text("Прозрачность"));
            image_layer_picker(ui, ctx, editor, layer);
            ui.horizontal(|ui| {
                if ui.button("^ Выше").clicked() {
                    reorder = Some((i, true));
                }
                if ui.button("v Ниже").clicked() {
                    reorder = Some((i, false));
                }
                if ui.button("Удалить").clicked() {
                    remove_idx = Some(i);
                }
            });
        });
    }

    if let Some((idx, up)) = reorder {
        reorder_layer(&mut el.image_layers, idx, up);
    }
    if let Some(idx) = remove_idx {
        el.image_layers.remove(idx);
        editor.selected_sub_layer = None;
    }
}
