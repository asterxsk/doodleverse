use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use doodle_ai::MockProvider;
use doodle_canvas::{BrushSettings, Camera, RawSample, StrokeBuilder};
use doodle_document::{BoardDocument, BoardProjection};
use doodle_render::{GridKind, GridSettings};
use doodle_types::{
    FillStyle, MAX_IMPORT_BYTES, MIN_SHAPE_SIZE, ObjectId, ObjectKind, Point, Pose, Rect,
    ShapeKind, ShapeObject, StatementKind, StatementObject, StatementStyle, StrokeGeometry,
    StrokeStyle,
};
use rune::{
    AppHandle, Drag, FileDialogOptions, Key, KeyEvent, MagnifyEvent, Mods, Phase, PointerButton,
    Tx, WheelEvent,
};

pub use crate::cards::{CardUi, PinPreview};
pub use crate::placement::{
    STATEMENT_MAX_HEIGHT, STATEMENT_WIDTH, board_is_empty, next_order_key, placement_is_valid,
    placement_rect, placement_translation, statement_natural_height, statement_size,
    statement_size_at,
};

/// How long after a popover is dismissed by a pointer press the board ignores presses. It covers
/// the frame in which the scrim's handler and the board's handler can both see the same press.
const POPOVER_PRESS_SUPPRESSION: f64 = 0.05;

/// How long after a popover trigger is pressed a board press is treated as part of the same
/// overlapping gesture. A second pointer going down before the trigger's release would otherwise
/// draw on the board while the popover is on its way open.
const POPOVER_TRIGGER_WINDOW: f64 = 0.12;

/// Window-point insets that board content must stay clear of: the menu bar and toolbar along the
/// top and the bottom-left cluster along the bottom, each plus a 12 pt margin. The left edge is
/// dynamic: it moves right past the tool-settings panel when that panel is visible.
pub const CHROME_TOP_INSET: f32 = 68.0;
pub const CHROME_BOTTOM_INSET: f32 = 61.0;
/// The tool-settings panel's fixed slot: 12 pt from the left, below the 44 pt top bar plus a gap.
pub const TOOL_PANEL_X: f32 = 12.0;
pub const TOOL_PANEL_Y: f32 = 64.0;
pub const TOOL_PANEL_WIDTH: f32 = 208.0;

/// Bottom-left controls. The minimap sits 12 pt above the cluster and is 200×140.
pub const MINIMAP_WIDTH: f32 = 200.0;
pub const MINIMAP_HEIGHT: f32 = 140.0;
/// Inset of the minimap's board drawing from its panel edge, so object strokes stay readable.
const MINIMAP_PAD: f64 = 8.0;
/// The bottom-left cluster's height and its distance from the window's bottom edge.
const CLUSTER_HEIGHT: f32 = 36.0;
const CLUSTER_BOTTOM: f32 = 10.0;

/// Sticky notes are a fixed note; text blocks measure their width between these bounds.
const STICKY_WIDTH: f64 = 240.0;
const STICKY_HEIGHT: f64 = 160.0;
const TEXT_MIN_WIDTH: f64 = 120.0;
const TEXT_MAX_WIDTH: f64 = 320.0;
const TEXT_MIN_HEIGHT: f64 = 30.0;
/// The style a statement's text is laid out with on the board (`canvas::statement_texts` at 100%).
const STATEMENT_TEXT_STYLE: rune::TextStyle = rune::TextStyle::new(14.0, 500.0);

/// Screen-point margin a pinned note keeps from the window's left and right edges, matching the
/// margin cards are clamped by. Without it a note pushed to the edge of a narrow, panned board
/// ends exactly on the window border with nowhere to breathe.
const BOARD_SCREEN_MARGIN: f64 = 12.0;

/// The window the app opens at. The first view runs before layout has measured anything, so
/// `DoodleApp::new` starts from this size to place the hint and choose the menu's contents for
/// the frame it draws; a headless run's `--size` replaces it.
pub const INITIAL_WINDOW_SIZE: [f32; 2] = [1180.0, 760.0];

/// The size the first view is laid out for. A headless run publishes its `--size` before state
/// creation, so a snapshot of frame zero matches the viewport; a window opens at the size in
/// `AppSpec`, which is [`INITIAL_WINDOW_SIZE`].
fn initial_window_size() -> [f32; 2] {
    rune::tooling::headless_run().map_or(INITIAL_WINDOW_SIZE, |run| {
        let (width, height) = run.size();
        [width, height]
    })
}

/// Which colour row owns the open custom-colour picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColourTarget {
    Stroke,
    Fill,
}

impl DoodleApp {
    /// Open the picker for `target`, its hex field seeded with the colour the row currently has.
    pub fn open_picker(&mut self, target: ColourTarget) {
        let current = match target {
            ColourTarget::Stroke => self.color,
            ColourTarget::Fill => self.fill_rgba,
        };
        self.picker_hex.replace_text(hex_of(current));
        self.picker = Some(target);
    }

    /// Apply the picker's hex field, if it parses, and close the picker.
    pub fn apply_picker_hex(&mut self) {
        let Some(target) = self.picker else { return };
        if let Some(rgba) = parse_hex(&self.picker_hex.text) {
            self.set_picker_colour(target, rgba);
        }
        self.picker = None;
    }

    /// Set the colour the picker is choosing for and remember it behind the row's last swatch.
    pub fn set_picker_colour(&mut self, target: ColourTarget, rgba: [u8; 4]) {
        match target {
            ColourTarget::Stroke => {
                self.color = rgba;
                if rgba[3] > 0 {
                    self.stroke_custom = rgba;
                }
            }
            ColourTarget::Fill => {
                self.fill_rgba = rgba;
                if rgba[3] > 0 {
                    self.fill_custom = rgba;
                }
            }
        }
    }
}

/// `#RRGGBB` for a held colour, the form the picker's field edits and the board stores.
pub fn hex_of(rgba: [u8; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgba[0], rgba[1], rgba[2])
}

/// Parse `#rgb`, `#rrggbb` or the same without `#`. The alpha is kept at 255: opacity is the
/// panel's own control, and a picker colour that carried alpha would double-apply it.
pub fn parse_hex(text: &str) -> Option<[u8; 4]> {
    let digits = text.trim().trim_start_matches('#');
    let byte = |index: usize| u8::from_str_radix(&digits[index..index + 2], 16).ok();
    match digits.len() {
        3 => {
            let mut out = [255u8; 4];
            for (index, ch) in digits.chars().enumerate() {
                let value = ch.to_digit(16)?;
                out[index] = (value * 17) as u8;
            }
            Some(out)
        }
        6 => Some([byte(0)?, byte(2)?, byte(4)?, 255]),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Draw,
    Select,
    Hand,
    Rectangle,
    Diamond,
    Ellipse,
    Arrow,
    Line,
    Text,
    Sticky,
    Eraser,
}

impl Tool {
    /// The shape a drag with this tool commits, if it draws one.
    pub fn shape_kind(self) -> Option<ShapeKind> {
        match self {
            Self::Rectangle => Some(ShapeKind::Rectangle),
            Self::Diamond => Some(ShapeKind::Diamond),
            Self::Ellipse => Some(ShapeKind::Ellipse),
            Self::Arrow => Some(ShapeKind::Arrow),
            Self::Line => Some(ShapeKind::Line),
            _ => None,
        }
    }

    /// Whether a click with this tool opens the inline text/sticky editor.
    pub fn statement_kind(self) -> Option<StatementKind> {
        match self {
            Self::Text => Some(StatementKind::Text),
            Self::Sticky => Some(StatementKind::Sticky),
            _ => None,
        }
    }
}

/// A shape drag in world space. The preview is drawn from it and the release commits one
/// `CreateShape`; nothing reaches the document before the release.
pub struct ShapeDrag {
    pub kind: ShapeKind,
    pub start: Point,
    pub current: Point,
}

/// An inline text or sticky editor, anchored at the world point it was opened on.
pub struct InlineEditor {
    pub kind: StatementKind,
    pub world: Point,
}

/// Board-to-minimap projection: the board extent's centre and its scale into the panel.
pub struct MinimapMap {
    pub center: Point,
    pub scale: f64,
}

impl MinimapMap {
    pub fn to_local(&self, world: Point) -> [f32; 2] {
        [
            MINIMAP_WIDTH * 0.5 + ((world.x - self.center.x) * self.scale) as f32,
            MINIMAP_HEIGHT * 0.5 + ((world.y - self.center.y) * self.scale) as f32,
        ]
    }
}

/// A whole-object erase drag. Objects are collected in world space and removed as one batch
/// when the drag ends, so the whole gesture is a single undo step. Until then the board skips
/// them, which shows the erase live without touching the document.
pub struct EraseGesture {
    pub removed: BTreeSet<ObjectId>,
    pub last: Point,
}

/// Screen-point radius of the eraser. Converted to world points at the current zoom, so the
/// pointer erases the same amount of board at every zoom.
pub const ERASER_RADIUS: f64 = 6.0;

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
    Opacity,
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
    /// Stroke and fill opacity for the next drawing, 0..=1. Existing objects keep their own
    /// alpha, as with every other style control in this pass.
    pub opacity: f32,
    /// The slider being dragged, so its thumb can hold coss's 1.2 drag scale until release.
    pub slider_drag: Option<Target>,
    pub fill_rgba: [u8; 4],
    pub fill_style: FillStyle,
    /// Background colour for the next sticky note, and opacity for text ink.
    pub note_rgba: [u8; 4],
    /// Whether mouse samples simulate speed-driven pressure. Hardware pen pressure is always
    /// honoured; this is the panel's pressure switch.
    pub pressure_enabled: bool,
    pub grid: GridSettings,
    pub light_theme: bool,
    pub reduced_motion: bool,
    pub stroke: Option<StrokeGesture>,
    pub drag: Option<SelectDrag>,
    pub marquee: Option<Marquee>,
    pub erasing: Option<EraseGesture>,
    pub shape_drag: Option<ShapeDrag>,
    pub editor: Option<InlineEditor>,
    pub editor_edit: rune::TextEdit,
    /// Which swatch row's custom-colour picker is open, if any.
    pub picker: Option<ColourTarget>,
    /// The picker's hex field; its text is parsed on Apply.
    pub picker_hex: rune::TextEdit,
    /// The colour behind the Stroke row's last swatch, and the one behind Background's.
    pub stroke_custom: [u8; 4],
    pub fill_custom: [u8; 4],
    pub toolbar_locked: bool,
    pub minimap_open: bool,
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
    pub overflow_open: bool,
    /// A press that dismissed a popover (or landed while one was open) must not also start a
    /// board gesture. A scrim handler can run before the board handler in the same frame, so the
    /// dismissal leaves this deadline; the next board press inside it is consumed.
    pub suppress_board_press_until: Option<f64>,
    pub pin_preview: Option<PinPreview>,
    pub canvas_layout: [f32; 4],
    /// The tool-settings panel's measured window rect, for wheel routing. Zero height before its
    /// first layout pass.
    pub tool_panel_layout: [f32; 4],
    pub window_size: [f32; 2],
    pub last_pointer_world: Point,
    /// The pointer in canvas-local screen points, for the eraser feedback that the canvas
    /// draws itself (the OS cursor is hidden under the eraser tool).
    pub pointer_screen: Option<[f32; 2]>,
    /// Whether the pointer is over the board rather than the chrome over it.
    pub pointer_on_canvas: bool,
    /// Smoothed direction of the pointer's recent board movement, for the eraser tail.
    pub eraser_trail_dir: Option<[f32; 2]>,
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
        let window_size = initial_window_size();
        Self {
            saved_revision: Some(doc.revision()),
            projection_revision: doc.revision(),
            projection,
            doc,
            camera: Camera::default(),
            selection: BTreeSet::new(),
            tool: Tool::Draw,
            brush: BrushSettings::default(),
            // Dark chrome is the default; light remains available from Canvas settings.
            color: [238, 234, 227, 255],
            opacity: 1.0,
            slider_drag: None,
            fill_rgba: [0, 0, 0, 0],
            fill_style: FillStyle::default(),
            note_rgba: [255, 241, 186, 255],
            pressure_enabled: true,
            grid: GridSettings {
                background_rgba: [22, 22, 22, 255],
                line_rgba: [82, 82, 82, 255],
                ..GridSettings::default()
            },
            light_theme: false,
            reduced_motion: false,
            stroke: None,
            drag: None,
            marquee: None,
            erasing: None,
            shape_drag: None,
            editor: None,
            editor_edit: rune::TextEdit::new(""),
            picker: None,
            picker_hex: rune::TextEdit::new(""),
            stroke_custom: [224, 49, 49, 255],
            fill_custom: [255, 236, 153, 255],
            toolbar_locked: false,
            minimap_open: false,
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
            overflow_open: false,
            suppress_board_press_until: None,
            pin_preview: None,
            canvas_layout: [0.0, 0.0, window_size[0], window_size[1]],
            tool_panel_layout: [TOOL_PANEL_X, TOOL_PANEL_Y, TOOL_PANEL_WIDTH, 0.0],
            window_size,
            last_pointer_world: Point::default(),
            pointer_screen: None,
            pointer_on_canvas: false,
            eraser_trail_dir: None,
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

    /// The left tool-settings panel is shown for every drawing tool and hidden for the
    /// navigation tools and zen mode.
    pub fn tool_panel_visible(&self) -> bool {
        !self.zen
            && matches!(
                self.tool,
                Tool::Draw
                    | Tool::Rectangle
                    | Tool::Diamond
                    | Tool::Ellipse
                    | Tool::Arrow
                    | Tool::Line
                    | Tool::Text
                    | Tool::Sticky
            )
    }

    /// Whether the active tool draws a closed shape, which is the only kind that can carry a
    /// background and a fill style.
    pub fn tool_panel_closed_shape(&self) -> bool {
        matches!(self.tool, Tool::Rectangle | Tool::Diamond | Tool::Ellipse)
    }

    /// Whether the active tool has a stroke width: text and sticky notes do not.
    pub fn tool_panel_stroke_width(&self) -> bool {
        !matches!(self.tool, Tool::Text | Tool::Sticky)
    }

    pub fn tool_panel_pen(&self) -> bool {
        matches!(self.tool, Tool::Draw)
    }

    /// The panel's current window rect while it is visible. Zero height means it has not laid
    /// out yet, so wheel routing should fall through.
    pub fn tool_panel_rect(&self) -> Option<[f32; 4]> {
        (self.tool_panel_visible() && self.tool_panel_layout[3] > 0.0)
            .then_some(self.tool_panel_layout)
    }

    /// Window-point inset the board's content must keep clear of on the left. The panel's slot
    /// plus a 12 pt gap when it is up, otherwise the plain window margin.
    pub fn board_left_inset(&self) -> f32 {
        if self.tool_panel_visible() {
            TOOL_PANEL_X + TOOL_PANEL_WIDTH + 12.0
        } else {
            12.0
        }
    }

    /// The stroke colour the next object is created with, after the opacity control.
    pub fn stroke_rgba(&self) -> [u8; 4] {
        with_alpha(self.color, self.opacity)
    }

    /// The fill colour the next closed shape is created with, after the opacity control.
    pub fn fill_rgba_effective(&self) -> [u8; 4] {
        with_alpha(self.fill_rgba, self.opacity)
    }

    /// The sticky-note background the next note is created with, after the opacity control.
    pub fn note_rgba_effective(&self) -> [u8; 4] {
        with_alpha(self.note_rgba, self.opacity)
    }

    fn effective_brush(&self) -> BrushSettings {
        BrushSettings {
            simulate_pressure: self.pressure_enabled,
            ..self.brush
        }
    }

    /// A scrim press dismisses the open popover. The press can also have reached the board in the
    /// same frame (before or after this handler), so the board ignores the next press instead of
    /// treating it as the start of a stroke.
    pub fn dismiss_popovers_by_pointer(&mut self) {
        self.menu_open = false;
        self.overflow_open = false;
        self.suppress_board_press_until = Some(self.clock + POPOVER_PRESS_SUPPRESSION);
    }

    /// The menu and overflow triggers mark their press-down, so a second press that lands before
    /// they release (two simultaneous taps) is consumed instead of starting a stroke.
    pub fn popover_trigger_drag(&mut self, drag: &Drag) {
        if drag.phase == Phase::Down {
            self.suppress_board_press_until = Some(self.clock + POPOVER_TRIGGER_WINDOW);
        }
    }

    fn board_press_is_suppressed(&self) -> bool {
        self.suppress_board_press_until
            .is_some_and(|until| self.clock <= until)
    }

    fn cancel_press_gestures(&mut self) {
        self.stroke = None;
        self.drag = None;
        self.marquee = None;
        self.erasing = None;
        self.shape_drag = None;
        self.panning = false;
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

    pub fn create_stroke(
        &mut self,
        pose: Pose,
        style: StrokeStyle,
        geometry: StrokeGeometry,
        fill_rgba: [u8; 4],
    ) {
        match self.doc.create_stroke(pose, style, geometry, fill_rgba) {
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
        self.erasing = None;
        self.shape_drag = None;
        self.panning = false;
        self.pin_preview = None;
        self.cancel_editor();
        self.cancel_pending_import();
    }

    /// Switches the active tool. Every in-flight board gesture belongs to the previous tool, so
    /// they are dropped rather than committed under the new one. An open inline editor is
    /// committed, as pressing a tool away from it finishes what was typed.
    pub fn set_tool(&mut self, tool: Tool) {
        self.commit_editor();
        self.tool = tool;
        // A stale direction would make the eraser tail point where the previous tool moved, so
        // the next pointer move only re-establishes the position.
        self.eraser_trail_dir = None;
        self.pointer_screen = None;
        self.cancel_press_gestures();
    }

    // ---- shapes and inline text ------------------------------------------------------------

    /// The shape the current drag would commit, or `None` when no shape tool is dragging, the
    /// drag is too small to mean anything, or the constrained box would be empty.
    pub fn shape_preview(&self) -> Option<(ShapeObject, Pose)> {
        let drag = self.shape_drag.as_ref()?;
        let mut dx = drag.current.x - drag.start.x;
        let mut dy = drag.current.y - drag.start.y;
        if (dx.hypot(dy) * self.zoom()) < 4.0 {
            return None;
        }
        if self.mods.shift {
            let side = dx.abs().max(dy.abs());
            dx = side.copysign(if dx < 0.0 { -1.0 } else { 1.0 });
            dy = side.copysign(if dy < 0.0 { -1.0 } else { 1.0 });
        }
        let pose = Pose::new(drag.start.x + dx.min(0.0), drag.start.y + dy.min(0.0));
        // Open paths may be zero-extent on one axis, so a perfectly horizontal or vertical line
        // keeps its true size instead of growing a 1pt box. Only they carry direction flags:
        // closed shapes are defined by the pose corner alone.
        let open = drag.kind.is_open();
        let min_size = if open { 0.0 } else { MIN_SHAPE_SIZE };
        let shape = ShapeObject {
            shape: drag.kind,
            width: dx.abs().max(min_size),
            height: dy.abs().max(min_size),
            style: self.effective_brush().style(self.stroke_rgba()),
            flip_x: open && dx < 0.0,
            flip_y: open && dy < 0.0,
            fill_rgba: if open {
                [0, 0, 0, 0]
            } else {
                self.fill_rgba_effective()
            },
            fill_style: self.fill_style,
        };
        Some((shape, pose))
    }

    /// Commits the drag as one `CreateShape`, so a single undo removes it.
    pub fn commit_shape(&mut self) {
        if self.shape_drag.is_none() {
            return;
        }
        let preview = self.shape_preview();
        self.shape_drag = None;
        let Some((shape, pose)) = preview else {
            return;
        };
        match self.doc.create_shape(pose, shape) {
            Ok(_) => {
                self.hint_visible = false;
                self.refresh();
            }
            Err(error) => self.set_notice(NoticeKind::Error, format!("Shape rejected: {error}")),
        }
    }

    pub fn open_editor(&mut self, kind: StatementKind, world: Point) {
        self.editor_edit = rune::TextEdit::new("");
        self.editor = Some(InlineEditor { kind, world });
    }

    /// Enter: an empty editor closes without creating anything; otherwise the text becomes a
    /// `Text` or `Sticky` statement at the point the editor was opened on.
    pub fn commit_editor(&mut self) {
        let Some(editor) = self.editor.take() else {
            return;
        };
        let text = self.editor_edit.text.trim().to_owned();
        self.editor_edit = rune::TextEdit::new("");
        if text.is_empty() {
            return;
        }
        let (width, height) = match editor.kind {
            StatementKind::Sticky => (STICKY_WIDTH, STICKY_HEIGHT),
            StatementKind::Text => {
                let (natural, _) = rune::text::measure(&text, &[], STATEMENT_TEXT_STYLE, None);
                let width = f64::from(natural).clamp(TEXT_MIN_WIDTH, TEXT_MAX_WIDTH);
                let (_, wrapped) =
                    rune::text::measure(&text, &[], STATEMENT_TEXT_STYLE, Some(width as f32));
                (width, f64::from(wrapped).max(TEXT_MIN_HEIGHT))
            }
        };
        let style = match editor.kind {
            StatementKind::Text => StatementStyle {
                rgba: self.stroke_rgba(),
            },
            StatementKind::Sticky => StatementStyle {
                rgba: self.note_rgba_effective(),
            },
        };
        let statement = StatementObject {
            text,
            width,
            height,
            style,
            kind: editor.kind,
        };
        let pose = Pose::new(editor.world.x, editor.world.y);
        match self.doc.create_statement(pose, statement) {
            Ok(_) => {
                self.hint_visible = false;
                self.refresh();
            }
            Err(error) => self.set_notice(NoticeKind::Error, format!("Text rejected: {error}")),
        }
    }

    /// Escape drops the editor and whatever was typed, creating nothing.
    pub fn cancel_editor(&mut self) {
        if self.editor.take().is_some() {
            self.editor_edit = rune::TextEdit::new("");
        }
    }

    // ---- minimap ---------------------------------------------------------------------------

    /// The minimap's window rect, 12 pt above the bottom-left cluster.
    pub fn minimap_rect(&self) -> [f32; 4] {
        let top = self.window_size[1] - CLUSTER_BOTTOM - CLUSTER_HEIGHT - 12.0 - MINIMAP_HEIGHT;
        [12.0, top, MINIMAP_WIDTH, MINIMAP_HEIGHT]
    }

    /// Mapping between the board and the minimap's local points: board content and the visible
    /// viewport together, letterboxed inside the panel with an 8 pt inset.
    pub fn minimap_map(&self) -> MinimapMap {
        let visible = self.camera.visible_world(self.viewport());
        let extent = self
            .projection
            .content_bounds()
            .map_or(visible, |bounds| bounds.union(visible));
        let usable_w = (f64::from(MINIMAP_WIDTH) - MINIMAP_PAD * 2.0).max(1.0);
        let usable_h = (f64::from(MINIMAP_HEIGHT) - MINIMAP_PAD * 2.0).max(1.0);
        let scale = (usable_w / extent.width().max(1.0)).min(usable_h / extent.height().max(1.0));
        MinimapMap {
            center: extent.center(),
            scale,
        }
    }

    /// The world point under a minimap-local point.
    pub fn minimap_world_at(&self, local: [f32; 2]) -> Point {
        let map = self.minimap_map();
        Point::new(
            map.center.x + (f64::from(local[0]) - f64::from(MINIMAP_WIDTH) * 0.5) / map.scale,
            map.center.y + (f64::from(local[1]) - f64::from(MINIMAP_HEIGHT) * 0.5) / map.scale,
        )
    }

    /// Clicking or dragging in the minimap centres the camera on that board point.
    pub fn minimap_navigate(&mut self, local: [f32; 2]) {
        let target = self.minimap_world_at(local);
        let zoom = self.zoom();
        self.camera.pan_by_screen_delta(
            (self.camera.center.x - target.x) * zoom,
            (self.camera.center.y - target.y) * zoom,
        );
    }

    pub fn minimap_drag(&mut self, drag: &Drag) {
        if drag.phase != Phase::Up {
            self.minimap_navigate([drag.x, drag.y]);
        }
    }

    pub fn apply_slider(&mut self, target: Target, fraction: f64) {
        let f = fraction.clamp(0.0, 1.0);
        let lerp = |min: f64, max: f64| min + (max - min) * f;
        match target {
            Target::Opacity => self.opacity = f as f32,
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
            Target::Opacity => self.opacity,
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

    /// The value a slider announces to assistive technology, in the unit the user sees.
    pub fn slider_value_text(&self, target: Target) -> String {
        match target {
            Target::Opacity => format!("{:.0}%", self.opacity.clamp(0.0, 1.0) * 100.0),
            Target::Smoothing => format!("{:.0}%", self.brush.smoothing * 100.0),
            Target::TaperStart => format!("{:.0}%", self.brush.taper_start / 0.45 * 100.0),
            Target::TaperEnd => format!("{:.0}%", self.brush.taper_end / 0.45 * 100.0),
            Target::GridSpacing => format!("{:.0} px", self.grid.spacing),
            Target::GridOpacity => format!("{:.0}%", self.grid.opacity.clamp(0.0, 1.0) * 100.0),
            Target::GridMarkSize => format!("{:.1} px", self.grid.mark_size),
            Target::GridLineWidth => format!("{:.1} px", self.grid.line_width),
            Target::CustomBoardRed
            | Target::CustomBoardGreen
            | Target::CustomBoardBlue
            | Target::CustomGridRed
            | Target::CustomGridGreen
            | Target::CustomGridBlue => {
                let channel = match target {
                    Target::CustomBoardRed | Target::CustomGridRed => 0,
                    Target::CustomBoardGreen | Target::CustomGridGreen => 1,
                    _ => 2,
                };
                let rgba = match target {
                    Target::CustomBoardRed | Target::CustomBoardGreen | Target::CustomBoardBlue => {
                        self.grid.background_rgba
                    }
                    _ => self.grid.line_rgba,
                };
                format!("{} of 255", rgba[channel])
            }
        }
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

    /// Foreground panels are mutually exclusive: at most one of Canvas settings, Shortcuts &
    /// about, and the minimap is open. Opening one closes the others, and closing it never
    /// reopens them.
    pub fn open_settings(&mut self) {
        self.show_settings = true;
        self.show_help = false;
        self.minimap_open = false;
    }

    pub fn toggle_settings(&mut self) {
        if self.show_settings {
            self.show_settings = false;
        } else {
            self.open_settings();
        }
    }

    pub fn open_help(&mut self) {
        self.show_help = true;
        self.show_settings = false;
        self.minimap_open = false;
    }

    pub fn toggle_help(&mut self) {
        if self.show_help {
            self.show_help = false;
        } else {
            self.open_help();
        }
    }

    /// The minimap button toggles the minimap's own state; opening it does close the two
    /// foreground panels, and closing it never reopens them.
    pub fn toggle_minimap(&mut self) {
        if self.minimap_open {
            self.minimap_open = false;
        } else {
            self.minimap_open = true;
            self.show_settings = false;
            self.show_help = false;
        }
    }

    /// Zen mode hides every island, so anything opened from one closes with it.
    pub fn toggle_zen(&mut self) {
        self.zen = !self.zen;
        if self.zen {
            self.show_settings = false;
            self.show_help = false;
            self.menu_open = false;
            self.overflow_open = false;
        }
    }

    pub fn toggle_theme(&mut self) {
        self.light_theme = !self.light_theme;
        if self.light_theme {
            self.grid.background_rgba = [255, 255, 255, 255];
            self.grid.line_rgba = [212, 212, 212, 255];
        } else {
            self.grid.background_rgba = [22, 22, 22, 255];
            self.grid.line_rgba = [82, 82, 82, 255];
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
        let viewport = self.viewport();
        // Keep the note in the board area the chrome leaves free: clear of the menu bar and
        // toolbar on top, the status and zoom islands at the bottom, and the same 12 pt side
        // margin cards are clamped by, so a note never lands on the window edge.
        let top = f64::from(CHROME_TOP_INSET);
        let bottom = f64::from(CHROME_BOTTOM_INSET);
        let board_min = self.camera.screen_to_world(
            Point::new(f64::from(self.board_left_inset()), top),
            viewport,
        );
        let board_max = self.camera.screen_to_world(
            Point::new(
                (viewport.width() - BOARD_SCREEN_MARGIN).max(BOARD_SCREEN_MARGIN),
                (viewport.height() - bottom).max(top),
            ),
            viewport,
        );
        let Some(card) = self.cards.iter().find(|card| card.card.local_id == card_id) else {
            self.pin_preview = None;
            return;
        };
        let Some(text) = card.card.pin_text().map(str::to_owned) else {
            self.pin_preview = None;
            return;
        };
        let free_height = board_max.y - board_min.y;
        // Place the note against the card's visible rect, not its world position: a card pushed
        // back inside the window by the clamp still sits where the user sees it. Right, left,
        // above and below are tried in that order, each clamped on screen; the first that stays
        // on the board and clear of the card wins.
        let layout = crate::cards::card_layout(self, card);
        let card_min = self.camera.screen_to_world(
            Point::new(f64::from(layout.x), f64::from(layout.y)),
            viewport,
        );
        let card_max = self.camera.screen_to_world(
            Point::new(
                f64::from(layout.x + layout.width),
                f64::from(layout.y + layout.height),
            ),
            viewport,
        );
        let card_rect = Rect::new(card_min.x, card_min.y, card_max.x, card_max.y);
        let margin = 24.0 / zoom;
        let right_margin = 48.0 / zoom;
        let candidates_for = |width: f64, height: f64| {
            let max_x = (board_max.x - width).max(board_min.x);
            let max_y = (board_max.y - height).max(board_min.y);
            let clamp_x = |x: f64| x.clamp(board_min.x, max_x);
            let clamp_y = |y: f64| y.clamp(board_min.y, max_y);
            [
                (
                    clamp_x(card_rect.max_x + right_margin),
                    clamp_y(card_rect.min_y),
                ),
                (
                    clamp_x(card_rect.min_x - margin - width),
                    clamp_y(card_rect.min_y),
                ),
                (
                    clamp_x(card_rect.min_x),
                    clamp_y(card_rect.min_y - margin - height),
                ),
                (clamp_x(card_rect.min_x), clamp_y(card_rect.max_y + margin)),
            ]
        };
        let place = |width: f64, height: f64| -> Option<Rect> {
            candidates_for(width, height)
                .into_iter()
                .map(|(x, y)| Rect::new(x, y, x + width, y + height))
                .find(|rect| overlap_area(*rect, card_rect) <= 0.0)
        };
        // A card that fills most of a small viewport leaves no side wide enough for the natural
        // note. Narrower widths are tried in turn; each re-wraps the text, and a width whose text
        // would be clipped by the height clamp or the free board area is skipped. Only when even
        // the narrowest width cannot fit does the note fall back to least overlap.
        let mut fitting = None;
        for width in [STATEMENT_WIDTH, 300.0, 260.0, 220.0, 200.0] {
            let natural_height = statement_natural_height(&text, width);
            if natural_height > STATEMENT_MAX_HEIGHT || natural_height > free_height {
                continue;
            }
            let (candidate_width, candidate_height) = statement_size_at(&text, width);
            if let Some(rect) = place(candidate_width, candidate_height) {
                fitting = Some((candidate_width, candidate_height, rect));
                break;
            }
        }
        let (width, height, chosen) = match fitting {
            Some((width, height, rect)) => (width, height, rect),
            None => {
                // A very long answer cannot be taller than the free board area, or its lower body
                // would sit under the status island however it is placed.
                let (width, mut height) = statement_size(&text);
                if height > free_height {
                    height = free_height.max(96.0);
                }
                let mut chosen = Rect::new(0.0, 0.0, 0.0, 0.0);
                let mut best_overlap = f64::INFINITY;
                for (x, y) in candidates_for(width, height) {
                    let rect = Rect::new(x, y, x + width, y + height);
                    let overlap = overlap_area(rect, card_rect);
                    if overlap < best_overlap {
                        best_overlap = overlap;
                        chosen = rect;
                    }
                }
                (width, height, chosen)
            }
        };
        let statement = StatementObject {
            text,
            width,
            height,
            style: StatementStyle::default(),
            kind: StatementKind::Sticky,
        };
        let pose = Pose::new(chosen.min_x, chosen.min_y);
        match self.doc.create_statement(pose, statement) {
            Ok(_) => {
                if let Some(card) = self
                    .cards
                    .iter_mut()
                    .find(|card| card.card.local_id == card_id)
                {
                    card.pinned = true;
                }
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

    pub fn poll_cards(&mut self, tx: &mut Tx) -> bool {
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
            let busy = card.card.is_busy();
            if busy != card.was_busy {
                card.was_busy = busy;
                // Streaming follows the bottom of the answer. A finished answer would otherwise
                // stay parked at its last line, so both ends of a generation jump the viewport
                // back to the first line.
                tx.scroll_to_y(format!("card-answer-scroll-{}", card.card.local_id), 0.0);
            }
            if !busy {
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
                ObjectKind::Shape(shape) => {
                    if doodle_canvas::geometry::shape_hit_clearance(
                        shape,
                        projected.object.pose,
                        world,
                    ) <= tolerance
                    {
                        return Some(projected.top_id);
                    }
                }
                ObjectKind::Group => {}
            }
        }
        None
    }

    /// Every top-level object the eraser's world-space radius touches at `world`. A stroke is
    /// hit against its centreline plus half its ink width, the same test selection uses, so the
    /// eraser removes a whole stroke rather than pixels. Statements are hit by their bounds.
    fn objects_at(&self, world: Point, radius: f64) -> Vec<ObjectId> {
        let mut ids = Vec::new();
        for projected in &self.projection.objects {
            let hit = match &projected.object.kind {
                ObjectKind::Stroke(stroke) => doodle_canvas::geometry::stroke_hit_clearance(
                    &stroke.geometry,
                    projected.object.pose,
                    world,
                )
                .is_some_and(|clearance| clearance <= radius),
                ObjectKind::Statement(_) => projected
                    .bounds
                    .is_some_and(|bounds| bounds.expand(radius).contains(world)),
                ObjectKind::Shape(shape) => {
                    doodle_canvas::geometry::shape_hit_clearance(
                        shape,
                        projected.object.pose,
                        world,
                    ) <= radius
                }
                ObjectKind::Group => false,
            };
            if hit {
                ids.push(projected.top_id);
            }
        }
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// Eraser hits along the segment the pointer just travelled, sampled every quarter radius so
    /// a fast drag cannot step over a thin line. Ids already erased in this gesture are skipped,
    /// so each object is removed once however often the pointer crosses it.
    fn erase_hits_between(&self, from: Point, to: Point) -> Vec<ObjectId> {
        let radius = ERASER_RADIUS / self.zoom();
        let length = from.distance(to);
        let steps = (length / (radius * 0.5)).ceil().max(1.0) as usize;
        let mut hits = Vec::new();
        for step in 0..=steps {
            let t = step as f64 / steps as f64;
            let at = Point::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
            hits.extend(self.objects_at(at, radius));
        }
        hits.sort_unstable();
        hits.dedup();
        hits.retain(|id| {
            self.erasing
                .as_ref()
                .is_none_or(|gesture| !gesture.removed.contains(id))
        });
        hits
    }

    /// An object the in-flight eraser drag has already removed from view. The document still
    /// holds it until the drag ends and the batch delete commits.
    pub fn pending_erase(&self, id: ObjectId) -> bool {
        self.erasing
            .as_ref()
            .is_some_and(|gesture| gesture.removed.contains(&id))
    }

    fn begin_erase(&mut self, world: Point) {
        self.stroke = None;
        self.drag = None;
        self.marquee = None;
        self.panning = false;
        let removed = self.objects_at(world, ERASER_RADIUS / self.zoom());
        self.erasing = Some(EraseGesture {
            removed: removed.into_iter().collect(),
            last: world,
        });
    }

    fn continue_erase(&mut self, world: Point) {
        let Some(last) = self.erasing.as_ref().map(|gesture| gesture.last) else {
            return;
        };
        let hits = self.erase_hits_between(last, world);
        if let Some(gesture) = self.erasing.as_mut() {
            gesture.removed.extend(hits);
            gesture.last = world;
        }
    }

    /// Commits the drag as one document command, so a single undo restores everything it
    /// removed. The pending set only hid the objects; the document still has them.
    fn finish_erase(&mut self, cancelled: bool) {
        let Some(gesture) = self.erasing.take() else {
            return;
        };
        if cancelled || gesture.removed.is_empty() {
            return;
        }
        let ids: Vec<ObjectId> = gesture.removed.into_iter().collect();
        let count = ids.len();
        match self.doc.delete_objects(&ids) {
            Ok(_) => {
                self.refresh();
                let noun = if count == 1 { "object" } else { "objects" };
                self.set_notice(NoticeKind::Info, format!("Erased {count} {noun}"));
            }
            Err(error) => self.set_notice(NoticeKind::Error, format!("Erase failed: {error}")),
        }
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
        self.track_pointer(x, y);
        let world = self
            .camera
            .screen_to_world(Point::new(f64::from(x), f64::from(y)), self.viewport());
        self.last_pointer_world = world;
        self.update_pending_import(world);
    }

    /// Remembers where the pointer is on the board and a smoothed direction of travel. The
    /// eraser's tail uses both, so it follows idle movement rather than appearing on a press.
    fn track_pointer(&mut self, x: f32, y: f32) {
        if !x.is_finite() || !y.is_finite() {
            return;
        }
        let next = [x, y];
        if let Some(previous) = self.pointer_screen {
            let delta = [next[0] - previous[0], next[1] - previous[1]];
            let distance = delta[0].hypot(delta[1]);
            if distance > 0.75 {
                let step = [delta[0] / distance, delta[1] / distance];
                self.eraser_trail_dir = Some(match self.eraser_trail_dir {
                    Some(previous) => {
                        let blended = [
                            previous[0] * 0.65 + step[0] * 0.35,
                            previous[1] * 0.65 + step[1] * 0.35,
                        ];
                        let length = blended[0].hypot(blended[1]).max(1e-3);
                        [blended[0] / length, blended[1] / length]
                    }
                    None => step,
                });
            }
        }
        self.pointer_screen = Some(next);
    }

    /// The settings panel is a fixed-height overlay, so its wheel scrolling is clamped here
    /// instead of zoomed underneath it.
    pub fn settings_rect(&self) -> Option<[f32; 4]> {
        self.show_settings.then(|| {
            let height = (self.window_size[1] - 150.0).clamp(280.0, 680.0);
            // Step past the tool-settings panel instead of painting under it.
            let x = if self.tool_panel_visible() {
                240.0
            } else {
                140.0
            };
            [x, 84.0, 320.0, height]
        })
    }

    /// The help panel's window rect, shared with `view::help_panel`. Its height leaves the
    /// bottom-left cluster (36 tall, 10 from the edge) plus a 12 pt gap clear.
    ///
    /// A local AI card is not a foreground panel, so help steps around it instead of covering
    /// it: the base slot is used while clear, then a slot left or right of a card, then one
    /// below the lowest card when the window is too narrow for both side by side.
    pub fn help_rect(&self) -> Option<[f32; 4]> {
        self.show_help.then(|| {
            const WIDTH: f32 = 420.0;
            let height = (self.window_size[1] - 188.0).clamp(240.0, 620.0);
            let base = [220.0, 120.0, WIDTH, height];
            let cards: Vec<[f32; 4]> = self
                .cards
                .iter()
                .map(|card| {
                    let layout = crate::cards::card_layout(self, card);
                    [layout.x, layout.y, layout.width, layout.height]
                })
                .collect();
            let clear = |rect: [f32; 4]| !cards.iter().any(|card| rects_overlap(rect, *card));
            if clear(base) {
                return base;
            }
            let max_x = (self.window_size[0] - WIDTH - 12.0).max(12.0);
            for card in &cards {
                for x in [card[0] - WIDTH - 12.0, card[0] + card[2] + 12.0] {
                    let candidate = [x.clamp(12.0, max_x), base[1], WIDTH, height];
                    if clear(candidate) {
                        return candidate;
                    }
                }
            }
            let below = cards
                .iter()
                .map(|card| card[1] + card[3])
                .fold(0.0, f32::max)
                + 12.0;
            if below + height <= self.window_size[1] - 12.0 {
                let candidate = [base[0], below, WIDTH, height];
                if clear(candidate) {
                    return candidate;
                }
            }
            base
        })
    }

    /// Whether the pointer is over a surface that scrolls itself and must keep the wheel: a
    /// panel scroll view or a local AI card. The board never zooms beneath these.
    fn wheel_over_overlay(&self, x: f32, y: f32) -> bool {
        let inside =
            |[rx, ry, rw, rh]: [f32; 4]| x >= rx && x <= rx + rw && y >= ry && y <= ry + rh;
        self.tool_panel_rect().is_some_and(inside)
            || self.settings_rect().is_some_and(inside)
            || self.help_rect().is_some_and(inside)
            || self.cards.iter().any(|card| {
                let layout = crate::cards::card_layout(self, card);
                inside([layout.x, layout.y, layout.width, layout.height])
            })
    }

    /// Plain wheel and `Ctrl`/`Cmd`+wheel zoom around the pointer, as Excalidraw does: a wheel
    /// notch up zooms in and a notch down zooms out. Rune reports the delta in content points,
    /// positive for content moving down and right, which is what a wheel notch up produces, so
    /// the factor grows with `dy`. `Shift`+wheel is the one pan gesture left on the wheel: it
    /// moves the camera horizontally (Rune has already turned a vertical wheel notch into `dx`
    /// on Windows and Linux, and macOS reports `dy`, which is used here).
    pub fn wheel(&mut self, ev: &WheelEvent) -> bool {
        if self.wheel_over_overlay(ev.x, ev.y) {
            // A panel or card scroll view takes the wheel; the board must not zoom beneath it.
            return false;
        }
        let viewport = self.viewport();
        if ev.mods.shift && !ev.mods.cmd && !ev.mods.ctrl {
            let dx = if ev.dx.abs() > f32::EPSILON {
                ev.dx
            } else {
                ev.dy
            };
            if dx.abs() > 0.05 {
                self.camera.pan_by_screen_delta(f64::from(dx), 0.0);
            }
            return true;
        }
        if ev.dy.abs() > 0.05 {
            let factor = (f64::from(ev.dy) * 0.0022).exp();
            self.camera.zoom_at(
                Point::new(f64::from(ev.x), f64::from(ev.y)),
                viewport,
                factor,
            );
        }
        true
    }

    pub fn magnify(&mut self, ev: &MagnifyEvent) -> bool {
        let viewport = self.viewport();
        // macOS reports a pinch directly, positive to zoom in. Windows and Linux report a pinch
        // and Ctrl+wheel as `-dy / 200` (see `El::on_magnify`), so the same wheel-up-zooms-in
        // direction the plain wheel uses is recovered here.
        let factor = if cfg!(target_os = "macos") {
            1.0 + f64::from(ev.delta)
        } else {
            (-f64::from(ev.delta) * 0.44).exp()
        };
        self.camera.zoom_at(
            Point::new(f64::from(ev.x), f64::from(ev.y)),
            viewport,
            factor,
        );
        true
    }

    pub fn canvas_drag(&mut self, drag: &Drag, button: PointerButton, tx: &mut Tx) {
        let viewport = self.viewport();
        self.track_pointer(drag.x, drag.y);
        let world = self
            .camera
            .screen_to_world(Point::new(f64::from(drag.x), f64::from(drag.y)), viewport);
        self.last_pointer_world = world;
        match drag.phase {
            Phase::Down => self.begin_press(drag, world, button, tx),
            Phase::Move => self.continue_press(drag, world),
            Phase::Up => self.finish_press(drag, world),
        }
    }

    fn begin_press(&mut self, drag: &Drag, world: Point, button: PointerButton, tx: &mut Tx) {
        // A press that dismisses a popover must not also start a board gesture. The scrim covers
        // the board, but in the frame it mounts its hit box can still be smaller than the window,
        // so the press can reach the canvas first; consume it here as the scrim would.
        if self.menu_open || self.overflow_open {
            self.dismiss_popovers_by_pointer();
            self.cancel_press_gestures();
            return;
        }
        // The scrim's own handler may have run first this frame and already dismissed the
        // popover. The dismissal leaves a short deadline, and this press is still the one that
        // did it, so it is consumed here rather than starting a stroke.
        if self.board_press_is_suppressed() {
            self.suppress_board_press_until = None;
            self.cancel_press_gestures();
            return;
        }
        // The same guard for a modal dialog: its scrim takes the press with a no-op handler, and
        // until that scrim has landed no stroke may start beneath it.
        if self.show_close_confirm || self.pin_preview.is_some() {
            return;
        }
        if self.pending_import.is_some() {
            self.commit_pending_import();
            return;
        }
        // The middle button pans the board whatever tool is active, exactly like the hand tool.
        if button == PointerButton::Middle {
            self.stroke = None;
            self.drag = None;
            self.marquee = None;
            self.erasing = None;
            self.shape_drag = None;
            self.panning = true;
            self.pan_last = [drag.x, drag.y];
            return;
        }
        // A press on the board finishes an inline editor first; text and sticky tools then open a
        // fresh one at the new point instead of drawing anything.
        if self.editor.is_some() {
            self.commit_editor();
        }
        let shift = self.mods.shift;
        if let Some(kind) = self.tool.statement_kind() {
            self.open_editor(kind, world);
            // `autofocus` only speaks to a modal; a board-anchored field asks for focus itself
            // so typing lands in the new editor straight away.
            tx.focus_when_shown("inline-editor-field", false);
            self.hint_visible = false;
            return;
        }
        match self.tool {
            Tool::Hand => {
                self.panning = true;
                self.pan_last = [drag.x, drag.y];
            }
            Tool::Draw => {
                let brush = self.effective_brush();
                let start = RawSample::new(world, doodle_platform::pen_pressure(), self.clock);
                let builder = StrokeBuilder::new(brush, start, 0.7 / self.zoom());
                self.stroke = Some(StrokeGesture {
                    builder,
                    style: brush.style(self.stroke_rgba()),
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
            Tool::Eraser => self.begin_erase(world),
            Tool::Rectangle | Tool::Diamond | Tool::Ellipse | Tool::Arrow | Tool::Line => {
                let Some(kind) = self.tool.shape_kind() else {
                    return;
                };
                self.stroke = None;
                self.drag = None;
                self.marquee = None;
                self.erasing = None;
                self.shape_drag = Some(ShapeDrag {
                    kind,
                    start: world,
                    current: world,
                });
            }
            Tool::Text | Tool::Sticky => {}
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
        if self.erasing.is_some() {
            self.continue_erase(world);
            return;
        }
        if let Some(gesture) = self.shape_drag.as_mut() {
            gesture.current = world;
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
        // A press that began while a popover was open, that dismissed one, or that was still held
        // when a popover opened (a tap begun just before the previous tap's release) is consumed:
        // it can never commit a stroke. Closing the popover here also makes a tap outside it
        // dismiss it, as the scrim would have.
        let popover_open = self.menu_open || self.overflow_open;
        if popover_open || self.board_press_is_suppressed() {
            self.menu_open = false;
            self.overflow_open = false;
            self.suppress_board_press_until = None;
            self.cancel_press_gestures();
            return;
        }
        if self.panning {
            self.panning = false;
            return;
        }
        if self.erasing.is_some() {
            self.finish_erase(drag.cancelled());
            return;
        }
        if self.shape_drag.is_some() {
            if drag.cancelled() {
                self.shape_drag = None;
            } else {
                self.commit_shape();
            }
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
                self.create_stroke(pose, gesture.style, geometry, self.fill_rgba);
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

/// Scale a colour's alpha by the opacity control. A transparent colour stays transparent.
fn with_alpha(rgba: [u8; 4], opacity: f32) -> [u8; 4] {
    let alpha = (f32::from(rgba[3]) * opacity.clamp(0.0, 1.0)).round();
    [rgba[0], rgba[1], rgba[2], alpha.clamp(0.0, 255.0) as u8]
}

/// Area two rects share; zero when they only touch or do not meet.
fn overlap_area(a: Rect, b: Rect) -> f64 {
    let width = (a.max_x.min(b.max_x) - a.min_x.max(b.min_x)).max(0.0);
    let height = (a.max_y.min(b.max_y) - a.min_y.max(b.min_y)).max(0.0);
    width * height
}

/// Whether two `[x, y, width, height]` window rects share any area. Touching edges do not
/// count, so a panel placed 12 pt clear of a card is clear at the exact boundary.
fn rects_overlap(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
}

pub fn on_modifiers(state: &mut DoodleApp, mods: Mods, _tx: &mut Tx) {
    state.mods = mods;
}

pub fn poll(state: &mut DoodleApp, t: f64, tx: &mut Tx) -> bool {
    state.clock = t;
    state.reload_handle();
    let refreshed = state.refresh();
    let cards = state.poll_cards(tx);
    let animating = state.any_card_busy()
        || state.stroke.is_some()
        || state.drag.is_some()
        || state.marquee.is_some()
        || state.erasing.is_some()
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
    // Return closes the inline editor even if the field itself did not take it.
    if ev.key == Key::Enter && state.editor.is_some() {
        state.commit_editor();
        return true;
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
            // The inline editor owns the plain keys while it is open; Rune sends typing to the
            // focused field first, and anything left over must not switch tools underneath it.
            if state.editor.is_some() {
                return false;
            }
            match text.as_str() {
                // P is the pen's primary key; B stays as an alias for muscle memory.
                "p" | "b" => state.set_tool(Tool::Draw),
                "v" => state.set_tool(Tool::Select),
                "h" => state.set_tool(Tool::Hand),
                "r" => state.set_tool(Tool::Rectangle),
                "d" => state.set_tool(Tool::Diamond),
                "o" => state.set_tool(Tool::Ellipse),
                "a" => state.set_tool(Tool::Arrow),
                "l" => state.set_tool(Tool::Line),
                "t" => state.set_tool(Tool::Text),
                "n" => state.set_tool(Tool::Sticky),
                "e" => state.set_tool(Tool::Eraser),
                "q" => state.toolbar_locked = !state.toolbar_locked,
                "f" => state.fit_to_content(),
                "0" => state.camera.reset(),
                "?" => state.toggle_help(),
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
            // Escape in the inline editor cancels only the editor, creating nothing.
            if state.editor.is_some() {
                state.cancel_editor();
                return true;
            }
            // Cancel any in-flight gesture, then close the topmost open overlay. The pin and
            // close dialogs own Escape first; the popovers follow, then the panels, then zen.
            let dialog_open = state.show_close_confirm || state.pin_preview.is_some();
            state.cancel_gestures();
            if dialog_open {
                state.show_close_confirm = false;
                // When Rune's modal trap did not take the Escape first (nothing was focused
                // inside it), retiring the dialog here is still a key dismissal: return to
                // its Pin button with the keyboard ring, not the pointer's ringless restore.
                if let Some(preview) = state.pin_preview.take() {
                    let ring = tx.by_keys() || tx.focus_visible();
                    tx.focus(format!("card-pin-{}", preview.card_id), ring);
                }
            } else if state.overflow_open {
                state.overflow_open = false;
            } else if state.menu_open {
                state.menu_open = false;
            } else if state.show_help {
                state.show_help = false;
            } else if state.show_settings {
                state.show_settings = false;
            } else if state.zen {
                state.zen = false;
            }
            true
        }
        Key::Function(1) => {
            state.toggle_help();
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
