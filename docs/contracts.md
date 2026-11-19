# Contracts

These are the boundaries other code, files, and future transports are allowed to rely on. If a
change breaks one of them, it needs a version bump or a documented decision.

## 1. Statelessness

- A board exists only in RAM for the lifetime of one process.
- The app never autosaves, restores a session, keeps a recent-files list, or writes logs containing
  board content.
- The only persistent app output is a file the user explicitly exports through a native dialog.
- Closing with unexported content requires a confirmation; "Export, then close" exports first.

## 2. Document schema

`DOCUMENT_SCHEMA_VERSION = 1`.

An object is a `StoredObject`:

```rust
struct StoredObject {
    id: ObjectId,
    parent: Option<ObjectId>,
    order_key: i64,
    pose: Pose { x: f64, y: f64 },
    kind: ObjectKind,
}

enum ObjectKind {
    Stroke(StrokeObject { style, geometry }),
    Group,
    Statement(StatementObject { text, width, height, style, kind }),
    Shape(ShapeObject {
        shape, width, height, style, flip_x, flip_y, fill_rgba, fill_style,
    }),
}
```

Invariants:

- IDs are unique inside a snapshot.
- `parent` refers to an existing `Group` in the same snapshot; groups cannot contain themselves.
- Group nesting is at most `MAX_GROUP_DEPTH` (64).
- Coordinates and sizes are finite and within `MAX_ABS_COORDINATE` (1e9).
- Stroke geometry version is `GEOMETRY_SCHEMA_VERSION` (1); every point has finite `x`, `y` and a
  finite width in `0.05..=400.0`; a stroke has 1..=`MAX_STROKE_POINTS` points.
- Statement text is at most `MAX_OBJECT_TEXT_BYTES` (64 KiB) of UTF-8; width is 40..=4000, height
  30..=4000.
- `ShapeKind` is rectangle, diamond, ellipse, arrow or line. Closed shapes keep a positive box;
  arrow and line may be zero-extent on one axis and carry `(flip_x, flip_y)` to replay the drag
  direction. `fill_rgba` alpha 0 means transparent; `fill_style` is `Hachure` (default),
  `CrossHatch` or `Solid`. Open paths ignore fill.
- `ShapeObject`'s fill fields are appended after the direction flags. The deserializer stops
  cleanly at end-of-buffer and defaults to transparent hachure, so payloads written before the
  fill existed still decode under schema version 1.
- Render order is `(order_key, id)`, independent of CRDT map order.

## 3. Portable file

Magic: `DVRSE\0\0\0` (8 bytes).

Little-endian header, 24 bytes total:

| Offset | Size | Field |
| --- | --- | --- |
| 0 | 8 | magic `DVRSE\0\0\0` |
| 8 | 2 | envelope version (`1`) |
| 10 | 2 | document schema version (`1`) |
| 12 | 2 | codec (`1` = postcard) |
| 14 | 2 | flags (`0`; any other value is rejected) |
| 16 | 4 | payload length in bytes |
| 20 | 4 | CRC32 of payload |
| 24 | N | postcard-encoded `BoardSnapshot` |

Decode order is strict and side-effect free:

1. Total size within `MAX_IMPORT_BYTES` + header.
2. Header present, magic exact.
3. Envelope version supported; codec supported; flags zero.
4. Payload length within limit and fully present (trailing bytes are not part of the payload).
5. CRC matches.
6. Payload decodes as `BoardSnapshot` with the schema version in the header.
7. `BoardSnapshot::validate` passes.
8. Payload is non-empty and total stroke geometry is bounded.
9. IDs are remapped; parents are remapped; bounds are normalized to the origin.

Nothing reaches the live document until all steps pass. A malformed file cannot partially import,
cannot reuse an existing ID, and cannot inject CRDT history.

Export validates the snapshot before writing. Writes use a hidden sibling temp file, `sync_all`,
then rename; on any error the temp file is removed and the previous export is left intact.

## 4. Import semantics

- First import into an empty board commits at the origin.
- Later imports enter placement mode: the decoded payload is shown as a ghost translated by a
  quantized cursor offset. Overlap with any occupied top-level bounds is rejected.
- A committed import is wrapped in one new group and applied as one local command, so one undo
  removes the whole import.
- Imported objects keep their relative layout; identity and provenance are not preserved by design.

## 5. Commands and undo

`DocumentCommand` is the only mutation surface: `CreateStroke`, `CreateStatement`, `MoveObjects`,
`DeleteObjects`, `GroupSelection`, `Ungroup`, `CommitImport`, `Undo`, `Redo`.

- A command runs in one local-origin transaction and returns `DocumentOutcome`.
- Local undo records field-level `ObjectDelta` before/after values, capped at `MAX_UNDO_STEPS`.
- Redo is cleared by a new local command.
- Remote-origin transactions never enter local history.
- No-op commands and invalid selections produce errors, not silent partial changes.

## 6. Projection

`BoardProjection` is the read model:

- Objects are sorted by `(order_key, id)`.
- `parent` and `depth` are derived; cycles are cut and depth is capped.
- `bounds` for a group is the union of visible descendant leaves.
- `top_id` identifies the top-level ancestor used by selection, delete, move, and occupancy.
- `content_bounds` and `occupied_regions` ignore empty groups.

The UI must not depend on CRDT iteration order, map keys, or internal yrs types.

## 7. AI boundary

- `GenerationRequest` carries one non-empty question (≤ `MAX_QUESTION_BYTES`) and an optional
  instruction (≤ 4096 bytes). It never carries board content, paths, peer IDs, or history.
- `AiProvider` streams `Started`, `TextDelta`, `Completed`, `Failed`, or `Cancelled` events.
- Dropping `GenerationHandle` requests cancellation; completion and failure are terminal.
- A card may keep up to `MAX_AI_ALTERNATIVES` completed answers locally.
- Pinning writes exactly one selected answer as a `StatementObject`; no draft, provider metadata,
  or credential is written to the board or file.
- The shipped provider is offline and labeled as a demo. No credential is stored anywhere.
- Real providers must keep the same boundary and read credentials only from an OS vault.

## 8. Appearance and camera

- Camera and appearance are local, never document state, never exported, and never synchronized.
- Zoom is clamped to `MIN_ZOOM..=MAX_ZOOM` (0.02..=64) and anchored at the cursor.
- Grid settings are sanitized before rendering; custom board/grid colors are allowed.
- Theme, grid, and brush settings are session-only.
- Default ink follows the board: flipping to a dark board swaps the untouched default ink from
  dark to light (and back); a color the user chose is never rewritten.
- Both app shells (`doodle-app`, `doodle-app-rune`) share these rules; appearance never crosses
  into the document.

## 9. Input

- `P` (with `B` as an alias) / `V` / `H` / `E` switch pen/select/hand/eraser; `Alt+Z` toggles
  zen mode; `Esc` cancels the active interaction, including an erase drag before it commits.
  Entering zen closes open panels so no island or modal is left floating.
- The Rune shell zooms on the wheel around the pointer (plain wheel, `Ctrl`/`Cmd`+wheel, or
  pinch); `Shift`+wheel pans horizontally; dragging with the Hand tool pans. The settings and
  help panels and local AI cards keep the wheel while the pointer is over them, so the board
  never zooms underneath.
- The eraser removes whole objects touched by its small world-space radius. One drag collects
  its objects and commits one `delete_objects` batch, so a single undo restores all of them.
- Text input owns the keyboard while a text field is focused; global shortcuts must not fire then.
- Pen pressure is `Option<f32>` in `0.0..=1.0`. `None` means "no pressure source" and selects the
  simulated mouse response. A hardware source must not report a constant fake value.
- Simulated pressure is speed-driven and directional: slow samples produce thick ink, fast samples
  thin ink. There is no elapsed-time envelope. `BrushSettings::simulate_pressure = false` makes a
  mouse stroke constant-width; hardware pen pressure is still honoured when present.
- Windows pressure is read from `WM_POINTER`/`GetPointerPenInfo`; the subclass forwards every
  message unchanged.

## 10. Networking (reserved, not shipped)

- Wire protocol version is reserved at `1`; no transport exists in this build.
- The intended topology is a full mesh of at most `MAX_ROOM_PEERS` (8) direct WebRTC peers.
- No TURN, relay, or server-routed board traffic is permitted.
- When transport lands, it must carry `encode_full_update_v1` output through
  `apply_remote_update_v1`; it must not bypass commands or projection.

## 11. Error and evidence rules

- Errors are typed enums (`FileError`, `DocumentError`, `ProviderError`, `SnapshotError`); the UI
  shows an actionable message and never panics on user input.
- Tests use synthetic fixtures and sentinel strings. No real board content, prompt, answer, or
  credential belongs in Git, logs, or screenshots.
- A check may only be reported as passed if it actually ran. See
  [verification.md](verification.md).
