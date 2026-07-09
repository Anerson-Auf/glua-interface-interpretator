use eframe::egui;

pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(72, 132, 220);
pub const ACCENT_DIM: egui::Color32 = egui::Color32::from_rgb(48, 88, 150);
pub const SUCCESS: egui::Color32 = egui::Color32::from_rgb(96, 200, 140);
pub const SURFACE: egui::Color32 = egui::Color32::from_rgb(30, 33, 42);
pub const SURFACE_RAISED: egui::Color32 = egui::Color32::from_rgb(36, 40, 50);
pub const BORDER: egui::Color32 = egui::Color32::from_rgb(52, 58, 72);

pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.menu_margin = egui::Margin::same(6.0);
    style.spacing.indent = 14.0;
    style.spacing.combo_width = 120.0;
    style.spacing.slider_width = 120.0;
    style.spacing.text_edit_width = 140.0;
    style.spacing.scroll.bar_width = 8.0;
    style.spacing.scroll.bar_inner_margin = 2.0;

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(20, 22, 28);
    visuals.window_fill = egui::Color32::from_rgb(26, 28, 36);
    visuals.extreme_bg_color = egui::Color32::from_rgb(14, 16, 20);
    visuals.faint_bg_color = SURFACE;
    visuals.code_bg_color = egui::Color32::from_rgb(18, 20, 26);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 4.0),
        blur: 12.0,
        spread: 0.0,
        color: egui::Color32::from_black_alpha(80),
    };
    visuals.window_rounding = egui::Rounding::same(10.0);
    visuals.menu_rounding = egui::Rounding::same(8.0);
    visuals.collapsing_header_frame = true;

    let round = egui::Rounding::same(6.0);
    visuals.widgets.noninteractive.rounding = round;
    visuals.widgets.inactive.rounding = round;
    visuals.widgets.hovered.rounding = round;
    visuals.widgets.active.rounding = round;
    visuals.widgets.open.rounding = round;

    visuals.widgets.noninteractive.bg_fill = SURFACE;
    visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::from_rgb(175, 180, 195);
    visuals.widgets.inactive.bg_fill = SURFACE_RAISED;
    visuals.widgets.inactive.fg_stroke.color = egui::Color32::from_rgb(210, 214, 225);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(48, 54, 68);
    visuals.widgets.active.bg_fill = ACCENT_DIM;
    visuals.widgets.open.bg_fill = egui::Color32::from_rgb(42, 48, 60);

    visuals.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(72, 132, 220, 55);
    visuals.selection.stroke.color = ACCENT;
    visuals.hyperlink_color = ACCENT;

    visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(28, 31, 40);
    visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(38, 42, 54);

    style.visuals = visuals;
    ctx.set_style(style);
}

pub fn app_title() -> egui::RichText {
    egui::RichText::new("GLua Builder")
        .strong()
        .size(15.0)
        .color(egui::Color32::from_rgb(230, 233, 240))
}

pub fn section_title(text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .strong()
        .size(13.0)
        .color(egui::Color32::from_rgb(190, 196, 210))
}

pub fn hint(text: &str) -> egui::RichText {
    egui::RichText::new(text).small().weak()
}

pub fn sidebar_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(SURFACE)
        .stroke(egui::Stroke::new(1.0, BORDER))
        .inner_margin(egui::Margin::same(10.0))
        .rounding(egui::Rounding::same(8.0))
}

pub fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(SURFACE)
        .stroke(egui::Stroke::new(1.0, BORDER))
        .inner_margin(egui::Margin::same(12.0))
        .rounding(egui::Rounding::same(8.0))
}

pub fn status_bar_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(SURFACE_RAISED)
        .stroke(egui::Stroke::new(1.0, BORDER))
        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
        .rounding(egui::Rounding::same(8.0))
}

pub fn sidebar_section(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(4.0);
    ui.label(section_title(title));
    ui.add_space(4.0);
    let inner_width = (ui.available_width() - 20.0).max(0.0);
    sidebar_frame().show(ui, |ui| {
        ui.set_min_width(inner_width);
        add_contents(ui);
    });
}

pub fn primary_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(text).fill(ACCENT)
}

pub fn tool_button(ui: &mut egui::Ui, text: &str, tooltip: &str) -> egui::Response {
    ui.add(egui::Button::new(text)).on_hover_text(tooltip)
}

pub fn vertical_separator(ui: &mut egui::Ui) {
    ui.add(egui::Separator::default().spacing(8.0).shrink(0.0));
}
