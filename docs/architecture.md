# Architecture

Doodleverse is a native Rust workspace with one rule: the shared board document is the only
authority, and every layer between input and pixels is deterministic. The app is stateless; an
exported file is the only thing that outlives a session.

## Layers

```
doodle-app        eframe/egui shell, floating chrome, canvas view, AI cards, dialogs
   |
doodle-platform   native file dialogs, window placement, Windows pen pressure
doodle-files      portable envelope: encode, decode, validate, remap, atomic write
doodle-ai         AI card state, provider trait, offline stream, mascot animation
doodle-render     grid frames, cached stroke meshes
doodle-document   yrs CRDT, commands, projection, local undo, snapshots
doodle-canvas     camera, brush pipeline, stroke geometry, hit testing
   |
doodle-types      IDs, geometry, objects, commands, limits, version constants
```

Dependencies point inward. `doodle-types` has no UI, IO, CRDT, or platform dependency, so the
document contracts can be tested without a window. `doodle-document` hides yrs behind commands and
a projection; the UI never reads CRDT internals.

## Data flow

```
pointer samples ──> brush ──> DraftStroke ──> CreateStroke command ──> BoardDocument (yrs)
                                                                         │
window size, camera, stroke cache ───────────────────────────────────────┤
                                                                         ▼
                                                            BoardProjection
                                                                         │
                                                          egui canvas painter
```

1. `canvas_view` collects pointer samples, asks `doodle_platform::pen_pressure()` for hardware
   pressure, and feeds the brush.
2. On release, the resolved stroke becomes a `CreateStroke` command. The command is the only way
   canvas content enters the document.
3. `BoardDocument` applies the command in a local-origin yrs transaction, records an `ObjectDelta`
   for local undo, and bumps its revision.
4. `project()` reads the CRDT, cuts cycles, caps group depth, derives bounds, and returns objects
   sorted by `(order_key, id)`.
5. The app paints from the projection and caches screen-space stroke meshes per revision and
   camera. Draft strokes are painted from the brush directly.

## Document model

- Objects are strokes, statements, or groups. Every object has an `ObjectId`, optional parent,
  `order_key`, and `Pose` (origin offset).
- Leaves own local geometry. A group owns no geometry; its bounds are the union of its descendant
  leaves.
- `top_id` is the nearest ancestor with no parent. Selection, deletion, occupancy, and move
  expansion operate on top-level objects, matching the product's "import arrives as a group"
  model.
- Projection is defensive: it rejects more than `MAX_OBJECTS`, cuts parent cycles, and caps depth
  at `MAX_GROUP_DEPTH`. A malformed document cannot hang the UI.

## Local undo

Undo is a stack of `ObjectDelta { id, before, after }` produced only by local commands.
`ORIGIN_REMOTE` transactions are never recorded, so applying a peer update cannot pollute the local
history. Redo clears on a new local command. The history is bounded by `MAX_UNDO_STEPS`.

This is deliberately not `yrs::UndoManager`: the CRDT undo manager would either undo remote edits
or need scope plumbing this build does not have.

## Files

Export materializes a `BoardSnapshot` from the projection, validates it, and writes the envelope.
Import decodes the whole envelope, validates checksum and schema, validates every object, remaps
all IDs (including parents), normalizes content to the origin, and returns an `ImportPayload`.
The live document is untouched on every error path. Commit wraps the payload in a group and adds it
as a single `CommitImport` command, so import is one undo step.

Writes go to a sibling temp file, are flushed and synced, then renamed over the target. A failed
write cannot truncate the previous export.

## Camera and rendering

`Camera` stores world offset and zoom, clamps zoom to `MIN_ZOOM..=MAX_ZOOM`, and maps screen to
world. Zoom is anchored at the cursor. Panning is an offset change, not a document command.

Rendering is immediate mode. Strokes are cached as triangle meshes keyed by revision, viewport, and
zoom; a new revision or camera invalidates the cache. The grid is generated from a bounded
level-of-detail frame so it neither aliases nor allocates unbounded dots at high zoom.

## UI shells

Two front ends sit on the same crates. `doodle-app` is the original eframe/egui shell;
`doodle-app-rune` is the current UI, built with Rune at revision
`958743308f36fdf1c568c513badc96ddb0c7f040`.

Both draw one full-window canvas layer and absolutely positioned islands above it. The Rune
shell now uses coss ui's neutral token set and component anatomy, dark by default and light
from Canvas settings: a board menu island top left, a centred toolbar that only selects tools
(lock, then the eleven tools), a three-dot overflow top right, and a left tool-settings panel
vertically centred beside the board for the active drawing tool, entering with coss's
scale-and-fade motion. Menus, dialogs and the settings/help panels share that motion; the
slider, switch, toolbar, buttons and menu rows follow the coss specs in D47. The panel carries
stroke swatches,
closed-shape background and fill style, stroke-width presets, the pen pressure switch and
opacity; Sticky replaces the stroke row with six note tints and Text colours statements with
the stroke ink. It is hidden for Hand, Select, Eraser and zen mode. A bottom-left cluster holds zoom,
canvas settings, the minimap and undo/redo. `Alt+Z` hides every island, closes any open panel,
and leaves one hint; the same shortcut restores them. Settings is a fixed-height column panel
with a sticky header and close button, help is a floating panel, and close/pin are centered
modals.

The Rune shell keeps its own small state machine in `app.rs` (`DoodleApp`) mirroring the
eframe state, with `view.rs` building the element tree, `canvas.rs` drawing the grid, shape
fills and strokes into one `Shape`, `cards.rs` rendering AI cards, and `theme.rs` holding the
light/dark palettes. Input precedence matches the eframe shell: pending import ghost, active
stroke or erase gesture, active move, marquee, then idle shortcuts. Text fields consume keyboard
input first. The canvas zooms on the wheel around the pointer, unless the pointer is over the
tool panel, settings or help panel or an AI card, whose scroll views take it; `Shift`+wheel pans
horizontally. Island roots are hit-testable so clicks never fall through to the board.

Closed shapes paint their interior before the stroke. Solid fill is one filled polygon; hachure
and cross-hatch are diagonal line families clipped analytically in screen space (polygon edge
intersections for rectangles and diamonds, line/ellipse roots for ellipses). Spacing tracks zoom
and stroke width, and the line count is capped so deep zoom-out stays cheap. Open paths ignore
fill. The board background, grid ink and the Slate/Ink presets are neutral in both themes, and a
sticky note picks light or dark ink from its own fill luminance so every note tint stays legible.

Rune has no per-vertex canvas mesh, so a pressure stroke is emitted as one filled outline per
run, with a left offset chain forward and a right offset chain back, plus a round disc at each
end and at each sharp turn. The outline interpolates the measured width point by point, so a
speed taper reads as one continuous edge instead of a ladder of stroked capsules. Zoomed far
out, where every measured width lands under a screen point, the run drops to a single
antialiased 1pt polyline with round caps; the same treatment a vector editor gives a
scaled-down line, which fades smoothly instead of stepping. The segment budget is capped at 200k.
The pointer cursor for the pen and sticky tools is rasterised from the same Material glyphs as
the toolbar, with a white halo so it reads on any board tint.
File dialogs use Rune's `Tx::open_file` / `Tx::save_file` (feature `native-file-dialog`), so
headless runs can answer them with scripted `file-dialog` events. The app icon is set with
`rune::set_app_icon` from the same `assets/app-icon.png` the eframe shell embeds.

## AI boundary

`doodle-ai` defines `AiProvider`, `GenerationRequest`, `GenerationHandle`, and `ProviderEvent`.
Generation runs off the UI thread and streams `TextDelta` chunks into the card. Dropping the handle
requests cancellation. `MockProvider` is deterministic, offline, and labeled as a demo.

The boundary carries one question and an optional short instruction. It never carries board
content, file paths, peer IDs, or conversation history. Pinning copies exactly one completed
answer into a statement object; drafts and alternatives stay in card state.

## Pen pressure

Windows pen input is read by subclassing the window procedure and handling `WM_POINTERDOWN`,
`WM_POINTERUPDATE`, `WM_POINTERUP`, and `WM_POINTERLEAVE`. `GetPointerType` filters to pens;
`GetPointerPenInfo` yields 0..1024 pressure, normalized to 0..1000 and stored atomically. The app
polls it while sampling. Non-Windows targets and mice return `None`, which selects the simulated
speed response. Simulation is directional and has no elapsed-time term: a responsive speed EMA
maps slow samples to thick ink and fast samples to thin ink. The pen panel's pressure switch
turns simulation off for a constant-width mouse stroke; hardware pen pressure still wins when
present.

## Determinism and limits

- Object order is always `(order_key, id)`.
- Import remaps every identity and normalizes bounds to the origin.
- Every limit lives in `doodle-types::limits`; enforcement is at the command, projection, or file
  boundary, not scattered through the UI.
- Version numbers for app, envelope, schema, geometry, and (reserved) wire protocol are separate
  constants.

## Where to change what

| Change | Start here |
| --- | --- |
| New board object type | `doodle-types::objects`, `DocumentCommand`, projection |
| New canvas tool | `doodle-app/src/app/canvas_view.rs`, `doodle-canvas` |
| New file field | envelope version bump, `doodle-files`, snapshot validation |
| New AI provider | implement `AiProvider` in `doodle-ai`; do not widen the boundary |
| Theme/layout (eframe) | `doodle-app/src/app/chrome.rs`, `panels.rs` |
| Theme/layout (Rune) | `doodle-app-rune/src/view.rs`, `theme.rs` |
| Native input | `doodle-platform`, then map into `RawSample` |
