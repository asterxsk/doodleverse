use doodle_ai::{AtlasManifest, MascotMood, frame_for_mood, parse_manifest};
use rune::Pixels;

/// Dibby's atlas split into one `Pixels` per frame, so a mood change only picks an index.
pub struct MascotAssets {
    frames: Vec<Pixels>,
    manifest: AtlasManifest,
}

impl MascotAssets {
    pub fn load() -> Option<Self> {
        let bytes = include_bytes!("../../../assets/mascot/dibby-atlas.png");
        let image = image::load_from_memory(bytes).ok()?.to_rgba8();
        let manifest = parse_manifest(include_str!("../../../assets/mascot/dibby-motion.json"))
            .map_err(|error| log::warn!("Dibby manifest rejected: {error}"))
            .ok()?;
        let atlas = &manifest.atlas;
        let mut frames = Vec::with_capacity(atlas.frame_count as usize);
        for index in 0..atlas.frame_count {
            let column = index % atlas.columns.max(1);
            let row = index / atlas.columns.max(1);
            let x = column * atlas.frame_width;
            let y = row * atlas.frame_height;
            if x + atlas.frame_width > image.width() || y + atlas.frame_height > image.height() {
                return None;
            }
            let crop =
                image::imageops::crop_imm(&image, x, y, atlas.frame_width, atlas.frame_height);
            let frame = Pixels::new();
            frame.set(
                atlas.frame_width,
                atlas.frame_height,
                crop.to_image().into_raw(),
            );
            frames.push(frame);
        }
        Some(Self { frames, manifest })
    }

    /// The frame to show for `mood` at `elapsed_ms` since that mood began.
    pub fn frame(&self, mood: MascotMood, elapsed_ms: u64) -> Option<&Pixels> {
        let index = frame_for_mood(mood, elapsed_ms, &self.manifest)?;
        self.frames.get(index as usize)
    }
}
