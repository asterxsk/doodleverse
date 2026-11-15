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

## UI shell

The app draws one `CentralPanel` for the canvas and floating `Area` islands above it: menu,
vertical tools, actions, and bottom status/zoom. `Alt+Z` hides every island and leaves a single hint;
the same shortcut restores them. Settings and help are egui windows; destructive close is a modal.

The canvas owns input precedence: pending import ghost, active stroke, active move, marquee, then
idle shortcuts. Text fields consume keyboard input before global shortcuts.

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
speed/progress envelope.

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
| Theme/layout | `doodle-app/src/app/chrome.rs`, `panels.rs` |
| Native input | `doodle-platform`, then map into `RawSample` |
