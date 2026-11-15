use doodle_ai::{AtlasManifest, MascotMood, frame_for_mood, frame_uv, parse_manifest};
use eframe::egui;

pub struct MascotAssets {
    pub texture: egui::TextureHandle,
    pub manifest: AtlasManifest,
}

impl MascotAssets {
    pub fn load(ctx: &egui::Context) -> Option<Self> {
        let bytes = include_bytes!("../../../../assets/mascot/dibby-atlas.png");
        let image = image::load_from_memory(bytes).ok()?.to_rgba8();
        let size = [image.width() as usize, image.height() as usize];
        let color = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
        let texture = ctx.load_texture("dibby-atlas", color, egui::TextureOptions::LINEAR);
        let manifest = parse_manifest(include_str!("../../../../assets/mascot/dibby-motion.json"))
            .map_err(|error| log::warn!("Dibby manifest rejected: {error}"))
            .ok()?;
        Some(Self { texture, manifest })
    }

    pub fn paint(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        mood: MascotMood,
        elapsed_ms: u64,
    ) {
        let Some(index) = frame_for_mood(mood, elapsed_ms, &self.manifest) else {
            return;
        };
        let Some(frame) = frame_uv(index, &self.manifest) else {
            return;
        };
        let uv = egui::Rect::from_min_max(
            egui::pos2(frame.uv[0], frame.uv[1]),
            egui::pos2(frame.uv[2], frame.uv[3]),
        );
        painter.image(self.texture.id(), rect, uv, egui::Color32::WHITE);
    }
}

/// The selected original icon, resized for the native window. Packaging-specific ICO/ICNS
/// exports remain a separate release task.
pub fn window_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../../../../assets/app-icon.png");
    let image = image::load_from_memory(bytes).ok()?;
    let resized = image
        .resize_exact(256, 256, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    Some(egui::IconData {
        rgba: resized.as_raw().clone(),
        width: 256,
        height: 256,
    })
}
