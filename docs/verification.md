# Verification

Last full gate: 2026-11-19, workspace with the Rune front end. A check is listed as passed only if
it ran in this workspace.

## Automated gate

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass |
| `cargo test --workspace --locked` | 99 passed, 0 failed |
| `cargo build --workspace --release --locked` | Pass |

Test distribution: `doodle-types` 10, `doodle-canvas` 24, `doodle-document` 20, `doodle-files` 16,
`doodle-ai` 11, `doodle-render` 5, `doodle-platform` 1, `doodle-app` 4, `doodle-app-rune` 8 (the Rune
shell is also verified through its headless script mode).

What the tests cover:

- Snapshot validation: duplicate IDs, missing parents, non-group parents, cycles, depth, invalid
  geometry and text.
- Envelope: roundtrip, bad magic, truncation, bad checksum, unsupported version/schema/codec/flags,
  oversize payload, empty board.
- Import: ID and parent remapping, origin normalization, wrapper group, atomic write, failed write
  leaves the target untouched.
- Document: every command, local undo/redo, remote-origin updates excluded from history, group
  depth and cycle handling, projection ordering and derived bounds.
- Canvas: camera clamping and anchoring, speed-driven brush pressure, stroke validity, hit clearance.
- Render: grid LOD frames, sanitized settings, cache invalidation.
- AI: request validation, streaming completion, cancellation, UTF-8 chunk-boundary regression,
  alternative cap, pin payload.
- App: placement quantization/overlap, palette and shortcut helpers.

## UI session evidence

Run on the native Windows build, window placed on the secondary display through
`DOODLE_TEST_WINDOW`. Screenshots were captured from the app window; the tracked one is
`docs/images/doodleverse-ui.png`.

| Scenario | Result |
| --- | --- |
| Launch, empty canvas, floating menu/tool/status islands | Verified |
| Draw with synthetic mouse drag | Verified after focusing the window; produced real strokes |
| Draw a stroke manually | Verified; user-drawn line persisted and rendered |
| Brush/select switching, swatch selection | Verified |
| Zen mode (`Alt+Z`) hides all chrome, hint remains | Verified |
| Canvas settings: presets, grid type, spacing, opacity, mark size, line width | Verified |
| Sand preset changes board/grid colors | Verified |
| AI card: ask, stream, complete answer | Verified after fixing UTF-8 chunking |
| Close with unexported board shows confirmation | Implemented and inspected; automated click-through not run |
| Import ghost, commit, undo | Implemented and unit-tested; full dialog roundtrip not yet exercised |
| Pin completed answer to board | Implemented; modal inspected, automated pin not yet exercised |
| Marquee, shift-toggle, move, group, ungroup, delete, undo/redo | Unit-tested; not all exercised through the GUI |

The first synthetic-input attempt produced no strokes because the window was not foreground; after
focusing it, drags and clicks worked. That was a harness issue, not an app bug.

## Rune shell evidence

The Rune front end was driven headlessly with its own script/snapshot mode (`--script`,
`--snap`), so each scenario below is a real frame of the native app, not a mock. Test scripts
and images live in the untracked `scratch/` directory; the tracked screenshot is
`docs/images/rune-window.png`.

| Scenario | Result |
| --- | --- |
| Launch, islands, empty-board hint | Verified |
| Brush strokes with simulated pressure, point/object counters, unsaved state | Verified |
| Marquee selection, group, selection outlines and corner handles | Verified |
| Undo then redo through the tool island | Verified; object counts changed 3 → 2 → 2 |
| Hand-tool drag pan and zoom-in button | Verified; grid origin and zoom badge moved |
| Wheel pan, `Ctrl`+wheel zoom and pinch magnify handlers | Superseded by plain-wheel zoom; `Ctrl`+wheel and pinch verified |
| Canvas settings: panel scroll, light/dark, six board presets, grid kinds, all sliders | Verified; wheel over the panel scrolls it to the brush section |
| Dark board swaps the default ink so new strokes stay readable | Verified |
| Zen mode closes open panels and hides every island | Verified |
| AI card: open, type, generate, stream, answer and provider label | Verified |
| Pin modal and pinning a completed answer | Verified; pin created a statement on the board |
| Export through the scripted file dialog | Verified; 2398-byte valid `.doodleverse` written |
| Import through the scripted file dialog, ghost placement, commit | Verified; "Imported 2 objects" |
| Real window on the secondary monitor, app icon in the title bar | Verified by screenshot |
| `WM_CLOSE` on a clean board quits the process | Verified |
| Close with unsaved content asks first, and its Export/Discard/Cancel paths | Implemented and inspected; not clicked through in a real window |

Headless commands used:

```bash
cargo build -p doodle-app-rune
./target/debug/doodleverse-rune.exe --size 1180x760 --script scratch/scroll.json --snap scratch/rune-scroll.png
```

The tool-keybadge, icon, wheel-zoom and eraser runs used the scripts in `scratch/kb/`
(`wheel-zoom.json`, `wheel-settings.json`, `wheel-card-880.json`, `pinch.json`,
`hand-pan.json`, `eraser-one.json`, `eraser-batch.json`, `light.json`, `dark.json`), each
rendered with `--at mark:NAME --snap` and `--tree`.

Real-window run on a second monitor (window is hidden only in the console sense; winit still shows
the app):

```powershell
$env:DOODLEVERSE_WINDOW_POS = '-1920,0'
Start-Process target\debug\doodleverse-rune.exe -WindowStyle Hidden
```

The same binary embeds `assets/app-icon.png` through `rune::set_app_icon`, which restored the
Windows title-bar, taskbar and Alt-Tab icon that the first Rune builds were missing.


## Excalidraw chrome, tokens and QA rounds

The top-bar redesign and the token/icon pass were driven headlessly at 1180×760 and 880×600 in
light and dark, then tested by nine independent fresh-subagent QA rounds. After each round a
separate implementer fixed the confirmed findings and a new tester re-ran the app.

| Area | Result |
| --- | --- |
| Top bar: menu island top left, centred horizontal toolbar, three-dot overflow top right | Verified at both sizes; no overlap; Ask Dibby and Export move into the overflow below 1160pt |
| Tokens at the time: indigo accent, radius-8 islands, 36×36 tool boxes, 22×22 swatches, `#121212` dark canvas (superseded by the coss pass below) | Verified in light and dark screenshots |
| Menu and overflow popovers mount and unmount instantly; scrim and Escape dismiss | Verified; no ghost rows one frame after dismissal |
| Settings panel scroll with sticky header and close; help panel sticky header and Escape | Verified at 880×600 |
| Canvas wheel zooms over the empty-board hint; wheel over panels and AI cards scrolls them, not the board | Verified: cursor-anchored zoom, settings/help scroll and card-answer scroll all leave the zoom badge unchanged |
| AI card with wrapped long question: card inside the window, footer clickable, answer scrolls fully | Verified at both sizes |
| Pinned answers: note sized to its text, clear of chrome, never under the card | Verified for short and long answers, panned and unpanned |
| Presses that open or dismiss a popover never start a canvas stroke | Verified at 0.02–0.1s delays and a normal tap still draws one stroke |
| Chrome stays fixed when a card opens or closes | Verified by matching island bounds at 0.15s and 0.3s |
| Accessibility: checked states, slider role/value/actions, named and disabled buttons, dialog roles, scrims hidden | Verified in the element tree |
| Pin and close dialogs trap Tab/Enter, hide the background from accessibility, and restore focus with a visible ring | Verified for the pin dialog; the close dialog shares the code but cannot be raised headlessly |
| Disclosure buttons report expanded/collapsed; disabled buttons consume presses instead of drawing | Verified |
| First frame matches the viewport; completed answers start at their first line | Verified |
| Dark theme strokes, undo/redo, counts, group/ungroup, zen | Verified |
| Tool keybinds: `P` pen with `B` alias, `V`, `H`, `E`; 10pt key badges and name+key tooltips | Verified in the element tree (`checked` state, `help` "Draw P" …) and in light/dark frames at both sizes |
| Excalidraw-style pen, cursor, hand, eraser and 16px-grid overflow icons | Verified in magnified crops of the frames |
| Whole-object eraser: one drag over one of two lines removes only that line; a drag across both removes both; one undo restores the whole gesture | Verified: counts 2 → 1 (undo 2), and 2 → 0 (single undo 2) |
| Plain wheel zooms at the cursor; pinch still zooms; hand drag still pans | Verified: a 200pt horizontal stroke under the pointer stayed anchored while zoom went 100% → 125%; pinch 100% → 130%; hand drag moved the grid at 100% |

Defects found and fixed across the rounds: answer text clipped and not scrollable, pinned note
off-screen and later overlapping the card, Export unreachable below 1160pt, missing names and
states on swatches/sliders/buttons, help Close below the fold, Escape not closing overlays, card
overflow at 880×600, "1 objects" grammar, first-frame press leak, chrome remount animation,
fading popover ghosts, wheel dead over the empty-board hint, and oversized pinned notes.

Known residuals: an extremely long pinned answer narrowed to 200pt can still exceed the free board
height and falls back to the least-overlapping placement; a group container counts as one object
in the status line (two strokes grouped report three objects). The release build was launched in a
real window after the token pass: title bar and taskbar icon present, top bar and islands render as
in the headless frames, and `WM_CLOSE` on an empty board exits the process cleanly.

## Pending verification: tool cursors, eraser feedback, zoom-out cost

Implemented on 2026-11-16 after the user's own run, and deliberately not re-tested at the user's
request ("don't do testing, just implement"). Only the compile gate and the affected unit tests
ran through the implementer:

- Per-tool canvas cursors: default arrow (select), grab/grabbing (hand and middle-drag), crosshair
  (shape tools), custom pen and sticky-note PNG cursors, I-beam (text), hidden OS pointer with a
  canvas-drawn translucent disc and 48pt tail (eraser).
- Wheel up zooms in, wheel down zooms out at the pointer, in both scroll conventions; the
  Windows/Linux pinch path keeps the same direction.
- Scene batching: grid lines, dots and pluses are one path per kind (`stroke_path`/`fill_path`),
  stroke segments are bucketed by half-point width and stroked per bucket, shape outlines are one
  path each, and the minimap is capped to 400 objects / 24k points. The accumulated-`Shape` clone
  per mark is gone; grid LOD is 16pt minimum spacing with a 3,000-mark cap and fades out below 5%
  zoom.
- Low-zoom freehand fidelity: accepted samples are refined with a uniform Catmull-Rom spline
  before geometry is produced, so far-apart samples at deep zoom no longer render as straight
  chords.

Not yet verified: real-window cursor images (the headless runner has no OS cursor), perceived frame
rate at each zoom level on the user's machine, middle-drag cursor state, and the visual feel of the
eraser tail and spline curvature. The user will run these checks when they return to the app.

## Known verification gaps

- **Hardware pen pressure** cannot be verified without a tablet. The Windows pointer probe compiles
  and is installed at startup; the fallback path (`None` → simulated pressure) is what the UI
  session exercised. No claim is made that a real pen was tested.
- **Export/import through the native dialogs of the eframe shell** was not automated. In the Rune
  shell the same paths were exercised with scripted dialog answers; the real Win32 dialog itself was
  not clicked through. Codec, validation, remapping, and atomic write are covered by tests.
- **Pin modal and statement placement** were inspected, not clicked through end to end.
- **Multi-machine, networking, and WebRTC** do not exist in this build and were not tested.
- **Real AI providers, ChatGPT sign-in, BYOK, ACP, and the credential vault** do not exist and were
  not tested.
- **Non-Windows platforms** were not built or run here. The pen probe has non-Windows stubs.
- **Packaging, signing, and installers** were not produced.

## Reproduce

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked
```

For a visual smoke test without covering the primary display:

```powershell
$env:DOODLE_TEST_WINDOW = "-1920,40,1600,900"
cargo run --release -p doodle-app          # eframe shell
$env:DOODLEVERSE_WINDOW_POS = '-1920,0'
cargo run -p doodle-app-rune               # Rune shell
```

Close the app before rebuilding; the running executable locks the release binary on Windows.

## coss ui tool panel, shape fill and speed-driven pressure (2026-11-19)

Implemented against `docs/plans/coss-tool-panel-pressure-blueprint.md`. This section records
what ran in the workspace; visual review of the panel variants and slow-vs-fast ink is the
orchestrator's screenshot pass.

| Area | Result |
| --- | --- |
| Tokens: coss neutral light/dark palettes, radius 10 surfaces and 8 controls, translucent borders, neutral primary instead of indigo | Defined in `doodle-app-rune/src/theme.rs`; both palettes compile and every constructor uses the shared radius constants |
| Top bar: tool selection only; lock is the leading control; brush pill removed | Verified by code inspection and workspace build; the toolbar keeps the menu, overflow and bottom cluster |
| Left tool-settings panel: stroke, closed-shape background/fill, width presets, pen pressure switch, opacity; hidden for Hand/Select/Eraser/zen | Panel visibility and contents are state-driven in `app.rs`/`view.rs`; wheel routing and board/card insets use the measured panel rect |
| Shape fill: solid, hachure and cross-hatch for rectangle, diamond and ellipse; stroke on top; alpha 0 skips fill | `canvas.rs` fills before stroking; preview and committed objects share the path; arrow/line ignore fill |
| Hatch clipping: polygon edge intersections for rect/diamond, analytic line/ellipse roots for ellipse; spacing tracks zoom and width; 240-line cap | Unit tests cover both intersection helpers and the outside-line cases |
| Pressure: speed EMA only, slow thick and fast thin; simulation can be switched off; hardware pen still wins | `slow_strokes_are_thicker_than_fast_strokes`, `pressure_off_keeps_a_constant_width`, `pressure_off_still_honours_hardware_pressure`, `non_finite_settings_are_sanitized` pass |
| Codec: new fill fields round-trip; legacy payloads ending after `style` or after `flip_y` decode with transparent hachure | `round_trip_preserves_shape_fill`, `snapshot_written_before_shape_fill_still_decodes`, and the existing direction/statement legacy tests pass |
| Neutral board: light `#ffffff`/`#d4d4d4` grid, dark `#161616`/`#525252`, Slate/Ink presets neutral | `toggle_theme` and `BOARD_PRESETS` inspected; light and dark screenshots show a neutral board under the neutral chrome |
| Statement colour: Text uses the stroke ink, Sticky uses one of six note tints with luminance-derived text ink, opacity applies to both | Inline editor and committed statement share the same colour path; light screenshots show coral text and a sage note |
| Swatch ring: light ring on dark ink, dark ring on pale ink, 2pt when selected | `theme::swatch` luma threshold; visible in both themes |
| Pressure tuning: `speed_ema * 0.7 / min_distance`, 300 px/s normalisation, `1.2 - 0.8 * n` clamped `0.4..=1.2`, default sensitivity 0.8 | Repeated real-window drags at 200 ms and 2400 ms show a clear thick/slow vs thin/fast difference in light and dark, and equal widths with Pressure Off |
| Gate | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, and `cargo build --workspace --release --locked` run in this workspace |

Not claimed here: the subjective feel of the hatch patterns and pressure curve. Those are checked
by the orchestrator's screenshot and critic pass.

## coss component, icon and zoom pass (2026-11-19)

Driven headlessly at 1180×760 in dark (the default) with `--script`/`--snap`, plus unit tests
for the icon glyphs.

| Area | Result |
| --- | --- |
| Launch opens dark: `#161616` card chrome, `#161616` board, neutral grid | Verified in `artifacts/blueprint-coss/50-farzoom-aa.png` and `54-panel-labels.png` |
| Toolbar follows coss Toolbar: `rounded-xl` card, p-1, 32pt ghost buttons, 1px separators | Verified in the same frames |
| Slider follows coss Slider: 4pt input track, primary indicator, 16pt white thumb with dark background border, 1.2 press scale | Verified; thumb glides and grows while dragging |
| Switch follows coss Switch: 18×30 primary/input track, 16pt thumb, spring | Verified for Pressure on and off |
| Field labels use the coss 14/500 primary ink with a 14/400 muted value | Verified in the panel crop |
| Menus, dialogs, settings/help/tool panels enter with `Transition::scale` 0.96–0.98 plus fade | Verified in the element tree; frames sampled mid-entry show the scaled state |
| Icons: Material Symbols Rounded (filled) through Rune's SVG engine, 25 glyphs | Verified per glyph with `rune::text::icon_coverage`; toolbar and cursors inspected in crops |
| Diamond tool glyph is a rhombus (rotated set square), not the faceted gem | Verified in `53-toolbar-crop.png` |
| Pen and sticky cursors rasterised from the same glyphs with a white halo; hotspots on the tip and note corner | Verified by regenerating the PNGs and reading them back; hotspots `[7.5,24.5]` and `[6.3,6.3]` |
| Far-zoom AA: strokes below one screen point drop to one 1pt antialiased polyline | Verified at 3% and ~17% zoom in `50-farzoom-aa*.png` and `51-far/out-crop.png`; no staircase |
| Taper unchanged at normal zoom: filled outline run with round end discs | Verified in `41-dark-taper.png` after the change |
| Selected tool, grid kind and theme presets use coss's `bg-input/64` pressed wash with primary ink, radius 8 segmented controls | Verified in `artifacts/critic/after-settings.png` |
| Canvas settings and Shortcuts & about open without panicking (the two `icon("x")` calls were a regression) | Verified headlessly; settings frame in `artifacts/critic/after-settings.png` |
| Light theme reachable from Canvas settings → Paper light | Verified in `artifacts/critic/light-reachable.png` |
| Menu/overflow popovers use coss `p-1`, stacked 28pt rows, radius 8 | Verified in `artifacts/critic/menu-crop.png` |

Gate for this pass: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
--locked -- -D warnings`, `cargo test --workspace --locked` (101 passed, 1 ignored — the cursor
regeneration test), `cargo build --workspace --release --locked`.

## Critic-round-2 fix pass (2026-11-19)

Headless pass at 1180×760, dark default, `artifacts/round2/`.

| Fix | Evidence |
| --- | --- |
| Tool panel no longer smears when switching Draw → Rectangle | `ghost.png` at 0.15 s after the click shows the rectangle body only |
| Overflow trigger keeps its ink and pressed wash while the menu is open | `ovopen.png` |
| Clicking the switch thumb's centre toggles Pressure off (row owns the press) | `switch.png` (off) and `card.png` (on) |
| Slider thumb holds the 1.2 drag scale and reports 41% mid-drag | `drag.png` |
| Tooltip uses the coss popover surface, radius 6, 12pt text, 0.98 fade | `tip.png` |
| AI card surface uses the dialog radius 16 | `card.png` |
