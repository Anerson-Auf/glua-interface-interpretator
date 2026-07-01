use eframe::egui;
use uuid::Uuid;

use crate::model::{DockMode, ElementKind, GradientDirection, NumAxis, Project, TextAlign};

use super::history::{apply_drag_snap, snap_size};
use super::image_cache::is_image_file;
use super::theme::{self, status_bar_frame};
use super::{EditorUi, SubLayerRef};

#[derive(Clone, Copy)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

struct HitEntry {
    id: Uuid,
    rect: egui::Rect,
    accepts_image: bool,
}

impl HitEntry {
    fn from_element(id: Uuid, rect: egui::Rect, kind: ElementKind) -> Self {
        Self {
            id,
            rect,
            accepts_image: kind.accepts_image_drop(),
        }
    }
}

pub fn show(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    editor: &mut EditorUi,
    project: &mut Project,
) {
    let scr_w = project.screen_w as f32;
    let scr_h = project.screen_h as f32;

    status_bar_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{} x {}", project.screen_w, project.screen_h))
                    .strong()
                    .color(theme::ACCENT),
            );
            ui.label(theme::hint("|"));
            let sel_label = if editor.selection.len() > 1 {
                format!("{} элементов", editor.selection.len())
            } else {
                editor
                    .primary()
                    .and_then(|id| project.element(id).map(|e| e.name.clone()))
                    .unwrap_or_else(|| "ничего".into())
            };
            ui.label(format!("Выбран: {sel_label}"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(theme::hint(
                    "Колесо - зум  |  СКМ/ПКМ - камера  |  Перетащите файл на холст",
                ));
            });
        });
    });
    ui.add_space(4.0);

    let workspace = ui.available_rect_before_wrap();
    ui.set_clip_rect(workspace);
    let resp = ui.allocate_rect(workspace, egui::Sense::click_and_drag());
    let painter = ui.painter_at(resp.rect);

    let workspace_bg = egui::Color32::from_rgb(14, 16, 20);
    painter.rect_filled(resp.rect, 0.0, workspace_bg);

    let pan_active = ui.input(|i| is_pan_input(i));

    let zoom = editor.canvas_zoom;
    let canvas_w = scr_w * zoom;
    let canvas_h = scr_h * zoom;
    let offset = artboard_origin(resp.rect, canvas_w, canvas_h, editor.canvas_pan);

    if pan_active {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }

    let canvas_rect = egui::Rect::from_min_size(offset, egui::vec2(canvas_w, canvas_h));

        draw_canvas_background(ctx, editor, &painter, canvas_rect);

        painter.rect_stroke(
            canvas_rect,
            4.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
        );

        if editor.show_grid {
            draw_grid(&painter, offset, canvas_w, canvas_h, zoom);
        }

        let ordered = collect_draw_order(project, project.root_id);
        let mut hits: Vec<HitEntry> = Vec::new();
        let mut element_drag_active = false;
        let mut resize_drag_active = false;

        for id in ordered {
            let abs = absolute_rect(project, id, scr_w, scr_h);
            let screen_rect = Rect {
                x: offset.x + abs.x * zoom,
                y: offset.y + abs.y * zoom,
                w: abs.w * zoom,
                h: abs.h * zoom,
            };

            let el = match project.element(id) {
                Some(e) => e.clone(),
                None => continue,
            };

            if !el.visible {
                continue;
            }

            let is_selected = editor.is_selected(id);
            let primary = editor.primary() == Some(id);
            let egui_rect = egui::Rect::from_min_size(
                egui::pos2(screen_rect.x, screen_rect.y),
                egui::vec2(screen_rect.w.max(1.0), screen_rect.h.max(1.0)),
            );

            let rounding = (el.corner_radius * zoom).min(screen_rect.w / 2.0);

            let hit_rect = egui::Rect::from_min_size(
                egui::pos2(screen_rect.x, screen_rect.y),
                egui::vec2(screen_rect.w.max(4.0), screen_rect.h.max(4.0)),
            );

            hits.push(HitEntry::from_element(id, hit_rect, el.kind));

            let elem_sense = if is_selected {
                egui::Sense::click_and_drag()
            } else {
                // Невыбранный дочерний элемент не перехватывает drag у родителя.
                egui::Sense::click()
            };
            let elem_resp =
                ui.interact(hit_rect, egui::Id::new(id), elem_sense);

            draw_element_background(
                ctx,
                editor,
                &painter,
                &el,
                &screen_rect,
                rounding,
                egui_rect,
                elem_resp.hovered(),
                elem_resp.is_pointer_button_down_on(),
            );

            let parent_w = el.w.preview(scr_w, scr_h);
            let parent_h = el.h.preview(scr_w, scr_h);

            let layer_hit = draw_image_layers(
                ui,
                ctx,
                editor,
                project,
                id,
                &el,
                &screen_rect,
                parent_w,
                parent_h,
                zoom,
                scr_w,
                scr_h,
                pan_active,
            );

            draw_element_text(
                &painter,
                &el,
                &screen_rect,
                parent_w,
                parent_h,
                zoom,
                scr_w,
                scr_h,
                elem_resp.hovered(),
                elem_resp.is_pointer_button_down_on(),
            );

            if el.kind == ElementKind::TextEntry {
                draw_text_entry_preview(&painter, &el, &screen_rect, zoom);
            }

            let stroke = if is_selected {
                egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 160, 255))
            } else {
                egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(255, 255, 255, 40))
            };
            painter.rect_stroke(egui_rect, rounding, stroke);

            painter.text(
                egui::pos2(screen_rect.x + 2.0, screen_rect.y + 2.0),
                egui::Align2::LEFT_TOP,
                if el.locked {
                    format!("[L] {}", el.name)
                } else {
                    el.name.clone()
                },
                egui::FontId::proportional(10.0),
                egui::Color32::from_rgba_unmultiplied(200, 200, 200, 180),
            );

            if elem_resp.clicked() && !pan_active && !layer_hit {
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

            let can_move = !pan_active
                && !layer_hit
                && editor.selected_sub_layer.is_none()
                && el.dock == DockMode::None
                && !el.locked
                && editor.is_selected(id)
                && editor.resize_drag.is_none()
                && editor.image_layer_move.is_none()
                && editor.image_layer_resize_drag.is_none();

            if can_move && editor.element_drag.is_none() {
                let pointer_moved = ui.input(|i| i.pointer.delta().length_sq() > 2.0);
                if elem_resp.drag_started()
                    || (elem_resp.is_pointer_button_down_on() && pointer_moved)
                {
                    if let Some(pointer) = pointer_pos(ui) {
                        editor.element_drag = Some(super::ElementDragState {
                            pointer_start: pointer,
                            origins: collect_move_origins(
                                project,
                                &editor.selection,
                                scr_w,
                                scr_h,
                            ),
                        });
                    }
                }
            }

            if primary
                && el.dock == DockMode::None
                && editor.selected_sub_layer.is_none()
                && !el.locked
            {
                let (pw, ph, rel_parent) = parent_dims(project, id, scr_w, scr_h);
                resize_drag_active |= draw_resize_handles(
                    ui,
                    editor,
                    project,
                    id,
                    &screen_rect,
                    pw,
                    ph,
                    rel_parent,
                    zoom,
                    scr_w,
                    scr_h,
                );
                if editor.resize_drag.is_some() {
                    editor.element_drag = None;
                }
            }
        }

        let primary_down = ui.input(|i| i.pointer.primary_down());

        if editor.image_layer_resize_drag.is_some() && primary_down {
            resize_drag_active = true;
        }

        if let Some(drag) = &editor.element_drag {
            if primary_down {
                if let Some(delta) = canvas_drag_delta(ui, drag.pointer_start, zoom) {
                    apply_element_drag(editor, project, &drag, delta, scr_w, scr_h);
                    element_drag_active = true;
                }
            } else {
                editor.element_drag = None;
            }
        }

        if let Some(layer_move) = &editor.image_layer_move {
            if primary_down {
                if let Some(delta) = canvas_drag_delta(ui, layer_move.pointer_start, zoom) {
                    if let Some(el_mut) = project.element_mut(layer_move.element_id) {
                        if let Some(img) = el_mut
                            .image_layers
                            .iter_mut()
                            .find(|l| l.id == layer_move.layer_id)
                        {
                            let snap = editor.snap_to_grid;
                            let nx = apply_drag_snap(layer_move.origin_x, delta.x, snap);
                            let ny = apply_drag_snap(layer_move.origin_y, delta.y, snap);
                            let parent_w = el_mut.w.preview(scr_w, scr_h);
                            let parent_h = el_mut.h.preview(scr_w, scr_h);
                            img.x = img.x.set_pixels_preserving_kind(
                                nx, parent_w, parent_h, scr_w, scr_h, NumAxis::X, true,
                            );
                            img.y = img.y.set_pixels_preserving_kind(
                                ny, parent_w, parent_h, scr_w, scr_h, NumAxis::Y, true,
                            );
                        }
                    }
                    element_drag_active = true;
                }
            } else {
                editor.image_layer_move = None;
            }
        }

        if !primary_down {
            editor.resize_drag = None;
            editor.image_layer_resize_drag = None;
        }

        editor.checkpoint_if_drag_start(project, element_drag_active || resize_drag_active);

        handle_image_drop(ui, ctx, editor, project, &hits, canvas_rect);

        handle_camera_input(ui, editor, resp.rect, pan_active, scr_w, scr_h);
}

fn draw_canvas_background(
    ctx: &egui::Context,
    editor: &mut EditorUi,
    painter: &egui::Painter,
    canvas_rect: egui::Rect,
) {
    let has_bg = editor.canvas_bg_visible && !editor.canvas_bg_path.is_empty();

    if has_bg {
        if let Some(tex) = editor
            .image_cache
            .texture_id(ctx, &editor.canvas_bg_path)
        {
            let tint = egui::Color32::from_rgba_unmultiplied(
                255,
                255,
                255,
                editor.canvas_bg_opacity,
            );
            painter.image(
                tex,
                canvas_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                tint,
            );
            return;
        }
    }

    painter.rect_filled(canvas_rect, 4.0, egui::Color32::from_rgb(18, 20, 26));
}

fn draw_element_background(
    ctx: &egui::Context,
    editor: &mut EditorUi,
    painter: &egui::Painter,
    el: &crate::model::UiElement,
    screen_rect: &Rect,
    rounding: f32,
    egui_rect: egui::Rect,
    hovered: bool,
    pressed: bool,
) {
    let preview_path = el.preview_image_path();
    let has_preview = !preview_path.is_empty();

    if el.bg_gradient.enabled {
        draw_gradient_fill(painter, egui_rect, rounding, &el.bg_gradient);
    } else if el.kind == ElementKind::EditablePanelImaged && has_preview {
        if let Some(tex) = editor.image_cache.texture_id(ctx, preview_path) {
            let tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, el.bg_image_alpha);
            painter.image(tex, egui_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), tint);
        }
    } else if el.paint_background && el.bg_color[3] > 0 {
        let bg = button_preview_color(el, hovered, pressed);
        let color = egui::Color32::from_rgba_unmultiplied(bg[0], bg[1], bg[2], bg[3]);
        painter.rect_filled(egui_rect, rounding, color);
    }

    if el.kind == ElementKind::Image {
        if has_preview {
            if let Some(tex) = editor.image_cache.texture_id(ctx, preview_path) {
                painter.image(
                    tex,
                    egui_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        } else {
            painter.rect_stroke(
                egui_rect,
                rounding,
                egui::Stroke::new(1.0, egui::Color32::from_gray(120)),
            );
            painter.text(
                egui::pos2(screen_rect.x + screen_rect.w / 2.0, screen_rect.y + screen_rect.h / 2.0),
                egui::Align2::CENTER_CENTER,
                "img",
                egui::FontId::proportional(24.0),
                egui::Color32::from_gray(140),
            );
        }
    }
}

fn button_preview_color(el: &crate::model::UiElement, hovered: bool, pressed: bool) -> [u8; 4] {
    if el.kind != ElementKind::Button {
        return el.bg_color;
    }
    if el.button_disabled {
        return [
            el.bg_color[0] / 2,
            el.bg_color[1] / 2,
            el.bg_color[2] / 2,
            el.bg_color[3],
        ];
    }
    if pressed && el.button_pressed_enabled {
        return el.button_pressed_bg;
    }
    if hovered && el.button_hover_enabled {
        return el.button_hover_bg;
    }
    el.bg_color
}

fn draw_gradient_fill(
    painter: &egui::Painter,
    rect: egui::Rect,
    rounding: f32,
    grad: &crate::model::GradientFill,
) {
    let steps = grad.steps.clamp(4, 128) as usize;
    let c0 = egui::Color32::from_rgba_unmultiplied(
        grad.color_start[0],
        grad.color_start[1],
        grad.color_start[2],
        grad.color_start[3],
    );
    let c1 = egui::Color32::from_rgba_unmultiplied(
        grad.color_end[0],
        grad.color_end[1],
        grad.color_end[2],
        grad.color_end[3],
    );

    for i in 0..steps {
        let t0 = i as f32 / steps as f32;
        let t1 = (i + 1) as f32 / steps as f32;
        let color = lerp_color(c0, c1, (t0 + t1) * 0.5);
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
        painter.rect_filled(strip, rounding.min(4.0), color);
    }
}

fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    egui::Color32::from_rgba_unmultiplied(
        lerp_u8(a.r(), b.r(), t),
        lerp_u8(a.g(), b.g(), t),
        lerp_u8(a.b(), b.b(), t),
        lerp_u8(a.a(), b.a(), t),
    )
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

fn text_align_egui(align: TextAlign, lx: f32, ly: f32, _rect: &Rect) -> (egui::Pos2, egui::Align2) {
    let y = ly + 8.0;
    match align {
        TextAlign::Left => (egui::pos2(lx, y), egui::Align2::LEFT_CENTER),
        TextAlign::Center => (egui::pos2(lx, y), egui::Align2::CENTER_CENTER),
        TextAlign::Right => (egui::pos2(lx, y), egui::Align2::RIGHT_CENTER),
    }
}

fn button_preview_text_color(
    el: &crate::model::UiElement,
    base: [u8; 4],
    hovered: bool,
    pressed: bool,
) -> [u8; 4] {
    if el.kind != ElementKind::Button {
        return base;
    }
    if el.button_disabled {
        return [
            base[0] / 2,
            base[1] / 2,
            base[2] / 2,
            base[3],
        ];
    }
    if pressed && el.button_pressed_enabled {
        return el.button_pressed_text_color;
    }
    if hovered && el.button_hover_enabled {
        return el.button_hover_text_color;
    }
    base
}

fn draw_element_text(
    painter: &egui::Painter,
    el: &crate::model::UiElement,
    screen_rect: &Rect,
    parent_w: f32,
    parent_h: f32,
    zoom: f32,
    scr_w: f32,
    scr_h: f32,
    hovered: bool,
    pressed: bool,
) {
    let text_preview = match el.kind {
        ElementKind::Frame | ElementKind::Button => Some(el.text.preview()),
        _ => None,
    };

    if !el.text_layers.is_empty() {
        for layer in &el.text_layers {
            let text = layer.text.preview();
            if text.is_empty() {
                continue;
            }
            let tc = button_preview_text_color(el, layer.text_color, hovered, pressed);
            let tc = egui::Color32::from_rgba_unmultiplied(tc[0], tc[1], tc[2], tc[3]);
            let lx = screen_rect.x + layer.x.preview_in_parent(parent_w, parent_h, scr_w, scr_h) * zoom;
            let ly = screen_rect.y + layer.y.preview_in_parent(parent_w, parent_h, scr_w, scr_h) * zoom;
            let align = text_align_egui(layer.align, lx, ly, screen_rect);
            painter.text(
                align.0,
                align.1,
                text,
                egui::FontId::proportional((layer.font_size as f32 * zoom).clamp(8.0, 32.0)),
                tc,
            );
        }
        return;
    }

    if let Some(text) = text_preview {
        if !text.is_empty() {
            let tc = button_preview_text_color(el, el.text_color, hovered, pressed);
            let tc = egui::Color32::from_rgba_unmultiplied(tc[0], tc[1], tc[2], tc[3]);
            painter.text(
                egui::pos2(
                    screen_rect.x + screen_rect.w / 2.0,
                    screen_rect.y + screen_rect.h / 2.0,
                ),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional((el.font_size as f32 * zoom).clamp(8.0, 32.0)),
                tc,
            );
        }
    }
}

fn draw_image_layers(
    ui: &egui::Ui,
    ctx: &egui::Context,
    editor: &mut EditorUi,
    project: &mut Project,
    element_id: Uuid,
    el: &crate::model::UiElement,
    screen_rect: &Rect,
    parent_w: f32,
    parent_h: f32,
    zoom: f32,
    scr_w: f32,
    scr_h: f32,
    pan_active: bool,
) -> bool {
    let mut any_layer_hit = false;

    for layer in &el.image_layers {
        let lx = screen_rect.x + layer.x.preview_in_parent(parent_w, parent_h, scr_w, scr_h) * zoom;
        let ly = screen_rect.y + layer.y.preview_in_parent(parent_w, parent_h, scr_w, scr_h) * zoom;
        let lw = layer.w.preview_in_parent(parent_w, parent_h, scr_w, scr_h) * zoom;
        let lh = layer.h.preview_in_parent(parent_w, parent_h, scr_w, scr_h) * zoom;
        let layer_rect = egui::Rect::from_min_size(
            egui::pos2(lx, ly),
            egui::vec2(lw.max(4.0), lh.max(4.0)),
        );

        let selected = editor.selected_sub_layer == Some(SubLayerRef::Image(layer.id));
        let layer_sense = if selected {
            egui::Sense::click_and_drag()
        } else {
            egui::Sense::click()
        };

        let resp = ui.interact(
            layer_rect,
            egui::Id::new((element_id, layer.id, "img")),
            layer_sense,
        );

        if !layer.local_image_path.is_empty() {
            if let Some(tex) = editor.image_cache.texture_id(ctx, &layer.local_image_path) {
                let tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, layer.alpha);
                ui.painter().image(
                    tex,
                    layer_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    tint,
                );
            }
        } else {
            ui.painter().rect_stroke(
                layer_rect,
                2.0,
                egui::Stroke::new(1.0, egui::Color32::from_gray(100)),
            );
            ui.painter().text(
                layer_rect.center(),
                egui::Align2::CENTER_CENTER,
                "img",
                egui::FontId::proportional(16.0),
                egui::Color32::from_gray(140),
            );
        }

        let stroke = if selected {
            egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 180, 60))
        } else {
            egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 200, 100, 80))
        };
        ui.painter().rect_stroke(layer_rect, 2.0, stroke);

        if resp.clicked() && !pan_active {
            editor.select_single(element_id);
            editor.selected_sub_layer = Some(SubLayerRef::Image(layer.id));
            any_layer_hit = true;
        }

        if resp.drag_started() && !pan_active && selected && editor.image_layer_move.is_none() {
            if let Some(pointer) = pointer_pos(ui) {
                let ox = layer.x.preview_in_parent(parent_w, parent_h, scr_w, scr_h);
                let oy = layer.y.preview_in_parent(parent_w, parent_h, scr_w, scr_h);
                editor.image_layer_move = Some(super::ImageLayerMoveState {
                    element_id,
                    layer_id: layer.id,
                    pointer_start: pointer,
                    origin_x: ox,
                    origin_y: oy,
                });
                any_layer_hit = true;
            }
        }

        if selected {
            draw_image_layer_resize(
                ui,
                editor,
                project,
                element_id,
                layer.id,
                &Rect {
                    x: lx,
                    y: ly,
                    w: lw,
                    h: lh,
                },
                parent_w,
                parent_h,
                zoom,
                scr_w,
                scr_h,
            );
        }
    }

    any_layer_hit
}

fn draw_image_layer_resize(
    ui: &egui::Ui,
    editor: &mut super::EditorUi,
    project: &mut Project,
    element_id: Uuid,
    layer_id: Uuid,
    rect: &Rect,
    parent_w: f32,
    parent_h: f32,
    zoom: f32,
    scr_w: f32,
    scr_h: f32,
) {
    let handle = 8.0;
    let corners = [
        (rect.x, rect.y, ResizeCorner::TopLeft),
        (rect.x + rect.w, rect.y, ResizeCorner::TopRight),
        (rect.x, rect.y + rect.h, ResizeCorner::BottomLeft),
        (rect.x + rect.w, rect.y + rect.h, ResizeCorner::BottomRight),
    ];

    for (cx, cy, corner) in corners {
        let r = egui::Rect::from_center_size(egui::pos2(cx, cy), egui::vec2(handle, handle));
        ui.painter().rect_filled(r, 1.0, egui::Color32::from_rgb(255, 200, 80));

        let resp = ui.interact(
            r,
            egui::Id::new((element_id, layer_id, corner as u8, "img_resize")),
            egui::Sense::drag(),
        );
        if resp.drag_started() {
            if let Some(el) = project.element(element_id) {
                if let Some(img) = el.image_layers.iter().find(|l| l.id == layer_id) {
                    if let Some(pointer) = pointer_pos(ui) {
                        editor.image_layer_resize_drag = Some(super::ImageLayerResizeDrag {
                            element_id,
                            layer_id,
                            corner: corner as u8,
                            pointer_start: pointer,
                            x: img.x.preview_in_parent(parent_w, parent_h, scr_w, scr_h),
                            y: img.y.preview_in_parent(parent_w, parent_h, scr_w, scr_h),
                            w: img.w.preview_in_parent(parent_w, parent_h, scr_w, scr_h),
                            h: img.h.preview_in_parent(parent_w, parent_h, scr_w, scr_h),
                        });
                    }
                }
            }
        }
        if let Some(orig) = editor
            .image_layer_resize_drag
            .filter(|o| o.element_id == element_id && o.layer_id == layer_id && o.corner == corner as u8)
        {
            if ui.input(|i| i.pointer.primary_down()) {
                if let Some(d) = canvas_drag_delta(ui, orig.pointer_start, zoom) {
                    let snap = editor.snap_to_grid;
                    let mut x = orig.x;
                    let mut y = orig.y;
                    let mut w = orig.w;
                    let mut h = orig.h;
                    match corner {
                        ResizeCorner::TopLeft => {
                            x += d.x;
                            y += d.y;
                            w -= d.x;
                            h -= d.y;
                        }
                        ResizeCorner::TopRight => {
                            y += d.y;
                            w += d.x;
                            h -= d.y;
                        }
                        ResizeCorner::BottomLeft => {
                            x += d.x;
                            w -= d.x;
                            h += d.y;
                        }
                        ResizeCorner::BottomRight => {
                            w += d.x;
                            h += d.y;
                        }
                    }
                    w = w.max(4.0);
                    h = h.max(4.0);
                    x = apply_drag_snap(orig.x, x - orig.x, snap);
                    y = apply_drag_snap(orig.y, y - orig.y, snap);
                    w = snap_size(w, 4.0, snap);
                    h = snap_size(h, 4.0, snap);

                    if let Some(el) = project.element_mut(element_id) {
                        if let Some(img) = el.image_layers.iter_mut().find(|l| l.id == layer_id) {
                            img.x = img.x.set_pixels_preserving_kind(
                                x, parent_w, parent_h, scr_w, scr_h, NumAxis::X, true,
                            );
                            img.y = img.y.set_pixels_preserving_kind(
                                y, parent_w, parent_h, scr_w, scr_h, NumAxis::Y, true,
                            );
                            img.w = img.w.set_pixels_preserving_kind(
                                w, parent_w, parent_h, scr_w, scr_h, NumAxis::W, true,
                            );
                            img.h = img.h.set_pixels_preserving_kind(
                                h, parent_w, parent_h, scr_w, scr_h, NumAxis::H, true,
                            );
                        }
                    }
                }
            }
        }
    }
}

fn draw_text_entry_preview(
    painter: &egui::Painter,
    el: &crate::model::UiElement,
    screen_rect: &Rect,
    zoom: f32,
) {
    if el.text_entry_draw_background && el.bg_color[3] > 0 {
        let color = egui::Color32::from_rgba_unmultiplied(
            el.bg_color[0],
            el.bg_color[1],
            el.bg_color[2],
            el.bg_color[3],
        );
        painter.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(screen_rect.x, screen_rect.y),
                egui::vec2(screen_rect.w, screen_rect.h),
            ),
            2.0,
            color,
        );
    }

    if el.text_entry_draw_border {
        painter.rect_stroke(
            egui::Rect::from_min_size(
                egui::pos2(screen_rect.x, screen_rect.y),
                egui::vec2(screen_rect.w, screen_rect.h),
            ),
            2.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(100)),
        );
    }

    let text = el.text.preview();
    if !text.is_empty() && el.text_layers.is_empty() {
        let tc = egui::Color32::from_rgba_unmultiplied(
            el.text_color[0],
            el.text_color[1],
            el.text_color[2],
            el.text_color[3],
        );
        painter.text(
            egui::pos2(screen_rect.x + 6.0 * zoom, screen_rect.y + screen_rect.h / 2.0),
            egui::Align2::LEFT_CENTER,
            text,
            egui::FontId::proportional((el.font_size as f32 * zoom).clamp(8.0, 28.0)),
            tc,
        );
    }
}

fn handle_image_drop(
    ui: &egui::Ui,
    ctx: &egui::Context,
    editor: &mut EditorUi,
    project: &mut Project,
    hits: &[HitEntry],
    canvas_rect: egui::Rect,
) {
    let (dropped, pointer) = ui.input(|i| {
        let paths: Vec<String> = i
            .raw
            .dropped_files
            .iter()
            .filter_map(|f| f.path.as_ref().map(|p| p.to_string_lossy().into_owned()))
            .filter(|p| is_image_file(p))
            .collect();
        (paths, i.pointer.interact_pos())
    });

    let Some(path) = dropped.first() else {
        return;
    };
    let Some(pos) = pointer else {
        return;
    };

    for hit in hits.iter().rev() {
        if hit.rect.contains(pos) && hit.accepts_image {
            if let Some(el) = project.element(hit.id) {
                if el.kind == ElementKind::Image {
                    if let Some(el_mut) = project.element_mut(hit.id) {
                        let old = el_mut.local_image_path.clone();
                        el_mut.assign_local_image(path.clone());
                        if !old.is_empty() {
                            editor.image_cache.invalidate(&old);
                        }
                        editor.image_cache.invalidate(path);
                        editor.set_status(ctx, format!("Изображение: {}", el_mut.name));
                        editor.select_single(hit.id);
                    }
                    return;
                }
                if el.kind.supports_image_layers() {
                    if let Some(el_mut) = project.element_mut(hit.id) {
                        let n = el_mut.image_layers.len() + 1;
                        let mut layer = crate::model::ImageLayer::new(format!("Картинка_{n}"));
                        layer.assign_local_image(path.clone());
                        let layer_id = layer.id;
                        el_mut.image_layers.push(layer);
                        editor.image_cache.invalidate(path);
                        editor.select_single(hit.id);
                        editor.selected_sub_layer = Some(SubLayerRef::Image(layer_id));
                        editor.set_status(ctx, "Добавлен слой изображения");
                    }
                    return;
                }
            }
        }
    }

    if canvas_rect.contains(pos) {
        let old = editor.canvas_bg_path.clone();
        editor.canvas_bg_path = path.clone();
        editor.canvas_bg_visible = true;
        if !old.is_empty() {
            editor.image_cache.invalidate(&old);
        }
        editor.image_cache.invalidate(path);
        editor.set_status(ctx, "Скриншот установлен как фон холста");
    }
}

fn is_pan_input(i: &egui::InputState) -> bool {
    i.pointer.middle_down()
        || i.pointer.secondary_down()
        || (i.key_down(egui::Key::Space) && i.pointer.primary_down())
}

fn artboard_origin(
    workspace: egui::Rect,
    canvas_w: f32,
    canvas_h: f32,
    pan: egui::Vec2,
) -> egui::Pos2 {
    let margin = 8.0;
    let mut pos = workspace.min;
    let free_x = workspace.width() - canvas_w - margin * 2.0;
    let free_y = workspace.height() - canvas_h - margin * 2.0;
    if free_x > 0.0 {
        pos.x += free_x * 0.5 + margin;
    } else {
        pos.x += margin;
    }
    if free_y > 0.0 {
        pos.y += free_y * 0.5 + margin;
    } else {
        pos.y += margin;
    }
    pos + pan
}

fn handle_camera_input(
    ui: &egui::Ui,
    editor: &mut EditorUi,
    workspace: egui::Rect,
    pan_active: bool,
    scr_w: f32,
    scr_h: f32,
) {
    if pan_active {
        let delta = ui.input(|i| i.pointer.delta());
        if delta != egui::Vec2::ZERO {
            editor.canvas_pan += delta;
        }
    }

    let (scroll, pointer) = ui.input(|i| (i.smooth_scroll_delta.y, i.pointer.hover_pos()));

    if scroll == 0.0 {
        return;
    }

    let Some(pointer) = pointer else {
        return;
    };

    if !workspace.contains(pointer) {
        return;
    }

    let old_zoom = editor.canvas_zoom;
    let factor = 1.0 + scroll * 0.002;
    let new_zoom = (old_zoom * factor).clamp(0.1, 3.0);
    if (new_zoom - old_zoom).abs() < f32::EPSILON {
        return;
    }

    let old_w = scr_w * old_zoom;
    let old_h = scr_h * old_zoom;
    let origin = artboard_origin(workspace, old_w, old_h, editor.canvas_pan);
    let rel = pointer - origin;
    editor.canvas_pan += rel * (1.0 - new_zoom / old_zoom);
    editor.canvas_zoom = new_zoom;
}

fn draw_resize_handles(
    ui: &egui::Ui,
    editor: &mut EditorUi,
    project: &mut Project,
    id: Uuid,
    rect: &Rect,
    parent_w: f32,
    parent_h: f32,
    relative_to_parent: bool,
    zoom: f32,
    scr_w: f32,
    scr_h: f32,
) -> bool {
    let handle = 8.0;
    let painter = ui.painter();
    let corners = [
        (rect.x, rect.y, ResizeCorner::TopLeft),
        (rect.x + rect.w, rect.y, ResizeCorner::TopRight),
        (rect.x, rect.y + rect.h, ResizeCorner::BottomLeft),
        (rect.x + rect.w, rect.y + rect.h, ResizeCorner::BottomRight),
    ];
    let mut resize_drag_active = false;

    for (cx, cy, corner) in corners {
        let r = egui::Rect::from_center_size(egui::pos2(cx, cy), egui::vec2(handle, handle));
        painter.rect_filled(r, 1.0, egui::Color32::WHITE);
        painter.rect_stroke(r, 1.0, egui::Stroke::new(1.0, egui::Color32::BLACK));

        let resp = ui.interact(r, egui::Id::new((id, corner as u8)), egui::Sense::drag());
        if resp.drag_started() {
            if let Some(el) = project.element(id) {
                if let Some(pointer) = pointer_pos(ui) {
                    editor.resize_drag = Some(super::ResizeDragState {
                        element_id: id,
                        corner: corner as u8,
                        pointer_start: pointer,
                        x: el.x.preview_in_parent(parent_w, parent_h, scr_w, scr_h),
                        y: el.y.preview_in_parent(parent_w, parent_h, scr_w, scr_h),
                        w: el.w.preview_in_parent(parent_w, parent_h, scr_w, scr_h),
                        h: el.h.preview_in_parent(parent_w, parent_h, scr_w, scr_h),
                    });
                }
            }
        }
        if let Some(orig) = editor
            .resize_drag
            .filter(|o| o.element_id == id && o.corner == corner as u8)
        {
            if ui.input(|i| i.pointer.primary_down()) {
                if let Some(d) = canvas_drag_delta(ui, orig.pointer_start, zoom) {
                    resize_drag_active = true;
                    let snap = editor.snap_to_grid;
                    let mut x = orig.x;
                    let mut y = orig.y;
                    let mut w = orig.w;
                    let mut h = orig.h;

                    match corner {
                        ResizeCorner::TopLeft => {
                            x += d.x;
                            y += d.y;
                            w -= d.x;
                            h -= d.y;
                        }
                        ResizeCorner::TopRight => {
                            y += d.y;
                            w += d.x;
                            h -= d.y;
                        }
                        ResizeCorner::BottomLeft => {
                            x += d.x;
                            w -= d.x;
                            h += d.y;
                        }
                        ResizeCorner::BottomRight => {
                            w += d.x;
                            h += d.y;
                        }
                    }

                    w = w.max(8.0);
                    h = h.max(8.0);
                    x = apply_drag_snap(orig.x, x - orig.x, snap);
                    y = apply_drag_snap(orig.y, y - orig.y, snap);
                    w = snap_size(w, 8.0, snap);
                    h = snap_size(h, 8.0, snap);

                    if let Some(el) = project.element_mut(id) {
                        el.x = el.x.set_pixels_preserving_kind(
                            x, parent_w, parent_h, scr_w, scr_h, NumAxis::X, relative_to_parent,
                        );
                        el.y = el.y.set_pixels_preserving_kind(
                            y, parent_w, parent_h, scr_w, scr_h, NumAxis::Y, relative_to_parent,
                        );
                        el.w = el.w.set_pixels_preserving_kind(
                            w, parent_w, parent_h, scr_w, scr_h, NumAxis::W, relative_to_parent,
                        );
                        el.h = el.h.set_pixels_preserving_kind(
                            h, parent_w, parent_h, scr_w, scr_h, NumAxis::H, relative_to_parent,
                        );
                    }
                }
            }
        }
    }

    resize_drag_active
}

#[derive(Clone, Copy)]
enum ResizeCorner {
    TopLeft = 0,
    TopRight = 1,
    BottomLeft = 2,
    BottomRight = 3,
}

fn pointer_pos(ui: &egui::Ui) -> Option<egui::Pos2> {
    ui.input(|i| i.pointer.interact_pos().or(i.pointer.hover_pos()))
}

fn canvas_drag_delta(ui: &egui::Ui, pointer_start: egui::Pos2, zoom: f32) -> Option<egui::Vec2> {
    pointer_pos(ui).map(|p| (p - pointer_start) / zoom)
}

fn apply_element_drag(
    editor: &EditorUi,
    project: &mut Project,
    drag: &super::ElementDragState,
    delta: egui::Vec2,
    scr_w: f32,
    scr_h: f32,
) {
    let snap = editor.snap_to_grid;
    for (sid, ox, oy) in &drag.origins {
        let (spw, sph, srel) = parent_dims(project, *sid, scr_w, scr_h);
        if let Some(el_mut) = project.element_mut(*sid) {
            if el_mut.locked || el_mut.dock != DockMode::None {
                continue;
            }
            let nx = apply_drag_snap(*ox, delta.x, snap);
            let ny = apply_drag_snap(*oy, delta.y, snap);
            el_mut.x = el_mut.x.set_pixels_preserving_kind(
                nx, spw, sph, scr_w, scr_h, NumAxis::X, srel,
            );
            el_mut.y = el_mut.y.set_pixels_preserving_kind(
                ny, spw, sph, scr_w, scr_h, NumAxis::Y, srel,
            );
        }
    }
}

fn collect_move_origins(
    project: &Project,
    selection: &[Uuid],
    scr_w: f32,
    scr_h: f32,
) -> Vec<(Uuid, f32, f32)> {
    selection
        .iter()
        .filter_map(|sid| {
            let el = project.element(*sid)?;
            if el.locked || el.dock != DockMode::None {
                return None;
            }
            let (spw, sph, _) = parent_dims(project, *sid, scr_w, scr_h);
            Some((
                *sid,
                el.x.preview_in_parent(spw, sph, scr_w, scr_h),
                el.y.preview_in_parent(spw, sph, scr_w, scr_h),
            ))
        })
        .collect()
}

fn draw_grid(painter: &egui::Painter, offset: egui::Pos2, w: f32, h: f32, zoom: f32) {
    let step = (20.0 * zoom).max(10.0);
    let color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 15);
    let mut x = 0.0;
    while x <= w {
        painter.line_segment(
            [offset + egui::vec2(x, 0.0), offset + egui::vec2(x, h)],
            egui::Stroke::new(1.0, color),
        );
        x += step;
    }
    let mut y = 0.0;
    while y <= h {
        painter.line_segment(
            [offset + egui::vec2(0.0, y), offset + egui::vec2(w, y)],
            egui::Stroke::new(1.0, color),
        );
        y += step;
    }
}

fn collect_draw_order(project: &Project, root: Uuid) -> Vec<Uuid> {
    let mut out = Vec::new();
    collect(project, root, &mut out);
    out
}

fn collect(project: &Project, id: Uuid, out: &mut Vec<Uuid>) {
    out.push(id);
    for child in project.children_of(id) {
        collect(project, child.id, out);
    }
}

fn parent_dims(project: &Project, id: Uuid, scr_w: f32, scr_h: f32) -> (f32, f32, bool) {
    let el = project.element(id).unwrap();
    if let Some(parent_id) = el.parent {
        let pr = absolute_rect(project, parent_id, scr_w, scr_h);
        (pr.w, pr.h, true)
    } else {
        (scr_w, scr_h, false)
    }
}

fn absolute_rect(project: &Project, id: Uuid, scr_w: f32, scr_h: f32) -> Rect {
    let el = project.element(id).unwrap();

    if let Some(parent_id) = el.parent {
        let parent_rect = absolute_rect(project, parent_id, scr_w, scr_h);
        let pw = parent_rect.w;
        let ph = parent_rect.h;
        Rect {
            x: parent_rect.x + el.x.preview_in_parent(pw, ph, scr_w, scr_h),
            y: parent_rect.y + el.y.preview_in_parent(pw, ph, scr_w, scr_h),
            w: el.w.preview_in_parent(pw, ph, scr_w, scr_h),
            h: el.h.preview_in_parent(pw, ph, scr_w, scr_h),
        }
    } else {
        Rect {
            x: el.x.preview(scr_w, scr_h),
            y: el.y.preview(scr_w, scr_h),
            w: el.w.preview(scr_w, scr_h),
            h: el.h.preview(scr_w, scr_h),
        }
    }
}
