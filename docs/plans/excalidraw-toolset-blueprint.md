# Blueprint — Excalidraw toolset, lock, minimap and bottom-left controls

Status: brief settled by the 2026-11-16 follow-up interview; implementation authorized.
Current in-flight change already adds keybind badges (P/V/H), better icons, wheel zoom and an
object eraser.

## Brief

Turn the Rune shell's toolbar into the Excalidraw toolset and move navigation chrome to the
bottom-left:

1. Tools: Lock (position only), Hand (H), Select (V), Rectangle (R), Diamond (D), Ellipse (O),
   Arrow (A), Line (L), Pen (P), Text (T), Sticky (N), Eraser (E), overflow (⋯).
2. Keybind badge on each tool, Excalidraw style: 10pt muted text bottom-right of the 36×36 box.
3. Lock pins the toolbar in place (no drift or animation). It does not keep the tool active.
4. Middle-click drag pans the board; wheel zooms around the cursor (from the in-flight change).
5. Bottom-left cluster replaces the status pill and the bottom-right zoom pill:
   zoom percentage, board-settings button (board colour + grid), minimap toggle, undo, redo.
6. Minimap toggles a panel above the cluster that shows board content and the visible viewport;
   clicking inside moves the camera there. It stays open until the button is toggled again.
7. The status text (`Local only · in memory · N objects`) is removed.
8. Undo/redo leave the top toolbar and live in the bottom-left cluster.

## Settled decisions (user answers)

- **Shape tools:** add them now, including text and sticky notes. This is a document-model change,
  not only chrome.
- **Lock:** pin toolbar position only.
- **Status:** remove the status pill.
- **Minimap:** panel plus click-to-navigate.

## Proposed document model

- New `ShapeObject { shape: ShapeKind, width, height, style: StrokeStyle }` where `ShapeKind` is
  `Rectangle | Diamond | Ellipse | Arrow | Line`; pose holds the top-left, width/height the box.
  Arrow/Line use the box diagonal for direction (arrowhead at the end point).
- Text and sticky reuse `StatementObject` with a style flag distinguishing a plain text block from
  a sticky note (yellow paper, fixed width).
- New `DocumentCommand::CreateShape { pose, shape }`; text/sticky keep `CreateStatement`.
- Codec gains a `shape` kind tag; snapshot validation and import remapping extended. Older builds
  cannot read new files — acceptable and recorded.
- Renderer and hit testing extended: rectangle/ellipse/diamond outlines from the box, arrow/line
  as a stroked segment with arrowhead, text/sticky as today's statement rendering.

## UI behaviour

- Drag with a shape tool draws a preview from press point to cursor; release commits one shape
  (Shift constrains to a square/circle/45°, matching Excalidraw).
- Text: click places a text block; a small inline editor opens (existing single-line input
  pattern), Enter commits, Escape cancels. Double-click an existing text/sticky reopens it.
- Sticky: click places a fixed-width yellow note with the same editor.
- Eraser: drag deletes whole objects it passes over, one undo step per gesture.
- Lock: a 36×36 toggle at the left of the toolbar before the divider; active state uses
  `selected_bg`; when on, the toolbar cannot move or animate.
- Minimap: 200×140 panel with 8pt radius, board rectangle, object thumbnails, viewport outline;
  click maps to a camera centre. Toggle key is the map button; Esc does not close it (it is not a
  popover), the button toggles.
- Bottom-left cluster: `[zoom %] [board settings] [minimap] [undo] [redo]`, 36 tall, radius 8,
  matching island styling; appears in zen mode like Excalidraw's zoom/undo cluster.
- Removing the status pill drops its notices; important messages use the existing notice pattern
  elsewhere if one exists, otherwise they are simply not shown.

## Acceptance criteria

1. Every tool draws its object and is undoable in one step per gesture.
2. Badges show P/V/H/E/R/D/O/A/L/T/N and match the active tool keys.
3. Lock on = toolbar position fixed; off = current behaviour.
4. Middle-drag and hand-drag pan; wheel zooms at the cursor; panels/cards still scroll.
5. Bottom-left cluster works at 1180×760 and 880×600 without overlap; status pill gone; top
   toolbar no longer holds undo/redo.
6. Minimap toggles, shows content and viewport, click moves the camera, stays open.
7. Board-settings button opens the existing canvas settings panel (board colour and grid).
8. Existing keys and tests keep passing; full gate green.

## Non-goals

- No collaboration, no new file format version beyond an additive kind tag, no shape resize
  handles, no rich text.
- No toolbar dragging; lock only guarantees a fixed position.

## Workflow

1. Model pass: `doodle-types`, `doodle-document`, `doodle-canvas`, Rune canvas rendering and hit
   testing for shapes, text and sticky.
2. Chrome pass: toolbar tools and badges, lock, middle-drag, minimap, bottom-left cluster, remove
   status and top undo/redo.
3. Fresh-subagent QA rounds after each pass; fixes through separate implementers.
