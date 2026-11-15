use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use doodle_ai::MockProvider;
use doodle_canvas::{BrushSettings, Camera, RawSample, StrokeBuilder};
use doodle_document::{BoardDocument, BoardProjection};
use doodle_render::{GridKind, GridSettings};
use doodle_types::{
    MAX_IMPORT_BYTES, ObjectId, ObjectKind, Point, Pose, Rect, StatementObject, StatementStyle,
    StrokeGeometry, StrokeStyle,
};
use rune::{
    AppHandle, Drag, FileDialogOptions, Key, KeyEvent, MagnifyEvent, Mods, Phase, Tx, WheelEvent,
};

pub use crate::cards::{CardUi, PinPreview};
pub use crate::placement::{
    board_is_empty, next_order_key, placement_is_valid, placement_rect, placement_translation,
    statement_size,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Draw,
    Select,
    Hand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    Info,
    Error,
}

pub struct Notice {
    pub text: String,
    pub kind: NoticeKind,
    pub until: f64,
}

pub struct StrokeGesture {
    pub builder: StrokeBuilder,
    pub style: StrokeStyle,
}

pub struct SelectDrag {
    pub start: [f32; 2],
    pub last: [f32; 2],
    pub moved: bool,
    pub top_ids: Vec<ObjectId>,
}

pub struct Marquee {
    pub start: [f32; 2],
    pub current: [f32; 2],
}

pub struct PendingImport {
    pub payload: doodle_files::ImportPayload,
    pub cursor: Point,
    pub valid: bool,
}

/// Every continuous value the settings panel edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    BrushSize,
    Pressure,
    Smoothing,
    TaperStart,
    TaperEnd,
    GridSpacing,
    GridOpacity,
    GridMarkSize,
    GridLineWidth,
    CustomBoardRed,
    CustomBoardGreen,
    CustomBoardBlue,
    CustomGridRed,
    CustomGridGreen,
    CustomGridBlue,
}

pub struct DoodleApp {
    pub doc: BoardDocument,
    pub projection: BoardProjection,
    pub projection_revision: u64,
    pub camera: Camera,
    pub selection: BTreeSet<ObjectId>,
    pub tool: Tool,
    pub brush: BrushSettings,
    pub color: [u8; 4],
    pub grid: GridSettings,
    pub light_theme: bool,
    pub reduced_motion: bool,
    pub stroke: Option<StrokeGesture>,
    pub drag: Option<SelectDrag>,
    pub marquee: Option<Marquee>,
    pub pending_import: Option<PendingImport>,
    pub cards: Vec<CardUi>,
    pub next_card_id: u64,
    pub provider: MockProvider,
    pub mascot: Option<crate::assets::MascotAssets>,
    pub saved_revision: Option<u64>,
    pub notice: Option<Notice>,
    pub show_settings: bool,
    pub show_help: bool,
    pub show_close_confirm: bool,
    pub zen: bool,
    pub closing: bool,
    pub reopen_pending: bool,
    pub menu_open: bool,
    pub pin_preview: Option<PinPreview>,
    pub canvas_layout: [f32; 4],
    pub window_size: [f32; 2],
    pub last_pointer_world: Point,
    pub hint_visible: bool,
    pub panning: bool,
    pub pan_last: [f32; 2],
    pub last_export_path: Option<PathBuf>,
    pub export_after_close: bool,
    pub handle: Option<AppHandle<DoodleApp>>,
    pub clock: f64,
    pub mods: Mods,
    pub scratch: rune::TextEdit,
}

impl DoodleApp {
    pub fn new() -> Self {
        let doc = BoardDocument::new();
        let projection = doc.project().unwrap_or_else(|_| BoardProjection {
            objects: Vec::new(),
            revision: doc.revision(),
            total_stroke_points: 0,
            index: std::collections::HashMap::new(),
        });
        Self {
            saved_revision: Some(doc.revision()),
            projection_revision: doc.revision(),
            projection,
            doc,
            camera: Camera::default(),
            selection: BTreeSet::new(),
            tool: Tool::Draw,
            brush: BrushSettings::default(),
            color: [32, 30, 35, 255],
            grid: GridSettings::default(),
            light_theme: true,
            reduced_motion: false,
            stroke: None,
            drag: None,
            marquee: None,
            pending_import: None,
            cards: Vec::new(),
            next_card_id: 1,
            provider: MockProvider::default(),
            mascot: crate::assets::MascotAssets::load(),
            notice: None,
            show_settings: false,
            show_help: false,
            show_close_confirm: false,
            zen: false,
            closing: false,
            reopen_pending: false,
            menu_open: false,
            pin_preview: None,
            canvas_layout: [0.0, 0.0, 960.0, 640.0],
            window_size: [960.0, 640.0],
            last_pointer_world: Point::default(),
            hint_visible: true,
            panning: false,
            pan_last: [0.0, 0.0],
            last_export_path: None,
            export_after_close: false,
            handle: None,
            clock: 0.0,
            mods: Mods::default(),
            scratch: rune::TextEdit::new(""),
        }
    }

    pub fn viewport(&self) -> Rect {
        Rect::new(
            0.0,
            0.0,
            f64::from(self.canvas_layout[2].max(1.0)),
            f64::from(self.canvas_layout[3].max(1.0)),
        )
    }

    pub fn zoom(&self) -> f64 {
        self.camera.zoom.max(doodle_types::MIN_ZOOM)
    }

    pub fn refresh(&mut self) -> bool {
        if self.projection_revision == self.doc.revision() {
            return false;
        }
        self.projection = self.doc.project().unwrap_or_else(|_| BoardProjection {
            objects: Vec::new(),
            revision: self.doc.revision(),
            total_stroke_points: 0,
            index: std::collections::HashMap::new(),
        });
        self.projection_revision = self.doc.revision();
        self.selection.retain(|id| {
            self.projection
                .get(*id)
                .is_some_and(doodle_document::ProjectedObject::is_top_level)
        });
        self.hint_visible = self.projection.objects.is_empty();
        true
    }

    pub fn dirty(&self) -> bool {
        self.saved_revision != Some(self.doc.revision())
    }

    pub fn mark_saved(&mut self) {
        self.saved_revision = Some(self.doc.revision());
    }

    pub fn set_notice(&mut self, kind: NoticeKind, text: impl Into<String>) {
        self.notice = Some(Notice {
            text: text.into(),
            kind,
            until: self.clock + 7.0,
        });
    }

    pub fn notice_text(&self) -> Option<(&str, NoticeKind)> {
        let notice = self.notice.as_ref()?;
        if self.clock > notice.until {
            return None;
        }
        Some((notice.text.as_str(), notice.kind))
    }

    pub fn selected_top_ids(&self) -> Vec<ObjectId> {
        self.selection
            .iter()
            .filter(|id| {
                self.projection
                    .get(**id)
                    .is_some_and(doodle_document::ProjectedObject::is_top_level)
            })
            .copied()
            .collect()
    }

    pub fn next_order(&self) -> i64 {
        next_order_key(&self.projection)
    }

    pub fn create_stroke(&mut self, pose: Pose, style: StrokeStyle, geometry: StrokeGeometry) {
        match self.doc.create_stroke(pose, style, geometry) {
            Ok(_) => {
                self.hint_visible = false;
                self.refresh();
            }
            Err(error) => self.set_notice(NoticeKind::Error, format!("Stroke rejected: {error}")),
        }
    }

    pub fn move_selection(&mut self, dx: f64, dy: f64) {
        let ids = self.selected_top_ids();
        if ids.is_empty() {
            return;
        }
        match self.doc.move_objects(&ids, dx, dy) {
            Ok(_) => {
                self.refresh();
            }
            Err(error) => self.set_notice(NoticeKind::Error, format!("Move failed: {error}")),
        }
    }

    pub fn delete_selection(&mut self) {
        for card in &mut self.cards {
            card.card.cancel();
        }
        let ids = self.selected_top_ids();
        if ids.is_empty() {
            return;
        }
        match self.doc.delete_objects(&ids) {
            Ok(_) => {
                self.selection.clear();
                self.refresh();
            }
            Err(error) => self.set_notice(NoticeKind::Error, format!("Delete failed: {error}")),
        }
    }

    pub fn group_selection(&mut self) {
        let ids = self.selected_top_ids();
        if ids.len() < 2 {
            self.set_notice(NoticeKind::Info, "Select at least two objects to group");
            return;
        }
        match self.doc.group_selection(&ids) {
            Ok(outcome) => {
                self.selection = outcome.created.into_iter().collect();
                self.refresh();
            }
            Err(error) => self.set_notice(NoticeKind::Error, format!("Group failed: {error}")),
        }
    }

    pub fn ungroup_selection(&mut self) {
        let ids = self.selected_top_ids();
        if ids.is_empty() {
            self.set_notice(NoticeKind::Info, "Select a group to ungroup");
            return;
        }
        match self.doc.ungroup(&ids) {
            Ok(_) => {
                self.selection.clear();
                self.refresh();
            }
            Err(error) => self.set_notice(NoticeKind::Error, format!("Ungroup failed: {error}")),
        }
    }

    pub fn undo(&mut self) {
        match self.doc.undo() {
            Ok(_) => {
                self.refresh();
            }
            Err(_) => self.set_notice(NoticeKind::Info, "Nothing to undo"),
        }
    }

    pub fn redo(&mut self) {
        match self.doc.redo() {
            Ok(_) => {
                self.refresh();
            }
            Err(_) => self.set_notice(NoticeKind::Info, "Nothing to redo"),
        }
    }

    pub fn select_all(&mut self) {
        self.selection = self
            .projection
            .top_level()
            .map(|object| object.object.id)
            .collect();
    }

    pub fn fit_to_content(&mut self) {
        let Some(bounds) = self.projection.content_bounds() else {
            self.camera.reset();
            return;
        };
        self.camera.fit(bounds, self.viewport(), 72.0);
    }

    pub fn zoom_from_center(&mut self, factor: f64) {
        let viewport = self.viewport();
        self.camera.zoom_at(
            Point::new(viewport.width() * 0.5, viewport.height() * 0.5),
            viewport,
            factor,
        );
    }

    pub fn create_ai_card(&mut self) {
        let id = self.next_card_id;
        self.next_card_id += 1;
        let stagger = (self.cards.len() % 4) as f64 * 28.0;
        let pos = Point::new(
            self.camera.center.x + 120.0 + stagger,
            self.camera.center.y - 160.0 + stagger,
        );
        self.cards.push(CardUi::new(id, pos));
    }

    pub fn cancel_gestures(&mut self) {
        self.stroke = None;
        self.drag = None;
        self.marquee = None;
        self.panning = false;
        self.pin_preview = None;
        self.cancel_pending_import();
    }

    pub fn apply_slider(&mut self, target: Target, fraction: f64) {
        let f = fraction.clamp(0.0, 1.0);
        let lerp = |min: f64, max: f64| min + (max - min) * f;
        match target {
            Target::BrushSize => self.brush.base_width = lerp(1.0, 32.0) as f32,
            Target::Pressure => self.brush.pressure_sensitivity = f as f32,
            Target::Smoothing => self.brush.smoothing = f as f32,
            Target::TaperStart => self.brush.taper_start = (f * 0.45) as f32,
            Target::TaperEnd => self.brush.taper_end = (f * 0.45) as f32,
            Target::GridSpacing => self.grid.spacing = lerp(8.0, 200.0),
            Target::GridOpacity => self.grid.opacity = f as f32,
            Target::GridMarkSize => self.grid.mark_size = lerp(0.25, 8.0) as f32,
            Target::GridLineWidth => self.grid.line_width = lerp(0.25, 8.0) as f32,
            Target::CustomBoardRed => self.grid.background_rgba[0] = (f * 255.0) as u8,
            Target::CustomBoardGreen => self.grid.background_rgba[1] = (f * 255.0) as u8,
            Target::CustomBoardBlue => self.grid.background_rgba[2] = (f * 255.0) as u8,
            Target::CustomGridRed => self.grid.line_rgba[0] = (f * 255.0) as u8,
            Target::CustomGridGreen => self.grid.line_rgba[1] = (f * 255.0) as u8,
            Target::CustomGridBlue => self.grid.line_rgba[2] = (f * 255.0) as u8,
        }
    }

    pub fn slider_fraction(&self, target: Target) -> f32 {
        let inv = |value: f64, min: f64, max: f64| ((value - min) / (max - min)) as f32;
        match target {
            Target::BrushSize => inv(f64::from(self.brush.base_width), 1.0, 32.0),
            Target::Pressure => self.brush.pressure_sensitivity,
            Target::Smoothing => self.brush.smoothing,
            Target::TaperStart => self.brush.taper_start / 0.45,
            Target::TaperEnd => self.brush.taper_end / 0.45,
            Target::GridSpacing => inv(self.grid.spacing, 8.0, 200.0),
            Target::GridOpacity => self.grid.opacity,
            Target::GridMarkSize => inv(f64::from(self.grid.mark_size), 0.25, 8.0),
            Target::GridLineWidth => inv(f64::from(self.grid.line_width), 0.25, 8.0),
            Target::CustomBoardRed => f32::from(self.grid.background_rgba[0]) / 255.0,
            Target::CustomBoardGreen => f32::from(self.grid.background_rgba[1]) / 255.0,
            Target::CustomBoardBlue => f32::from(self.grid.background_rgba[2]) / 255.0,
            Target::CustomGridRed => f32::from(self.grid.line_rgba[0]) / 255.0,
            Target::CustomGridGreen => f32::from(self.grid.line_rgba[1]) / 255.0,
            Target::CustomGridBlue => f32::from(self.grid.line_rgba[2]) / 255.0,
        }
        .clamp(0.0, 1.0)
    }

    pub fn apply_board_preset(&mut self, background: [u8; 4], lines: [u8; 4], light: bool) {
        self.grid.background_rgba = background;
        self.grid.line_rgba = lines;
        self.light_theme = light;
        self.adapt_ink();
    }

    /// Keeps the default ink readable when the board flips between paper and dark. A stroke color
    /// the user picked is never changed; only the two default inks swap.
    fn adapt_ink(&mut self) {
        const DARK_INK: [u8; 4] = [32, 30, 35, 255];
        const LIGHT_INK: [u8; 4] = [238, 234, 227, 255];
        if self.light_theme && self.color == LIGHT_INK {
            self.color = DARK_INK;
        } else if !self.light_theme && self.color == DARK_INK {
            self.color = LIGHT_INK;
        }
    }

    /// Zen mode hides every island, so anything opened from one closes with it.
    pub fn toggle_zen(&mut self) {
        self.zen = !self.zen;
        if self.zen {
            self.show_settings = false;
            self.show_help = false;
            self.menu_open = false;
        }
    }

    pub fn toggle_theme(&mut self) {
        self.light_theme = !self.light_theme;
        if self.light_theme {
            self.grid.background_rgba = [250, 248, 243, 255];
            self.grid.line_rgba = [174, 168, 158, 255];
        } else {
            self.grid.background_rgba = [24, 23, 28, 255];
            self.grid.line_rgba = [92, 89, 101, 255];
        }
        self.adapt_ink();
    }

    /// Opens the native picker through Rune, so headless scripts answer it like a user would.
    pub fn open_file(&mut self, tx: &mut Tx) {
        tx.open_file(
            FileDialogOptions {
                title: "Open board".into(),
                ..Default::default()
            }
            .filter("Doodleverse board", &["doodleverse"]),
            |state: &mut DoodleApp, result, _| match result {
                Ok(Some(path)) => state.open_path(&path),
                Ok(None) => {}
                Err(error) => state.set_notice(NoticeKind::Error, format!("Open failed: {error}")),
            },
        );
    }

    fn open_path(&mut self, path: &Path) {
        match read_board_file(path) {
            Ok(payload) => self.accept_import(payload),
            Err(message) => self.set_notice(NoticeKind::Error, message),
        }
    }

    fn accept_import(&mut self, payload: doodle_files::ImportPayload) {
        let count = payload.object_count;
        if board_is_empty(&self.projection) {
            let wrapper = ObjectId::new();
            let order = self.next_order();
            let objects = payload.into_commit(wrapper, 0.0, 0.0, order);
            match self.doc.commit_import(objects) {
                Ok(_) => {
                    self.selection.clear();
                    self.refresh();
                    self.hint_visible = false;
                    self.set_notice(NoticeKind::Info, format!("Imported {count} objects"));
                }
                Err(error) => {
                    self.set_notice(NoticeKind::Error, format!("Import failed: {error}"));
                }
            }
        } else {
            let cursor = self.last_pointer_world;
            let valid = placement_is_valid(
                placement_rect(&payload, placement_translation(&payload, cursor)),
                &self.projection.occupied_regions(),
            );
            self.pending_import = Some(PendingImport {
                payload,
                cursor,
                valid,
            });
            self.set_notice(
                NoticeKind::Info,
                "Move to place the import, click to drop, Esc to cancel",
            );
        }
    }

    pub fn update_pending_import(&mut self, cursor: Point) {
        let Some(pending) = self.pending_import.as_mut() else {
            return;
        };
        pending.cursor = cursor;
        let translation = placement_translation(&pending.payload, cursor);
        let candidate = placement_rect(&pending.payload, translation);
        pending.valid = placement_is_valid(candidate, &self.projection.occupied_regions());
    }

    pub fn commit_pending_import(&mut self) {
        let Some(pending) = self.pending_import.take() else {
            return;
        };
        let translation = placement_translation(&pending.payload, pending.cursor);
        let candidate = placement_rect(&pending.payload, translation);
        if !placement_is_valid(candidate, &self.projection.occupied_regions()) {
            self.pending_import = Some(pending);
            self.set_notice(NoticeKind::Error, "That area is occupied — move the ghost");
            return;
        }
        let wrapper = ObjectId::new();
        let order = self.next_order();
        let count = pending.payload.object_count;
        let objects = pending
            .payload
            .into_commit(wrapper, translation.0, translation.1, order);
        match self.doc.commit_import(objects) {
            Ok(_) => {
                self.selection.clear();
                self.refresh();
                self.hint_visible = false;
                self.set_notice(NoticeKind::Info, format!("Imported {count} objects"));
            }
            Err(error) => self.set_notice(NoticeKind::Error, format!("Import failed: {error}")),
        }
    }

    pub fn cancel_pending_import(&mut self) {
        if self.pending_import.take().is_some() {
            self.set_notice(NoticeKind::Info, "Import cancelled");
        }
    }

    pub fn export_file(&mut self, tx: &mut Tx) {
        if self.projection.content_bounds().is_none() {
            self.set_notice(NoticeKind::Info, "Board is empty — nothing to export");
            return;
        }
        let snapshot = self.doc.snapshot_visible();
        let bytes = match doodle_files::encode(&snapshot) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.set_notice(NoticeKind::Error, format!("Export failed: {error}"));
                return;
            }
        };
        let suggested = self
            .last_export_path
            .as_ref()
            .and_then(|path| path.file_stem())
            .and_then(|stem| stem.to_str())
            .unwrap_or("doodleverse-board")
            .to_owned();
        let file_name = format!("{suggested}.doodleverse");
        tx.save_file(
            FileDialogOptions {
                title: "Export board".into(),
                file_name: Some(file_name),
                ..Default::default()
            }
            .filter("Doodleverse board", &["doodleverse"]),
            move |state: &mut DoodleApp, result, tx| match result {
                Ok(Some(path)) => {
                    let written = state.write_export(&path, &bytes);
                    if written && state.export_after_close {
                        state.closing = true;
                        tx.quit();
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    state.set_notice(NoticeKind::Error, format!("Export failed: {error}"));
                }
            },
        );
    }

    fn write_export(&mut self, path: &Path, bytes: &[u8]) -> bool {
        let path = doodle_platform::ensure_board_extension(path.to_path_buf());
        match doodle_files::write_atomic(&path, bytes) {
            Ok(()) => {
                self.last_export_path = Some(path.clone());
                self.mark_saved();
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("board");
                self.set_notice(NoticeKind::Info, format!("Exported {name}"));
                true
            }
            Err(error) => {
                self.set_notice(NoticeKind::Error, format!("Export failed: {error}"));
                false
            }
        }
    }

    pub fn confirm_pin(&mut self, card_id: u64) {
        let zoom = self.zoom();
        let Some(card) = self
            .cards
            .iter_mut()
            .find(|card| card.card.local_id == card_id)
        else {
            self.pin_preview = None;
            return;
        };
        let Some(text) = card.card.pin_text().map(str::to_owned) else {
            self.pin_preview = None;
            return;
        };
        let (width, height) = statement_size(&text);
        let statement = StatementObject {
            text,
            width,
            height,
            style: StatementStyle::default(),
        };
        let pose = Pose::new(card.card.world_pos.x + 384.0 / zoom, card.card.world_pos.y);
        match self.doc.create_statement(pose, statement) {
            Ok(_) => {
                card.pinned = true;
                self.pin_preview = None;
                self.refresh();
                self.set_notice(NoticeKind::Info, "Answer pinned to the board");
            }
            Err(error) => {
                self.pin_preview = None;
                self.set_notice(NoticeKind::Error, format!("Pin failed: {error}"));
            }
        }
    }

    pub fn poll_cards(&mut self) -> bool {
        let mut changed = false;
        let now = self.clock;
        let reduced_motion = self.reduced_motion;
        for card in &mut self.cards {
            card.update_mood(now, reduced_motion);
            if card.sync_question() {
                changed = true;
            }
            if card.card.poll() {
                card.mark_changed(now);
                changed = true;
            }
            if !card.card.is_busy() {
                card.busy_started = None;
            }
        }
        changed
    }

    pub fn any_card_busy(&self) -> bool {
        self.cards.iter().any(|card| card.card.is_busy())
    }

    pub fn hit_top(&self, world: Point) -> Option<ObjectId> {
        let tolerance = 5.0 / self.zoom();
        for projected in self.projection.objects.iter().rev() {
            match &projected.object.kind {
                ObjectKind::Stroke(stroke) => {
                    if let Some(clearance) = doodle_canvas::geometry::stroke_hit_clearance(
                        &stroke.geometry,
                        projected.object.pose,
                        world,
                    ) && clearance <= tolerance
                    {
                        return Some(projected.top_id);
                    }
                }
                ObjectKind::Statement(_) => {
                    if projected
                        .bounds
                        .is_some_and(|bounds| bounds.contains(world))
                    {
                        return Some(projected.top_id);
                    }
                }
                ObjectKind::Group => {}
            }
        }
        None
    }

    pub fn drag_offsets(&self) -> std::collections::HashMap<ObjectId, (f64, f64)> {
        let mut offsets = std::collections::HashMap::new();
        if let Some(drag) = &self.drag
            && drag.moved
        {
            let dx = f64::from(drag.last[0] - drag.start[0]) / self.zoom();
            let dy = f64::from(drag.last[1] - drag.start[1]) / self.zoom();
            for id in &drag.top_ids {
                offsets.insert(*id, (dx, dy));
            }
        }
        offsets
    }

    // ---- input ---------------------------------------------------------------------------

    pub fn pointer_moved(&mut self, x: f32, y: f32) {
        let world = self
            .camera
            .screen_to_world(Point::new(f64::from(x), f64::from(y)), self.viewport());
        self.last_pointer_world = world;
        self.update_pending_import(world);
    }

    /// The settings panel is a fixed-height overlay, so its wheel scrolling is clamped here
    /// instead of panned into the board beneath it.
    pub fn settings_rect(&self) -> Option<[f32; 4]> {
        self.show_settings.then(|| {
            let height = (self.window_size[1] - 150.0).clamp(280.0, 680.0);
            [140.0, 84.0, 320.0, height]
        })
    }

    pub fn wheel(&mut self, ev: &WheelEvent) -> bool {
        if self
            .settings_rect()
            .is_some_and(|[x, y, w, h]| ev.x >= x && ev.x <= x + w && ev.y >= y && ev.y <= y + h)
        {
            // The panel's own scroll view takes the wheel; the board beneath must not pan.
            return false;
        }
        let viewport = self.viewport();
        let zooming = ev.mods.cmd || ev.mods.ctrl;
        if zooming {
            if ev.dy.abs() > 0.05 {
                let factor = (-f64::from(ev.dy) * 0.0022).exp();
                self.camera.zoom_at(
                    Point::new(f64::from(ev.x), f64::from(ev.y)),
                    viewport,
                    factor,
                );
            }
        } else {
            self.camera
                .pan_by_screen_delta(f64::from(ev.dx), f64::from(ev.dy));
        }
        true
    }

    pub fn magnify(&mut self, ev: &MagnifyEvent) -> bool {
        let viewport = self.viewport();
        let factor = 1.0 + f64::from(ev.delta);
        self.camera.zoom_at(
            Point::new(f64::from(ev.x), f64::from(ev.y)),
            viewport,
            factor,
        );
        true
    }

    pub fn canvas_drag(&mut self, drag: &Drag) {
        let viewport = self.viewport();
        let world = self
            .camera
            .screen_to_world(Point::new(f64::from(drag.x), f64::from(drag.y)), viewport);
        self.last_pointer_world = world;
        match drag.phase {
            Phase::Down => self.begin_press(drag, world),
            Phase::Move => self.continue_press(drag, world),
            Phase::Up => self.finish_press(drag, world),
        }
    }

    fn begin_press(&mut self, drag: &Drag, world: Point) {
        if self.pending_import.is_some() {
            self.commit_pending_import();
            return;
        }
        let shift = self.mods.shift;
        match self.tool {
            Tool::Hand => {
                self.panning = true;
                self.pan_last = [drag.x, drag.y];
            }
            Tool::Draw => {
                let start = RawSample::new(world, doodle_platform::pen_pressure(), self.clock);
                let builder = StrokeBuilder::new(self.brush, start, 0.7 / self.zoom());
                self.stroke = Some(StrokeGesture {
                    builder,
                    style: self.brush.style(self.color),
                });
                self.hint_visible = false;
                self.drag = None;
                self.marquee = None;
            }
            Tool::Select => {
                if let Some(top) = self.hit_top(world) {
                    if shift {
                        if !self.selection.insert(top) {
                            self.selection.remove(&top);
                        }
                    } else if !self.selection.contains(&top) {
                        self.selection.clear();
                        self.selection.insert(top);
                    }
                    self.drag = Some(SelectDrag {
                        start: [drag.x, drag.y],
                        last: [drag.x, drag.y],
                        moved: false,
                        top_ids: self.selected_top_ids(),
                    });
                    self.marquee = None;
                } else {
                    if !shift {
                        self.selection.clear();
                    }
                    self.marquee = Some(Marquee {
                        start: [drag.x, drag.y],
                        current: [drag.x, drag.y],
                    });
                    self.drag = None;
                }
            }
        }
    }

    fn continue_press(&mut self, drag: &Drag, world: Point) {
        if self.panning {
            let dx = f64::from(drag.x - self.pan_last[0]);
            let dy = f64::from(drag.y - self.pan_last[1]);
            self.pan_last = [drag.x, drag.y];
            self.camera.pan_by_screen_delta(dx, dy);
            return;
        }
        if let Some(gesture) = self.stroke.as_mut() {
            gesture.builder.push(RawSample::new(
                world,
                doodle_platform::pen_pressure(),
                self.clock,
            ));
            return;
        }
        if let Some(drag_state) = self.drag.as_mut() {
            drag_state.last = [drag.x, drag.y];
            let dx = drag_state.last[0] - drag_state.start[0];
            let dy = drag_state.last[1] - drag_state.start[1];
            if dx.hypot(dy) > 2.5 {
                drag_state.moved = true;
            }
            return;
        }
        if let Some(marquee) = self.marquee.as_mut() {
            marquee.current = [drag.x, drag.y];
        }
    }

    fn finish_press(&mut self, drag: &Drag, world: Point) {
        if self.panning {
            self.panning = false;
            return;
        }
        if let Some(mut gesture) = self.stroke.take() {
            gesture.builder.push(RawSample::new(
                world,
                doodle_platform::pen_pressure(),
                self.clock,
            ));
            let (pose, geometry) = gesture.builder.finish();
            if geometry.is_valid() {
                self.create_stroke(pose, gesture.style, geometry);
            }
            return;
        }
        if let Some(state) = self.drag.take() {
            if state.moved && !drag.cancelled() {
                let dx = f64::from(state.last[0] - state.start[0]) / self.zoom();
                let dy = f64::from(state.last[1] - state.start[1]) / self.zoom();
                if dx.abs() > 0.01 || dy.abs() > 0.01 {
                    self.move_selection(dx, dy);
                }
            }
            return;
        }
        if let Some(marquee) = self.marquee.take()
            && !drag.cancelled()
        {
            let viewport = self.viewport();
            let start = self.camera.screen_to_world(
                Point::new(f64::from(marquee.start[0]), f64::from(marquee.start[1])),
                viewport,
            );
            let end = self.camera.screen_to_world(
                Point::new(f64::from(marquee.current[0]), f64::from(marquee.current[1])),
                viewport,
            );
            let selection_rect = Rect::new(
                start.x.min(end.x),
                start.y.min(end.y),
                start.x.max(end.x),
                start.y.max(end.y),
            );
            for object in self.projection.top_level() {
                if object
                    .bounds
                    .is_some_and(|bounds| bounds.intersects_rect(selection_rect))
                {
                    self.selection.insert(object.object.id);
                }
            }
        }
    }

    // ---- cards ---------------------------------------------------------------------------

    pub fn preview_pin_answer(&mut self, card_id: u64) {
        let text = self
            .cards
            .iter()
            .find(|card| card.card.local_id == card_id)
            .and_then(|card| card.card.pin_text())
            .map(str::to_owned);
        if let Some(text) = text {
            self.pin_preview = Some(PinPreview { card_id, text });
        }
    }

    pub fn card_submit(&mut self, card_id: u64) {
        let Some(card) = self.cards.iter_mut().find(|c| c.card.local_id == card_id) else {
            return;
        };
        card.sync_question();
        if card.card.question.trim().is_empty() {
            return;
        }
        card.busy_started = Some(self.clock);
        match card.card.submit(&self.provider, None) {
            Ok(()) => card.pinned = false,
            Err(error) => {
                card.busy_started = None;
                self.set_notice(
                    NoticeKind::Error,
                    format!("Could not start generation: {error}"),
                );
            }
        }
    }

    pub fn card_cancel(&mut self, card_id: u64) {
        if let Some(card) = self.cards.iter_mut().find(|c| c.card.local_id == card_id) {
            card.card.cancel();
            card.busy_started = None;
        }
    }

    pub fn card_select(&mut self, card_id: u64, index: usize) {
        if let Some(card) = self.cards.iter_mut().find(|c| c.card.local_id == card_id) {
            card.card.select(index);
            card.pinned = false;
        }
    }

    pub fn close_card(&mut self, card_id: u64) {
        self.cards.retain(|card| card.card.local_id != card_id);
        self.set_notice(NoticeKind::Info, "Closed the local AI card");
    }

    pub fn drag_card(&mut self, card_id: u64, drag: &Drag) {
        let zoom = self.zoom();
        if let Some(card) = self.cards.iter_mut().find(|c| c.card.local_id == card_id) {
            match drag.phase {
                Phase::Down => card.last_drag = 0.0,
                Phase::Move => {
                    let travel = drag.dx.hypot(drag.dy);
                    let step = travel - card.last_drag;
                    card.last_drag = travel;
                    if step != 0.0 {
                        let length = travel.max(1.0);
                        card.card.world_pos.x +=
                            f64::from(drag.dx / length) * f64::from(step) / zoom;
                        card.card.world_pos.y +=
                            f64::from(drag.dy / length) * f64::from(step) / zoom;
                        card.mark_changed(self.clock);
                    }
                }
                Phase::Up => card.last_drag = 0.0,
            }
        }
    }

    pub fn reload_handle(&mut self) {
        if self.reopen_pending {
            self.reopen_pending = false;
            if let Some(handle) = &self.handle {
                handle.show();
            }
        }
    }
}

pub fn on_modifiers(state: &mut DoodleApp, mods: Mods, _tx: &mut Tx) {
    state.mods = mods;
}

pub fn poll(state: &mut DoodleApp, t: f64, tx: &mut Tx) -> bool {
    state.clock = t;
    state.reload_handle();
    let refreshed = state.refresh();
    let cards = state.poll_cards();
    let animating = state.any_card_busy()
        || state.stroke.is_some()
        || state.drag.is_some()
        || state.marquee.is_some()
        || state.panning;
    if animating {
        tx.wake_after(Duration::from_millis(33));
    }
    refreshed || cards || animating
}

pub fn key_event(state: &mut DoodleApp, ev: &KeyEvent, tx: &mut Tx) -> bool {
    let mods = ev.mods;
    if mods.cmd {
        if ev.is_command('z') {
            if mods.shift {
                state.redo();
            } else {
                state.undo();
            }
            return true;
        }
        if ev.is_command('y') {
            state.redo();
            return true;
        }
        if ev.is_command('a') {
            state.select_all();
            return true;
        }
        if ev.is_command('g') {
            if mods.shift {
                state.ungroup_selection();
            } else {
                state.group_selection();
            }
            return true;
        }
        if ev.is_command('o') {
            state.open_file(tx);
            return true;
        }
        if ev.is_command('s') {
            state.export_file(tx);
            return true;
        }
        return false;
    }
    match &ev.key {
        Key::Text(text) => {
            if mods.alt && text.eq_ignore_ascii_case("z") {
                state.toggle_zen();
                return true;
            }
            if mods.alt || mods.ctrl {
                return false;
            }
            match text.as_str() {
                "b" => state.tool = Tool::Draw,
                "v" => {
                    state.tool = Tool::Select;
                    state.stroke = None;
                }
                "h" => {
                    state.tool = Tool::Hand;
                    state.stroke = None;
                }
                "f" => state.fit_to_content(),
                "0" => state.camera.reset(),
                "?" => state.show_help = !state.show_help,
                "+" | "=" => state.zoom_from_center(1.2),
                "-" => state.zoom_from_center(1.0 / 1.2),
                _ => return false,
            }
            true
        }
        Key::Delete | Key::Backspace => {
            state.delete_selection();
            true
        }
        Key::Escape => {
            state.cancel_gestures();
            state.show_close_confirm = false;
            state.pin_preview = None;
            state.menu_open = false;
            true
        }
        Key::Function(1) => {
            state.show_help = !state.show_help;
            true
        }
        _ => {
            let _ = tx;
            false
        }
    }
}

pub fn on_close(state: &mut DoodleApp, tx: &mut Tx) {
    if state.closing || !state.dirty() {
        tx.quit();
        return;
    }
    for card in &mut state.cards {
        card.card.cancel();
    }
    state.cancel_gestures();
    state.show_close_confirm = true;
    state.reopen_pending = true;
}

fn read_board_file(path: &Path) -> Result<doodle_files::ImportPayload, String> {
    let metadata = std::fs::metadata(path).map_err(|error| format!("Cannot open file: {error}"))?;
    if metadata.len() > MAX_IMPORT_BYTES + 64 {
        return Err("File is too large to import".into());
    }
    let bytes = std::fs::read(path).map_err(|error| format!("Cannot read file: {error}"))?;
    doodle_files::decode_and_validate(&bytes)
        .map_err(|error| format!("Invalid board file: {error}"))
}

#[allow(dead_code)]
fn _kind_label(kind: GridKind) -> &'static str {
    match kind {
        GridKind::None => "None",
        GridKind::Dots => "Dots",
        GridKind::Pluses => "Pluses",
        GridKind::Lines => "Lines",
    }
}
