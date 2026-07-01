mod codegen;
mod model;
mod ui;

use eframe::egui;

use model::Project;
use ui::EditorUi;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_title("GLua Interface Builder"),
        ..Default::default()
    };

    eframe::run_native(
        "GLua Interface Builder",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

struct App {
    project: Project,
    editor: EditorUi,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        ui::theme::apply(&cc.egui_ctx);
        Self {
            project: Project::default(),
            editor: EditorUi::default(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.editor.selection.is_empty() {
            self.editor.select_single(self.project.root_id);
        }
        self.editor.show(ctx, &mut self.project);
    }
}
