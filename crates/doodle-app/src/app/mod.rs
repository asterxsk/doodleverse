pub(crate) mod assets;
mod canvas_view;
mod cards;
mod chrome;
mod panels;
mod placement;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use doodle_ai::MockProvider;
use doodle_canvas::{BrushSettings, Camera, StrokeBuilder};
use doodle_document::{BoardDocument, BoardProjection};
use doodle_render::{GridSettings, SceneRenderer};
use doodle_types::{
    MAX_IMPORT_BYTES, ObjectId, Point, Pose, Rect, StatementObject, StatementStyle, StrokeGeometry,
    StrokeStyle,
};
use eframe::egui;

pub(crate) use cards::{CardUi, PinPreview};
pub(crate) use placement::{
    board_is_empty, next_order_key, placement_is_valid, placement_rect, placement_translation,
    statement_size,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tool {
    Draw,
    Select,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoticeKind {
    Info,
    Error,
}

pub(crate) struct Notice {
    pub text: String,
    pub kind: NoticeKind,
    pub until: Instant,
}

pub(crate) struct StrokeGesture {
    pub builder: StrokeBuilder,
    pub style: StrokeStyle,
}

pub(crate) struct SelectDrag {
    pub start_screen: egui::Pos2,
    pub last_screen: egui::Pos2,
    pub moved: bool,
    pub top_ids: Vec<ObjectId>,
}

pub(crate) struct Marquee {
    pub start_screen: egui::Pos2,
    pub current_screen: egui::Pos2,
}

pub(crate) struct PendingImport {
    pub payload: doodle_files::ImportPayload,
    pub cursor: Point,
    pub valid: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Palette {
    pub text: egui::Color32,
    pub muted: egui::Color32,
    pub accent: egui::Color32,
    pub card: egui::Color32,
    pub border: egui::Color32,
    pub danger: egui::Color32,
    pub selection: egui::Color32,
}

pub(crate) fn palette(light: bool) -> Palette {
    if light {
        Palette {
            text: egui::Color32::from_rgb(34, 32, 37),
            muted: egui::Color32::from_rgb(112, 107, 116),
            accent: egui::Color32::from_rgb(222, 108, 85),
            card: egui::Color32::from_rgb(255, 253, 248),
            border: egui::Color32::from_rgb(222, 215, 202),
            danger: egui::Color32::from_rgb(180, 56, 48),
            selection: egui::Color32::from_rgb(55, 145, 141),
        }
    } else {
        Palette {
            text: egui::Color32::from_rgb(238, 234, 227),
            muted: egui::Color32::from_rgb(164, 158, 172),
            accent: egui::Color32::from_rgb(255, 155, 130),
            card: egui::Color32::from_rgb(36, 35, 42),
            border: egui::Color32::from_rgb(66, 63, 74),
            danger: egui::Color32::from_rgb(255, 129, 118),
            selection: egui::Color32::from_rgb(94, 201, 192),
        }
    }
}

pub struct DoodleApp {
    pub(crate) doc: BoardDocument,
    pub(crate) projection: BoardProjection,
    pub(crate) projection_revision: u64,
    pub(crate) camera: Camera,
    pub(crate) renderer: SceneRenderer,
    pub(crate) selection: BTreeSet<ObjectId>,
    pub(crate) tool: Tool,
    pub(crate) brush: BrushSettings,
    pub(crate) color: [u8; 4],
    pub(crate) grid: GridSettings,
    pub(crate) light_theme: bool,
    pub(crate) reduced_motion: bool,
    pub(crate) stroke: Option<StrokeGesture>,
    pub(crate) drag: Option<SelectDrag>,
    pub(crate) marquee: Option<Marquee>,
    pub(crate) pending_import: Option<PendingImport>,
    pub(crate) cards: Vec<CardUi>,
    pub(crate) next_card_id: u64,
    pub(crate) provider: MockProvider,
    pub(crate) mascot: Option<assets::MascotAssets>,
    pub(crate) saved_revision: Option<u64>,
    pub(crate) notice: Option<Notice>,
    pub(crate) show_settings: bool,
    pub(crate) show_help: bool,
    pub(crate) show_close_confirm: bool,
    pub(crate) zen: bool,
    pub(crate) closing: bool,
    pub(crate) pin_preview: Option<PinPreview>,
    pub(crate) canvas_rect: egui::Rect,
    pub(crate) last_pointer_world: Point,
    pub(crate) hint_visible: bool,
    pub(crate) last_frame: Instant,
    pub(crate) space_down: bool,
    pub(crate) pan_active: bool,
    pub(crate) last_export_path: Option<PathBuf>,
}

impl DoodleApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut style = (*cc.egui_ctx.global_style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(9.0, 5.0);
        style.visuals = egui::Visuals::light();
        style.visuals.selection.bg_fill = egui::Color32::from_rgb(222, 108, 85);
        style.visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        cc.egui_ctx.set_global_style(style);

        {
            use raw_window_handle::HasWindowHandle;
            if let Ok(handle) = cc.window_handle()
                && doodle_platform::install_pen_input(handle)
            {
                log::info!("pen pressure input installed");
            }
        }

        let doc = BoardDocument::new();
        let projection = doc.project().unwrap_or_else(|_| BoardProjection {
            objects: Vec::new(),
            revision: doc.revision(),
            total_stroke_points: 0,
            index: std::collections::HashMap::new(),
        });
        let now = Instant::now();
        Self {
            saved_revision: Some(doc.revision()),
            projection_revision: doc.revision(),
            projection,
            doc,
            camera: Camera::default(),
            renderer: SceneRenderer::new(),
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
            mascot: assets::MascotAssets::load(&cc.egui_ctx),
            notice: None,
            show_settings: false,
            show_help: false,
            show_close_confirm: false,
            zen: false,
            closing: false,
            pin_preview: None,
            canvas_rect: egui::Rect::NOTHING,
            last_pointer_world: Point::default(),
            hint_visible: true,
            last_frame: now,
            space_down: false,
            pan_active: false,
            last_export_path: None,
        }
    }

    pub(crate) fn palette(&self) -> Palette {
        palette(self.light_theme)
    }

    pub(crate) fn refresh(&mut self) {
        if self.projection_revision == self.doc.revision() {
            return;
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
    }

    pub(crate) fn dirty(&self) -> bool {
        self.saved_revision != Some(self.doc.revision())
    }

    pub(crate) fn mark_saved(&mut self) {
        self.saved_revision = Some(self.doc.revision());
    }

    pub(crate) fn set_notice(&mut self, kind: NoticeKind, text: impl Into<String>) {
        self.notice = Some(Notice {
            text: text.into(),
            kind,
            until: Instant::now() + Duration::from_secs(7),
        });
    }

    pub(crate) fn notice_text(&self) -> Option<(&str, NoticeKind)> {
        let notice = self.notice.as_ref()?;
        if Instant::now() > notice.until {
            return None;
        }
        Some((notice.text.as_str(), notice.kind))
    }

    pub(crate) fn selected_top_ids(&self) -> Vec<ObjectId> {
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

    pub(crate) fn next_order(&self) -> i64 {
        next_order_key(&self.projection)
    }

    pub(crate) fn create_stroke(
        &mut self,
        pose: Pose,
        style: StrokeStyle,
        geometry: StrokeGeometry,
    ) {
        match self.doc.create_stroke(pose, style, geometry) {
            Ok(_) => {
                self.hint_visible = false;
                self.refresh();
            }
            Err(error) => self.set_notice(NoticeKind::Error, format!("Stroke rejected: {error}")),
        }
    }

    pub(crate) fn move_selection(&mut self, dx: f64, dy: f64) {
        let ids = self.selected_top_ids();
        if ids.is_empty() {
            return;
        }
        match self.doc.move_objects(&ids, dx, dy) {
            Ok(_) => self.refresh(),
            Err(error) => self.set_notice(NoticeKind::Error, format!("Move failed: {error}")),
        }
    }

    pub(crate) fn delete_selection(&mut self) {
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

    pub(crate) fn group_selection(&mut self) {
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

    pub(crate) fn ungroup_selection(&mut self) {
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

    pub(crate) fn undo(&mut self) {
        match self.doc.undo() {
            Ok(_) => self.refresh(),
            Err(_) => self.set_notice(NoticeKind::Info, "Nothing to undo"),
        }
    }

    pub(crate) fn redo(&mut self) {
        match self.doc.redo() {
            Ok(_) => self.refresh(),
            Err(_) => self.set_notice(NoticeKind::Info, "Nothing to redo"),
        }
    }

    pub(crate) fn select_all(&mut self) {
        self.selection = self
            .projection
            .top_level()
            .map(|object| object.object.id)
            .collect();
    }

    pub(crate) fn fit_to_content(&mut self) {
        let Some(bounds) = self.projection.content_bounds() else {
            self.camera.reset();
            return;
        };
        self.camera.fit(bounds, screen_rect(self.canvas_rect), 72.0);
    }

    pub(crate) fn create_ai_card(&mut self) {
        let id = self.next_card_id;
        self.next_card_id += 1;
        let stagger = (self.cards.len() % 4) as f64 * 28.0;
        let pos = Point::new(
            self.camera.center.x + 120.0 + stagger,
            self.camera.center.y - 160.0 + stagger,
        );
        self.cards.push(CardUi::new(id, pos));
    }

    pub(crate) fn open_file(&mut self) {
        let Some(path) = doodle_platform::open_board_dialog() else {
            return;
        };
        let result = read_board_file(&path);
        match result {
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

    pub(crate) fn update_pending_import(&mut self, cursor: Point) {
        let Some(pending) = self.pending_import.as_mut() else {
            return;
        };
        pending.cursor = cursor;
        let translation = placement_translation(&pending.payload, cursor);
        let candidate = placement_rect(&pending.payload, translation);
        pending.valid = placement_is_valid(candidate, &self.projection.occupied_regions());
    }

    pub(crate) fn commit_pending_import(&mut self) {
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

    pub(crate) fn cancel_pending_import(&mut self) {
        if self.pending_import.take().is_some() {
            self.set_notice(NoticeKind::Info, "Import cancelled");
        }
    }

    pub(crate) fn export_file(&mut self) {
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
            .unwrap_or("doodleverse-board");
        let Some(path) = doodle_platform::save_board_dialog(suggested) else {
            return;
        };
        let path = doodle_platform::ensure_board_extension(path);
        match doodle_files::write_atomic(&path, &bytes) {
            Ok(()) => {
                self.last_export_path = Some(path.clone());
                self.mark_saved();
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("board");
                self.set_notice(NoticeKind::Info, format!("Exported {name}"));
            }
            Err(error) => self.set_notice(NoticeKind::Error, format!("Export failed: {error}")),
        }
    }

    pub(crate) fn confirm_pin(&mut self, card_id: u64) {
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
        let pose = Pose::new(
            card.card.world_pos.x + 384.0 / self.camera.zoom.max(doodle_types::MIN_ZOOM),
            card.card.world_pos.y,
        );
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

    pub(crate) fn poll_cards(&mut self) {
        let now = Instant::now();
        for card in &mut self.cards {
            if card.card.poll() {
                card.mark_changed(now);
            }
            if !card.card.is_busy() {
                card.busy_started = None;
            }
        }
    }

    pub(crate) fn handle_close_request(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.viewport().close_requested()) {
            return;
        }
        if self.closing || !self.dirty() {
            return;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        self.show_close_confirm = true;
    }

    pub(crate) fn request_close(&mut self, ctx: &egui::Context) {
        self.closing = true;
        self.show_close_confirm = false;
        // Cancel any in-flight provider work so owned threads stop before exit.
        for card in &mut self.cards {
            card.card.cancel();
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    pub(crate) fn schedule_repaint(&self, ctx: &egui::Context) {
        let animated_cards = self.cards.iter().any(|card| {
            card.card.is_busy() || !self.reduced_motion || card.hover_active(Instant::now())
        });
        let active_gesture = self.stroke.is_some()
            || self.drag.as_ref().is_some_and(|drag| drag.moved)
            || self.marquee.is_some();
        if animated_cards || active_gesture {
            ctx.request_repaint_after(Duration::from_millis(33));
        }
    }
}

impl eframe::App for DoodleApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.refresh();
        self.poll_cards();
        self.handle_close_request(&ctx);
        self.last_frame = Instant::now();

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(to_color32(self.grid.background_rgba)))
            .show(ui, |ui| {
                self.canvas_ui(ui);
            });

        self.toolbox_ui(&ctx);
        self.status_islands(&ctx);
        self.settings_window(&ctx);
        self.help_window(&ctx);
        self.close_modal(&ctx);
        self.pin_modal(&ctx);
        self.card_areas(&ctx);
        self.schedule_repaint(&ctx);
    }
}

pub(crate) fn screen_rect(rect: egui::Rect) -> Rect {
    Rect::new(
        rect.min.x as f64,
        rect.min.y as f64,
        rect.max.x as f64,
        rect.max.y as f64,
    )
}

fn read_board_file(path: &PathBuf) -> Result<doodle_files::ImportPayload, String> {
    let metadata = std::fs::metadata(path).map_err(|error| format!("Cannot open file: {error}"))?;
    if metadata.len() > MAX_IMPORT_BYTES + 64 {
        return Err("File is too large to import".into());
    }
    let bytes = std::fs::read(path).map_err(|error| format!("Cannot read file: {error}"))?;
    doodle_files::decode_and_validate(&bytes)
        .map_err(|error| format!("Invalid board file: {error}"))
}

pub(crate) fn to_color32(rgba: [u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
}
