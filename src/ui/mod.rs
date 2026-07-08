pub mod canvas;
pub mod hierarchy;
pub mod history;
pub mod image_cache;
pub mod input;
pub mod properties;
pub mod scripts;
pub mod theme;
pub mod toolbar;

use eframe::egui;
use uuid::Uuid;

use crate::model::{ElementKind, Project, UiElement};

use self::history::History;
use self::image_cache::{pick_image_file, ImageCache};
use self::theme::{sidebar_section, tool_button};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubLayerRef {
    Text(uuid::Uuid),
    Image(uuid::Uuid),
}

pub struct EditorUi {
    pub selection: Vec<Uuid>,
    pub selected_sub_layer: Option<SubLayerRef>,
    pub canvas_zoom: f32,
    pub canvas_pan: egui::Vec2,
    pub show_grid: bool,
    pub snap_to_grid: bool,
    pub status_message: Option<(String, f64)>,
    pub pending_add_kind: Option<ElementKind>,
    pub pending_close_button: bool,
    pub show_code_window: bool,
    pub code_preview: String,
    pub image_cache: ImageCache,
    pub canvas_bg_path: String,
    pub canvas_bg_visible: bool,
    pub canvas_bg_opacity: u8,
    pub history: History,
    pub hierarchy_search: String,
    pub hierarchy_drag_id: Option<Uuid>,
    pub show_scripts_window: bool,
    pub scripts_target: Option<Uuid>,
    pub scripts_checkpointed: Option<Uuid>,
    drag_checkpointed: bool,
    /// Drag элемента: дельта от позиции указателя, не от egui::Response (rect двигается).
    pub(crate) element_drag: Option<ElementDragState>,
    pub(crate) resize_drag: Option<ResizeDragState>,
    pub(crate) image_layer_move: Option<ImageLayerMoveState>,
    pub(crate) image_layer_resize_drag: Option<ImageLayerResizeDrag>,
    pub left_panel_width: f32,
    pub right_panel_width: f32,
    splitter_drag: Option<SplitterDrag>,
}

#[derive(Clone, Copy)]
enum SplitterDrag {
    /// Запоминаем стартовую ширину соседа + уже накопленную дельту с прошлых
    /// кадров. `resp.drag_delta()` в egui возвращает дельту ТОЛЬКО за последний
    /// кадр, поэтому если писать `start + dx` напрямую, значение будет
    /// сбрасываться к `start + dx_этого_кадра` каждый кадр и панель будет
    /// «сопротивляться и возвращаться». Аккумулируем вручную.
    Left { start: f32, accum: f32 },
    Right { start: f32, accum: f32 },
}

#[derive(Clone)]
pub(crate) struct ElementDragState {
    pub pointer_start: egui::Pos2,
    pub origins: Vec<(Uuid, f32, f32)>,
}

#[derive(Clone, Copy)]
pub(crate) struct ImageLayerMoveState {
    pub element_id: Uuid,
    pub layer_id: Uuid,
    pub pointer_start: egui::Pos2,
    pub origin_x: f32,
    pub origin_y: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct ImageLayerResizeDrag {
    pub element_id: Uuid,
    pub layer_id: Uuid,
    pub corner: u8,
    pub pointer_start: egui::Pos2,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct ResizeDragState {
    pub element_id: Uuid,
    pub corner: u8,
    pub pointer_start: egui::Pos2,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Default for EditorUi {
    fn default() -> Self {
        Self {
            selection: Vec::new(),
            selected_sub_layer: None,
            canvas_zoom: 0.5,
            canvas_pan: egui::Vec2::ZERO,
            show_grid: true,
            snap_to_grid: true,
            status_message: None,
            pending_add_kind: None,
            pending_close_button: false,
            show_code_window: false,
            code_preview: String::new(),
            image_cache: ImageCache::default(),
            canvas_bg_path: String::new(),
            canvas_bg_visible: true,
            canvas_bg_opacity: 255,
            history: History::default(),
            hierarchy_search: String::new(),
            hierarchy_drag_id: None,
            show_scripts_window: false,
            scripts_target: None,
            scripts_checkpointed: None,
            drag_checkpointed: false,
            element_drag: None,
            resize_drag: None,
            image_layer_move: None,
            image_layer_resize_drag: None,
            left_panel_width: 248.0,
            right_panel_width: 300.0,
            splitter_drag: None,
        }
    }
}

impl EditorUi {
    pub fn primary(&self) -> Option<Uuid> {
        self.selection.last().copied()
    }

    pub fn is_selected(&self, id: Uuid) -> bool {
        self.selection.contains(&id)
    }

    pub fn select_single(&mut self, id: Uuid) {
        self.selection = vec![id];
        self.selected_sub_layer = None;
    }

    pub fn toggle_selection(&mut self, id: Uuid) {
        if let Some(pos) = self.selection.iter().position(|&x| x == id) {
            self.selection.remove(pos);
        } else {
            self.selection.push(id);
        }
    }

    pub fn select_range(&mut self, project: &Project, from: Uuid, to: Uuid) {
        let flat = flatten_hierarchy(project, project.root_id);
        let Some(a) = flat.iter().position(|&id| id == from) else {
            self.select_single(to);
            return;
        };
        let Some(b) = flat.iter().position(|&id| id == to) else {
            self.select_single(to);
            return;
        };
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        self.selection = flat[lo..=hi].to_vec();
        self.selected_sub_layer = None;
    }

    pub fn sync_selection_after_project_change(&mut self, project: &Project) {
        self.selection
            .retain(|id| project.element(*id).is_some());
        if self.selection.is_empty() {
            self.selection.push(project.root_id);
        }
        if let Some(t) = self.scripts_target {
            if project.element(t).is_none() {
                self.scripts_target = None;
                self.show_scripts_window = false;
                self.scripts_checkpointed = None;
            }
        }
    }

    pub fn checkpoint_if_drag_start(&mut self, project: &Project, dragging: bool) {
        if dragging && !self.drag_checkpointed {
            self.history.checkpoint(project);
            self.drag_checkpointed = true;
        }
        if !dragging {
            self.drag_checkpointed = false;
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, project: &mut Project) {
        input::handle_shortcuts(ctx, self, project);
        toolbar::show(ctx, self, project);

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
            let full_h = ui.available_height();
            // Ширина сплиттера должна совпадать с тем, что выделяет column_splitter.
            const SPLIT_W: f32 = 8.0;
            let total_w = ui.available_width();
            const MIN_CENTER: f32 = 200.0;

            // Желаемые ширины — то, что хочет пользователь. Никакого жёсткого
            // clamp здесь: реальные ограничения (120..800 для left, 160..800 для
            // right) применяются в column_splitter при записи в self.*_panel_width.
            let desired_left = self.left_panel_width;
            let desired_right = self.right_panel_width;

            let max_for_sides = (total_w - SPLIT_W * 2.0 - MIN_CENTER).max(0.0);
            // Применяем scaling К ЖЕЛАЕМЫМ ширинам, чтобы нарисовать.
            let (left_w, right_w) = if desired_left + desired_right > max_for_sides {
                let scale = max_for_sides / (desired_left + desired_right);
                (desired_left * scale, desired_right * scale)
            } else {
                (desired_left, desired_right)
            };

            let center_w = (total_w - left_w - right_w - SPLIT_W * 2.0).max(MIN_CENTER);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.set_min_height(full_h);

                ui.allocate_ui_with_layout(
                    egui::vec2(left_w, full_h),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        let panel_rect = ui.max_rect();
                        ui.painter()
                            .rect_filled(panel_rect, 0.0, egui::Color32::from_rgb(20, 22, 28));
                        ui.set_width(left_w);
                        ui.set_max_width(left_w);
                        egui::ScrollArea::vertical()
                            .id_salt("left_panel_scroll")
                            .show(ui, |ui| {
                                ui.set_width(left_w - 8.0);
                                show_left_panel(ui, ctx, self, project);
                            });
                    },
                );

                column_splitter(
                    ui,
                    self,
                    SplitterSide::Left,
                    full_h,
                    desired_left,
                );

                ui.allocate_ui_with_layout(
                    egui::vec2(center_w, full_h),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.set_width(center_w);
                        ui.set_max_width(center_w);
                        canvas::show(ui, ctx, self, project);
                        hierarchy::apply_pending_reorder(ui, self, project);
                    },
                );

                column_splitter(
                    ui,
                    self,
                    SplitterSide::Right,
                    full_h,
                    desired_right,
                );

                ui.allocate_ui_with_layout(
                    egui::vec2(right_w, full_h),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        let panel_rect = ui.max_rect();
                        ui.painter()
                            .rect_filled(panel_rect, 0.0, egui::Color32::from_rgb(20, 22, 28));
                        ui.set_width(right_w);
                        ui.set_max_width(right_w);
                        ui.label(theme::section_title("Свойства"));
                        ui.add_space(6.0);
                        egui::ScrollArea::vertical()
                            .id_salt("properties_scroll")
                            .show(ui, |ui| {
                                ui.set_width(right_w - 8.0);
                                properties::show(ui, ctx, self, project);
                            });
                    },
                );
            });

            if let Some((msg, until)) = &self.status_message {
                if ctx.input(|i| i.time) < *until {
                    egui::Area::new(egui::Id::new("status_toast"))
                        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -12.0))
                        .show(ctx, |ui| {
                            theme::card_frame().show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("●").color(theme::SUCCESS));
                                    ui.label(egui::RichText::new(msg).color(theme::SUCCESS));
                                });
                            });
                        });
                } else {
                    self.status_message = None;
                }
            }
        });

        if let Some(kind) = self.pending_add_kind.take() {
            self.history.checkpoint(project);
            let parent = self.primary().unwrap_or(project.root_id);
            let name = format!(
                "{}_{}",
                kind.label().trim_start_matches('D').replace(' ', ""),
                project.elements.len()
            );
            let el = UiElement::new(kind, name);
            let id = project.add_element(el, parent);
            self.select_single(id);
        }

        if self.pending_close_button {
            self.pending_close_button = false;
            self.history.checkpoint(project);
            let parent = self.primary().unwrap_or(project.root_id);
            if let Some(id) = project.add_close_button(parent) {
                self.select_single(id);
            }
        }

        if self.show_code_window {
            let mut open = self.show_code_window;
            egui::Window::new("GLua Preview")
                .default_size([600.0, 400.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.code_preview)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .desired_rows(20),
                    );
                });
            self.show_code_window = open;
        }

        scripts::show(ctx, self, project);
    }

    pub fn set_status(&mut self, ctx: &egui::Context, msg: impl Into<String>) {
        let until = ctx.input(|i| i.time) + 3.0;
        self.status_message = Some((msg.into(), until));
    }
}

fn column_splitter(
    ui: &mut egui::Ui,
    editor: &mut EditorUi,
    side: SplitterSide,
    height: f32,

    desired_w: f32,
) {
    // Широкая hit-zone (8px) поверх видимой тонкой полоски (2px) — проще поймать
    // мышью, и при этом визуально сплиттер остаётся тонким.
    const SPLIT_W: f32 = 8.0;
    const SPLIT_VISUAL_W: f32 = 2.0;
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(SPLIT_W, height), egui::Sense::drag());

    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }

    if resp.drag_started() {
        // Стартуем от ЖЕЛАЕМОЙ ширины (не от уже сжатого scaling'ом значения),
        // иначе при узком окне первое же движение будет «упираться».
        editor.splitter_drag = Some(match side {
            SplitterSide::Left => SplitterDrag::Left {
                start: desired_w,
                accum: 0.0,
            },
            SplitterSide::Right => SplitterDrag::Right {
                start: desired_w,
                accum: 0.0,
            },
        });
    }

    if resp.dragged() {
        let dx = resp.drag_delta().x;
        match (&mut editor.splitter_drag, side) {
            (
                Some(SplitterDrag::Left { start, accum }),
                SplitterSide::Left,
            ) => {
                *accum += dx;
                editor.left_panel_width = (*start + *accum).clamp(120.0, 800.0);
            }
            (
                Some(SplitterDrag::Right { start, accum }),
                SplitterSide::Right,
            ) => {
                *accum += dx;
                editor.right_panel_width = (*start - *accum).clamp(160.0, 800.0);
            }
            _ => {}
        }
    }

    if resp.drag_stopped() {
        editor.splitter_drag = None;
    }

    // Рисуем тонкую полоску по центру hit-zone.
    let color = if resp.hovered() || resp.dragged() {
        theme::ACCENT
    } else {
        theme::BORDER
    };
    let visual_rect = egui::Rect::from_min_size(
        egui::pos2(
            rect.left() + (SPLIT_W - SPLIT_VISUAL_W) * 0.5,
            rect.top(),
        ),
        egui::vec2(SPLIT_VISUAL_W, height),
    );
    ui.painter().rect_filled(visual_rect, 0.0, color);
}

enum SplitterSide {
    Left,
    Right,
}

fn show_left_panel(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    editor: &mut EditorUi,
    project: &mut Project,
) {
    sidebar_section(ui, "Добавить", |ui| {
        egui::Grid::new("element_palette")
            .num_columns(2)
            .spacing([6.0, 6.0])
            .show(ui, |ui| {
                for kind in [
                    ElementKind::Panel,
                    ElementKind::Frame,
                    ElementKind::Button,
                    ElementKind::EditablePanel,
                    ElementKind::EditablePanelImaged,
                    ElementKind::TextEntry,
                    ElementKind::Image,
                ] {
                    if ui
                        .button(kind.label())
                        .on_hover_text("Добавить к выбранному родителю")
                        .clicked()
                    {
                        editor.pending_add_kind = Some(kind);
                    }
                }
            });
        ui.add_space(6.0);
        if ui
            .button("CloseButton")
            .on_hover_text("Кнопка закрытия (DButton)")
            .clicked()
        {
            editor.pending_close_button = true;
        }
    });

    sidebar_section(ui, "Вид холста", |ui| {
        ui.horizontal(|ui| {
            ui.checkbox(&mut editor.show_grid, "Сетка");
            ui.checkbox(&mut editor.snap_to_grid, "Snap");
        });
        ui.add(
            egui::Slider::new(&mut editor.canvas_zoom, 0.1..=3.0)
                .logarithmic(true)
                .text("Масштаб"),
        );
        if ui.button("Сбросить вид").clicked() {
            editor.canvas_zoom = 0.5;
            editor.canvas_pan = egui::Vec2::ZERO;
        }
        egui::CollapsingHeader::new(theme::hint("Горячие клавиши"))
            .default_open(false)
            .show(ui, |ui| {
                ui.label(theme::hint(
                    "Del - удалить\n\
                     Стрелки - сдвиг (Shift x10)\n\
                     Ctrl+Z / Ctrl+Y - отмена\n\
                     Ctrl+клик - мультивыбор",
                ));
            });
    });

    sidebar_section(ui, "Палитра", |ui| {
        palette_ui(ui, project);
    });

    sidebar_section(ui, "Фон превью", |ui| {
        ui.label(theme::hint("Только в редакторе, не экспортируется"));
        ui.add_space(4.0);
        ui.checkbox(&mut editor.canvas_bg_visible, "Показывать");
        ui.add(
            egui::Slider::new(&mut editor.canvas_bg_opacity, 0..=255).text("Прозрачность"),
        );
        ui.horizontal(|ui| {
            if tool_button(ui, "Файл", "Выбрать скриншот").clicked() {
                if let Some(path) = pick_image_file() {
                    let old = editor.canvas_bg_path.clone();
                    editor.canvas_bg_path = path;
                    editor.canvas_bg_visible = true;
                    if !old.is_empty() {
                        editor.image_cache.invalidate(&old);
                    }
                    editor.image_cache.invalidate(&editor.canvas_bg_path);
                }
            }
            if tool_button(ui, "X", "Убрать фон").clicked() {
                let old = editor.canvas_bg_path.clone();
                editor.canvas_bg_path.clear();
                editor.image_cache.invalidate(&old);
            }
        });
        if !editor.canvas_bg_path.is_empty() {
            ui.label(theme::hint(&truncate_path(&editor.canvas_bg_path)));
            if let Some(tex) = editor.image_cache.texture_id(ctx, &editor.canvas_bg_path) {
                let w = ui.available_width().min(200.0);
                ui.image((tex, egui::vec2(w, w * 0.56)));
            }
        } else {
            ui.label(theme::hint("Перетащите изображение на холст"));
        }
    });

    sidebar_section(ui, "Иерархия", |ui| {
        hierarchy::show(ui, editor, project);
    });
}

fn palette_ui(ui: &mut egui::Ui, project: &mut Project) {
    let len = project.color_palette.len();
    let mut add_color = None;
    let mut palette_updates: Vec<(usize, [u8; 4])> = Vec::new();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
        for i in 0..len {
            let mut c = project.color_palette[i];
            let resp = ui.color_edit_button_srgba_unmultiplied(&mut c);
            if resp.changed() {
                palette_updates.push((i, c));
            }
        }
        if tool_button(ui, "+", "Добавить цвет").clicked() {
            project.color_palette.push([55, 120, 200, 255]);
        }
    });
    ui.add_space(6.0);
    ui.label(theme::hint("Клик по цвету - редактировать. Ниже - применить к фону."));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
        for c in &project.color_palette {
            let (r, g, b, a) = (c[0], c[1], c[2], c[3]);
            let preview = egui::Color32::from_rgba_unmultiplied(r, g, b, a);
            if ui
                .add(
                    egui::Button::new(" ")
                        .fill(preview)
                        .min_size(egui::vec2(22.0, 22.0)),
                )
                .on_hover_text("Применить к фону выбранного")
                .clicked()
            {
                add_color = Some(*c);
            }
        }
    });
    for (i, c) in palette_updates {
        project.color_palette[i] = c;
    }
    if let Some(c) = add_color {
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("palette_apply"), c));
    }
}

fn flatten_hierarchy(project: &Project, root: Uuid) -> Vec<Uuid> {
    let mut out = Vec::new();
    fn walk(project: &Project, id: Uuid, out: &mut Vec<Uuid>) {
        out.push(id);
        for child in project.children_ids(id) {
            walk(project, child, out);
        }
    }
    walk(project, root, &mut out);
    out
}

fn truncate_path(path: &str) -> String {
    if path.len() <= 40 {
        return path.to_string();
    }
    format!("...{}", &path[path.len().saturating_sub(37)..])
}
