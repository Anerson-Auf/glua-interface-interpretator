use std::collections::HashMap;
use std::path::Path;

use eframe::egui;

pub struct ImageCache {
    textures: HashMap<String, egui::TextureHandle>,
}

impl Default for ImageCache {
    fn default() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }
}

impl ImageCache {
    pub fn texture_id(&mut self, ctx: &egui::Context, path: &str) -> Option<egui::TextureId> {
        if path.is_empty() {
            return None;
        }
        if !Path::new(path).exists() {
            return None;
        }
        if !self.textures.contains_key(path) {
            if let Some(color_image) = load_color_image(path) {
                let handle = ctx.load_texture(
                    format!("img_{path}"),
                    color_image,
                    egui::TextureOptions::LINEAR,
                );
                self.textures.insert(path.to_string(), handle);
            }
        }
        self.textures.get(path).map(|h| h.id())
    }

    pub fn invalidate(&mut self, path: &str) {
        self.textures.remove(path);
    }
}

fn load_color_image(path: &str) -> Option<egui::ColorImage> {
    let img = image::open(path).ok()?;
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(
        size,
        rgba.as_raw(),
    ))
}

pub fn is_image_file(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            matches!(
                e.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp"
            )
        })
        .unwrap_or(false)
}

pub fn suggest_material_path(path: &str) -> String {
    let name = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("image.png");
    format!("materials/{name}")
}

pub fn pick_image_file() -> Option<String> {
    rfd::FileDialog::new()
        .add_filter("Изображения", &["png", "jpg", "jpeg", "webp", "gif", "bmp"])
        .pick_file()
        .map(|p| p.display().to_string())
}
