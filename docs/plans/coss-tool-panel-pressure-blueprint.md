# Blueprint — coss ui restyle, left tool panel, pressure ink

Date: 2026-11-19
Target match: **90%** (critic stops at ≥90 with all must-haves verified)

## Agreed outcome

Doodleverse's Rune front end looks and behaves like a coss ui app: neutral, crisp, compact,
one radius scale, opaque borders, no indigo accent. The top bar only selects tools. Tool
settings live in a fixed floating panel on the left, modelled on Excalidraw's panel but in the
coss theme. Pen ink responds to speed like a real brush: slow strokes are thick, fast strokes
are thin. Shapes gain stroke colour, background fill (hachure / cross-hatch / solid), stroke
width, pressure toggle and opacity.

## User decisions (recorded)

- Left panel controls: **Excalidraw set + fill** — stroke colour, background/fill for closed
  shapes, stroke width presets, pressure on/off, opacity.
- Panel behaviour: **Excalidraw-like** — fixed top-left below the menu island, shown only for
  drawing tools (pen, shapes, text, sticky), hidden for hand/select/eraser.
- Selection: **next drawing only** — panel edits new strokes/shapes; selecting an existing
  object does not expose style editing in this pass.
- Theme: **coss ui**, not Excalidraw's indigo. Reference screenshots govern layout only.
- Target: **90%**.

## Must-haves

1. **coss ui token pass across the whole Rune app** — top bar, menu, overflow, left panel,
   canvas settings, help, minimap, bottom cluster, notices, AI cards, inline editor.
   - Tokens from coss `:root` / `.dark` (see Values below). One radius scale: 10 surfaces,
     8 controls. Borders are translucent black 8% / white 6%. Accent is neutral, not indigo.
   - Light and dark themes both defined; no pure inversion.
   - Hover, pressed, selected, focus and disabled states remain visible in both themes.
2. **Top bar = tool selection only.** Lock stays as the leading mode control (Excalidraw
   reference), then the 11 tools, then nothing. Brush pill is removed. Undo/redo, zoom,
   minimap and board settings stay in the bottom-left cluster.
3. **Left tool-settings panel** — fixed at left 12, top 64, width ≈ 208, card surface,
   radius 10, 1px border, padding 12, section gap 12. Visible for Draw, Rectangle, Diamond,
   Ellipse, Arrow, Line, Text, Sticky; hidden for Hand, Select, Eraser. Contents adapt:
   - Stroke: 6 swatches (current colour ringed).
   - Background: transparent + 5 muted swatches; closed shapes only.
   - Fill: hachure / cross-hatch / solid buttons; closed shapes only.
   - Stroke width: 3 preset buttons (thin 2, bold 4, extra 8); hidden for text/sticky.
   - Pressure: off / on segmented toggle; pen only.
   - Opacity: 0–100 slider with live value; applies to stroke alpha and fill alpha.
   - Text/sticky: stroke colour and opacity only.
4. **Shape fill in the document model and Rune renderer.**
   - `ShapeObject` gains `fill_rgba: [u8;4]` (alpha 0 = transparent) and
     `fill_style: FillStyle { Hachure, CrossHatch, Solid }`, appended with an EOF-tolerant
     postcard visitor so old boards still decode. Schema stays 1.
   - Rune canvas paints solid fill and hatch/cross-hatch inside rectangle, diamond and ellipse;
     strokes paint on top. Hatch spacing tracks zoom and stroke width; line count is capped so
     deep zoom-out stays cheap. Arrow/line ignore fill.
   - Codec/file roundtrip tests cover the new fields and legacy payloads.
5. **Pressure ink.**
   - Mouse simulation is speed-driven and clearly directional: slow → thick, fast → thin.
     Remove the elapsed-time envelope that thickens later samples. Speed EMA responsive.
   - Pressure toggle off yields constant width (hardware pen pressure still honoured when
     present). Default on.
   - Unit tests: slow stroke mean width > fast stroke mean width by a clear margin; pressure
     off gives near-constant widths; legacy behaviour bounds respected.
6. **Verification and evidence.**
   - `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -D warnings`,
     `cargo test --workspace`, `cargo build --workspace --release --locked`.
   - Launch the release app, capture: light and dark chrome, panel for pen/shape/text, and
     slow-vs-fast pen strokes showing thickness difference. Evidence saved under
     `artifacts/blueprint-coss/`.
   - Fresh independent critic scores against this brief; fix and re-critique until ≥90%.

## coss ui values (from coss.com/ui/docs/styling)

Light:
- background/card/popover `#ffffff`; foreground/card-foreground `neutral-800 #262626`
- primary `#262626`; primary-foreground `neutral-50 #fafafa`
- secondary/muted/accent: black 4% (≈ `#f5f5f5` over white)
- muted-foreground ≈ `#606060`
- border black 8% (≈ `#ebebeb`), input black 10% (≈ `#e5e5e5`), ring `neutral-400 #a3a3a3`
- destructive `red-500 #fb2c36`; success `emerald-500 #00bc7d`; warning `amber-500 #fd9a00`;
  info `blue-500 #2b7fff`
- radius 10; controls radius 8

Dark:
- background ≈ `#161616` (neutral-950 95% + white 5%)
- card/popover ≈ `#1b1b1b` (background 98% + white 2%)
- foreground `neutral-100 #f5f5f5`
- primary `#f5f5f5`; primary-foreground `#262626`
- secondary/muted/accent: white 4% over card ≈ `#272727`
- muted-foreground ≈ `#808080`
- border white 6% (≈ `#262626`), input white 8% (≈ `#2a2a2a`), ring `neutral-500 #737373`
- radius 10; controls radius 8

Component anatomy:
- Toolbar: bordered card, p-1, 32–36px ghost icon buttons, active item = primary fill with
  primary-foreground icon; hover = accent. Separators = 1px border token.
- Panel: card surface, 1px border, radius 10, subtle shadow; section labels 12px medium
  muted-foreground, sentence case.
- Slider: 4px accent track, primary knob, muted value text.
- Menus/popovers: popover surface, radius 10, border, larger shadow, 32px rows, accent hover.
- Focus: 2px ring token, offset 1px.

## Assumptions

- coss ui is a React/Tailwind system; this is a native Rune app, so the component anatomy and
  tokens are translated into Rune primitives (Lucent's incompatible-stack rule). No web
  runtime is introduced.
- Inter is not vendored; the system sans stack stays. Weight/size hierarchy follows coss.
- Lock remains the toolbar's leading mode control, matching the supplied Excalidraw reference.
- Board background/grid presets are user content settings and stay available; their default
  colours and the Slate/Ink presets were neutralised so the board reads as coss rather than
  Excalidraw indigo (revised during the build).
- Opacity is encoded as alpha on the existing colour tokens; no separate per-object opacity
  field is added.
- eframe shell keeps compiling; full fill rendering parity there is out of scope.

## Out of scope

- Editing style of an already-selected object.
- Custom colour pickers beyond the existing swatch set.
- Fill for arrow/line or text/sticky backgrounds.
- New document schema version.

## Acceptance evidence a critic can inspect

- Screenshots in `artifacts/blueprint-coss/` for light/dark, each panel variant, and
  slow-vs-fast ink.
- `cargo test --workspace` output including new brush and fill tests.
- Diff of `theme.rs`, `view.rs`, `canvas.rs`, `brush.rs`, `objects.rs` against the brief.
- No indigo tokens left in the Rune chrome; all interactive states present.

## Build record

- Implementation: coss tokens, tool-only bar, left panel, shape fill (model + analytic hatch
  clipping), speed-driven pressure, statement colours, neutral boards. Decisions D36–D43.
- Evidence in `artifacts/blueprint-coss/`: `20-light-pressure`, `21b-light-text-panel`,
  `22b-light-sticky-panel`, `23b-light-select-no-panel`, `24b-light-shape-panel`,
  `25-light-solid`, `27-light-coral-text`, `28-light-sage-note`, `30-dark-pressure`,
  `31-dark-filled-shape`, `32-dark-pressure-off`.
- Gate (final revision): fmt check clean, clippy `-D warnings` clean, 99 workspace tests
  passed, release build finished.
- Critic passes: see below.
