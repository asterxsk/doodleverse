# Doodleverse — decisions made while the user was away

Autopilot run: 2026-11-15. Request: implement `IMPLEMENTATION_PLAN.md` end to end, using the
laptop display as a scratch pad for testing. User unavailable for questions; every decision made
without them is recorded here.

## Decisions

### D01 — eframe 0.37 instead of a hand-rolled winit + egui-wgpu shell
- **Because:** the plan asks for a native Rust whiteboard with a GPU surface. eframe 0.37 on wgpu
  already owns the winit event loop, wgpu device, surface, and egui renderer; a custom shell would
  re-implement that plumbing and add a large failure surface for no product difference.
- **Alternatives:** winit + egui-wgpu + egui-winit directly (plan's literal wording); a pure wgpu
  renderer with egui overlays on top.
- **Reversibility:** moderate — app shell is one crate; canvas/render/document crates do not depend
  on eframe.
- **Risk if wrong:** eframe 0.37 is newer than the plan assumed and its API has drifted
  (`App::ui`, `Panel::top/bottom`, `Context::global_style`); worked around and verified by build.
  This supersedes plan decision D03.

### D02 — Local undo as command deltas, not CRDT undo
- **Because:** yrs' undo manager would capture remote edits too, and this build has no peers yet.
  Capturing before/after `ObjectDelta` per local command gives exact, predictable undo and redo,
  and remote-origin transactions are never recorded.
- **Alternatives:** `yrs::UndoManager` (would need remote-scope plumbing); snapshot-per-command
  (too much memory at 100k objects).
- **Reversibility:** cheap — history sits behind `BoardDocument`.
- **Risk if wrong:** when networking lands, remote changes may interleave with a pending redo
  stack; redo is cleared on any local command but not on remote updates. Revisit before P02/P07.

### D03 — Portable file carries a validated board snapshot, not a CRDT update blob
- **Because:** the requirement is a portable, self-contained `.doodleverse` file that any copy of
  the app can open; visible geometry (strokes, statements, groups) is the durable content. Encoding
  a yrs update couples the file to a CRDT implementation version and can smuggle deleted history.
- **Alternatives:** encode `encode_full_update_v1` bytes inside the envelope; hybrid snapshot +
  update.
- **Reversibility:** moderate — envelope version and schema are explicit; a new payload type can
  be added without breaking v1 readers.
- **Risk if wrong:** future collaborative history/undo metadata cannot round-trip through a file.
  The file is explicitly "board content", not "session state", so this matches the brief.

### D04 — Import always remaps IDs and normalizes placement
- **Because:** two boards must be able to coexist without identity collisions; imported content is
  untrusted and must not be able to seize an existing ID. Import validates the whole payload,
  rejects duplicate IDs, remaps all IDs (including group parents), and translates content to the
  origin or to a user-chosen ghost placement.
- **Alternatives:** preserve IDs when no collision exists (still unsafe across merges); reject
  imports on any collision (bad UX).
- **Reversibility:** cheap.
- **Risk if wrong:** imported groups cannot be diffed against their source board by ID. Acceptable;
  the contract only promises content.

### D05 — Empty board imports land immediately; non-empty boards use click-to-place ghost
- **Because:** first import should not demand a placement decision; later imports must not silently
  cover existing work. Ghost shows occupancy validity and refuses overlapping drops.
- **Alternatives:** always ghost; always center; always paste at cursor with overlap allowed.
- **Reversibility:** cheap.
- **Risk if wrong:** overlap tolerance is an epsilon, so a deliberately touching layout may need a
  nudge. Noticed in UI text.

### D06 — AI ships as an offline mock provider only
- **Because:** BYOK, ChatGPT OAuth, and ACP all require accounts, credentials, or containment work
  that autopilot must not perform. The mock exercises the real card lifecycle (submit, stream,
  cancel, alternatives, failure, pin) with zero network and zero stored secrets.
- **Alternatives:** implement provider interfaces without keys (dead code); leave AI cards out
  (loses the reviewed interaction).
- **Reversibility:** cheap — `AiProvider` is the seam; a real provider is another impl.
- **Risk if wrong:** users may expect real answers from "Dibby demo (offline)". The card labels the
  provider and the help window states the limitation.

### D07 — Pinning a completed answer creates a board statement with the exact answer text
- **Because:** the product requires explicit sharing of AI output to the board and nothing else.
  The pin modal previews exactly what will be written; the question, alternatives, and provider
  metadata stay session-local.
- **Alternatives:** pin all alternatives; pin a summary; keep AI text outside the board document.
- **Reversibility:** cheap.
- **Risk if wrong:** a statement is plain text, so the answer loses markdown structure. The plan's
  statement object is plain text; richer formatting is a later object type.

### D08 — One process, one board, RAM only
- **Because:** the product is explicitly stateless: no autosave, no recent-files database, no
  recovery cache. Closing without export loses the session, and the close confirmation says so.
- **Alternatives:** autosave to a temp file (directly contradicts the product); session restore.
- **Reversibility:** cheap but a product decision the user should confirm.
- **Risk if wrong:** accidental close loses work. Mitigated by close confirmation with
  Export-then-close, and a persistent unsaved marker.

### D09 — Settings are session-only
- **Because:** persisting theme/grid/brush would create a settings store, a file format, and a
  migration surface not requested by the plan. Defaults are good; the settings panel says
  "session-only".
- **Alternatives:** write a config file next to the executable or in the OS config dir.
- **Reversibility:** cheap.
- **Risk if wrong:** users re-pick their theme each launch. Recorded for later.

### D10 — Native file dialogs via rfd, atomic sibling temp + rename
- **Because:** no in-app file browser to build, and an interrupted write must not truncate the
  previous export. `write_atomic` writes a sibling temp file, fsyncs, then renames.
- **Alternatives:** drag-and-drop only; a custom file picker.
- **Reversibility:** cheap.
- **Risk if wrong:** Windows rename over an existing file relies on the platform; the files crate
  test covers the happy path, not a crash mid-write (cannot be simulated portably).

### D11 — Direct-only WebRTC is deferred; no room UI is shown
- **Because:** signalling, ICE, and peer transports need a server or manual signalling exchange and
  real multi-machine testing; none can be added or verified unattended. The types and CRDT update
  encode/apply APIs are in place so the transport can land behind them.
- **Alternatives:** a local two-window loopback test (proves nothing about WebRTC); a fake rooms UI
  (dishonest).
- **Reversibility:** cheap — no UI promises rooms today.
- **Risk if wrong:** the core product promise (shared doodling) is missing from this build. Stated
  honestly in the status bar and help.

### D12 — No TURN, no relay, no board bytes through a server
- **Because:** product requirement. Enforced by omission: no networking code exists in this build,
  so there is nothing to accidentally route.
- **Reversibility:** must remain a constraint when networking lands.
- **Risk if wrong:** direct-only connections can fail behind symmetric NAT; that is the accepted
  product trade-off.

### D13 — Windows are placed on the laptop display only through `DOODLE_TEST_WINDOW`
- **Because:** the user offered the secondary display as a scratch pad. Hard-coding monitor
  placement would be wrong for real users, so the test harness uses an env var parsed by
  `doodle_platform::test_window_placement`; normal launches use remembered/default placement.
- **Alternatives:** always start on the primary display (would cover the user's work); a CLI flag
  (env var is enough for tests).
- **Reversibility:** cheap.
- **Risk if wrong:** a user setting that env var gets an unusual position; it is documented as a
  test hook.

### D14 — Mascot frames come from the approved atlas manifest, never regenerated
- **Because:** assets are approved input. The app parses the schema-2 manifest, derives UVs, and
  animates by elapsed time; it never invents frames.
- **Alternatives:** per-state PNG files (more IO, no atlas benefit); CSS/HTML preview only.
- **Reversibility:** cheap.
- **Risk if wrong:** manifest and atlas can drift; `parse_manifest` validates schema, dimensions,
  and frame ranges, and the atlas is embedded at compile time.

### D15 — Reduced motion wins over every mascot state
- **Because:** accessibility priority confirmed in the assets contract; it also stops repaint
  scheduling while idle.
- **Alternatives:** reduced motion only disables hover/busy but keeps talking.
- **Reversibility:** cheap.
- **Risk if wrong:** card looks static while generating. Status text still changes.

### D16 — No credentials, vault, hardware pen pressure, packaging, or release automation
- **Because:** each needs installing software, OS keychain integration, signing identities, or
  hardware unavailable here. Mouse pressure is simulated from speed and stroke progress; the
  settings panel states hardware pressure is a later milestone.
- **Alternatives:** a plaintext config for keys (violates the OS-vault-only contract).
- **Reversibility:** all are additive behind existing seams.
- **Risk if wrong:** pen users get simulated pressure until P04a lands.

### D17 — Keep the workspace license at "MIT OR Apache-2.0"
- **Because:** the plan's workspace manifest assumes it; changing licensing is the user's call.
- **Alternatives:** none taken.
- **Reversibility:** cheap but legally meaningful; recorded as an assumption.
- **Risk if wrong:** user wanted a different license; single manifest edit plus file headers.

### D18 — Commit in phases; push checkpoints when explicitly authorized
- **Because:** autopilot may commit locally; pushing is outward-facing and needs explicit user
  authorization. The user later authorized checkpoints to `github.com/asterxsk/doodleverse`, so
  pushes are allowed for this run. History stays split so a failed later phase does not bury
  working earlier phases.
- **Alternatives:** one giant commit (harder to audit); no commits (loses the record).
- **Reversibility:** cheap (local history).
- **Risk if wrong:** none material; pushed content is public, so each checkpoint is verified first.

### D19 — Floating Excalidraw-style toolbox, zen mode on Alt+Z
- **Because:** the user asked for Excalidraw's floating toolbox and its keybind. Excalidraw keeps
  tools in floating "island" panels over the canvas and hides the UI with Alt+Z zen mode (verified
  in Excalidraw's `actionToggleZenMode`: Alt+Z, no Ctrl/Cmd).
- **Alternatives:** keep the fixed top toolbar; use `Alt+H` (a plugin convention, not core
  Excalidraw); hide the UI with a menu-only action.
- **Reversibility:** cheap — the islands are drawn by one app module and can become panels again.
- **Risk if wrong:** on QWERTZ layouts Excalidraw users sometimes report Alt+Y; this build follows
  the logical Z key, so keyboard layout mapping is left to the OS.

### D20 — Miro-like board theming
- **Because:** the user asked for any board color and adjustable dot size. The existing grid
  settings gained board-color presets (Paper, Snow, Sand, Mist, Slate, Ink), a free color picker,
  and dot/plus size plus line width controls, all session-local.
- **Alternatives:** fixed light/dark themes only; separate "canvas" and "grid" color systems.
- **Reversibility:** cheap.
- **Risk if wrong:** theme is not persisted across launches (see D09).

### D21 — Hardware pen pressure through the Windows pointer stack
- **Because:** the PRD already requires real stylus pressure (5.5); egui 0.37 exposes no pressure
  field, so the app subclasses its window procedure and reads `WM_POINTER` pen pressure via
  `GetPointerPenInfo`. Samples feed `RawSample.pressure`; mouse input keeps the simulated envelope.
- **Alternatives:** winit tablet events (not surfaced by eframe 0.37); Wintab (legacy, extra DLL);
  leaving pressure simulated (contradicts the PRD).
- **Reversibility:** cheap — the probe sits behind `doodle_platform::pen_pressure()`.
- **Risk if wrong:** non-Windows platforms fall back to simulated pressure until native pointer
  probes land; the API returns `None` there, which is the same path as using a mouse. The subclass
  only reads pointer messages and forwards every message unchanged.

### D22 — Rune becomes the primary UI, eframe stays as a second shell
- **Because:** the user asked specifically for the Rune library by anomalyco, with the UI modified
  to match Excalidraw. Rune is a native Rust UI toolkit with its own renderer, so it was added as a
  new binary (`doodleverse-rune`) over the same `doodle-*` crates instead of rewriting the eframe
  shell in place.
- **Alternatives:** restyle egui to imitate Excalidraw (egui widgets cannot match the island look
  without custom painting); delete the eframe app (loses a working reference shell and its tests).
- **Reversibility:** cheap — both shells are separate binaries over shared crates.
- **Risk if wrong:** two UI code bases to keep in sync; features added to one must be mirrored in
  the other, or the eframe shell drifts and is eventually retired.

### D23 — Pin Rune to one revision and patch five crates
- **Because:** Rune at `958743308f36fdf1c568c513badc96ddb0c7f040` needs its own winit,
  parley_engine, wgpu-hal, accesskit_windows and accesskit_macos forks. The workspace carries the
  five `[patch.crates-io]` entries at the root, and `Cargo.lock` resolves the shared `wgpu-hal` to
  Rune's patched 30.0.1 so eframe and Rune link one copy.
- **Alternatives:** a separate workspace for the Rune app (shared `doodle-*` crates would need
  path dependencies and duplicate lockfiles); floating on Rune `main` (build breaks are guaranteed).
- **Reversibility:** medium — moving the pins means re-checking both shells compile.
- **Risk if wrong:** a Rune update can require touching the patches and re-resolving `wgpu-hal`.

### D24 — Pressure taper as one line shape per segment
- **Because:** Rune's `Shape` API has no per-vertex mesh, so a stroke cannot vary width within one
  polyline. Each segment is emitted as its own `line` with the averaged pressure width and round
  caps, which preserves the visual taper at the cost of more instances.
- **Alternatives:** a single fixed-width polyline (loses pressure response, contradicts the PRD);
  a custom wgpu pipeline inside Rune (large, fragile against the pinned revision).
- **Reversibility:** cheap — `canvas.rs::draw_stroke` is the only place that emits segments.
- **Risk if wrong:** long strokes get expensive; the renderer caps the segment budget at 200k.

### D25 — File dialogs through Rune's `Tx::open_file` / `Tx::save_file`
- **Because:** the blocking `rfd` dialogs used by the eframe shell bypass Rune's headless dialog
  interception and hung scripted runs. Rune's transactional dialogs (feature
  `native-file-dialog`) work in both real windows and headless scripts.
- **Alternatives:** keep `rfd` and give up headless file testing; stub dialogs behind a cfg flag.
- **Reversibility:** cheap — both call sites live in `app.rs`.
- **Risk if wrong:** Rune's dialog API changes with the pinned revision; the feature must stay
  enabled in `Cargo.toml`.

### D26 — Default ink follows the board theme
- **Because:** a dark board with the light-theme default ink made new strokes nearly invisible.
  Switching theme or board preset now swaps only the two default inks (dark ↔ light); any color the
  user picked is never touched.
- **Alternatives:** always start light strokes after a theme change (overrides user choice);
  leave it to the user.
- **Reversibility:** cheap.
- **Risk if wrong:** a user who intentionally toggles themes may see their first default color
  change once, which is the intent.

### D27 — Settings panel is a column island with its own scroll view
- **Because:** Rune's default `el()` lays children out as a row, and a row stretches its child to
  the panel height, so the content never registers as overflowing and wheel scrolling did nothing.
  A column island with a fixed `.h(..)` makes the scroll view real. The canvas wheel handler also
  ignores wheel events whose pointer is inside the panel rect, so the board beneath never pans.
- **Alternatives:** keep the row island and clamp content into view (unreachable brush controls);
  manual offset scrolling (needs a content height the layout does not expose).
- **Reversibility:** cheap — `view.rs::settings_panel` and `theme::island_col`.
- **Risk if wrong:** any future panel with fixed height and scrolling must use `island_col`.

### D28 — Adopt the Excalidraw token table for the Rune chrome
- **Because:** the scoping report measured the live app; matching its surfaces, indigo accent and
  geometry is what makes the Rune shell read as Excalidraw rather than an approximation. Values:
  light bg `#ffffff`, card `#ffffff`, border `#f1f0ff`, text `#1b1b1f`, accent `#6965db`; dark bg
  `#121212`, card `#232329`, border `#2e2d39`, text `#e3e3e8`, accent `#a8a5ff`.
- **Alternatives:** keep the warm coral palette (inconsistent with the Excalidraw arrangement the
  user asked for); use a fully custom palette (no reference to verify against).
- **Reversibility:** cheap — one palette function in `theme.rs`.
- **Risk if wrong:** the app's previous warm identity is gone; restoring it is a palette edit.

### D29 — Draw toolbar icons from Rune shapes, not SVG paths
- **Because:** Rune has no SVG path element but does expose `Shape`/`Path`/`LineCap`; simplified
  pencil, cursor, hand and three-dot geometry fits the same 16px grid and keeps the crate
  dependency-free. Every tool keeps its key and handler and gains an accessibility label.
- **Alternatives:** bundle an icon font (licensing, another dependency); parse SVG path data
  (arc segments in the spec paths make this disproportionate).
- **Reversibility:** cheap — icons live in four small helpers in `view.rs`.
- **Risk if wrong:** the hand and cursor read slightly more geometric than Excalidraw's curves.

### D30 — Record spec deviations instead of hacking around Rune's state model
- **Because:** Rune's `El` supports a pressed fill and pressed text colour but no pressed border, so
  the spec's "hover bg + 1px `#4440bf`" cannot be expressed without changing the rest state.
  Assistant is not bundled because its redistribution licence is unverified; the system font stack
  stands in. The six brush preset colours stay as artwork defaults, so the old coral survives as
  the "Coral" swatch while the accent role is indigo.
- **Alternatives:** force a border on press by swapping elements (flicker, layout churn); bundle
  Assistant anyway (licence risk); rename the Coral swatch (breaks the unchanged-key rule).
- **Reversibility:** cheap — each deviation is one helper or constant.
- **Risk if wrong:** pressed states stay visually flatter than Excalidraw's.

## Assumptions

- **A01 — Rust 1.99.1 toolchain is acceptable.** `rust-toolchain.toml` pins it, matching the
  plan's rust-version 1.95 floor. Confirm with the user's CI image; adjust the channel if needed.
- **A02 — Only the visible board is exported.** Hidden parents/deleted CRDT state are not part of
  the file. Product language ("export the board") supports this.
- **A03 — 8 peers, 100k objects, 64 MiB snapshot limits are the intended product limits.** Taken
  verbatim from the plan; all enforcement points cite `doodle_types::limits`.
- **A04 — The user's laptop display is `DISPLAY1` at negative X when the primary is on the right.**
  Verified by a native-window screenshot during this run; the env hook is generic, not
  DISPLAY1-specific.
- **A05 — Statement text is plain text, one style.** Pinned answers and future text objects share
  `StatementObject`; rich text was not in the plan's object model.

## Blockers (not attempted, by autopilot rule)

- **B01 — Peer-to-peer mesh (P02/P07/P10/P12).** Needs signalling/ICE design, likely a signalling
  server or manual SDP exchange, and multi-machine testing. Resume with a transport crate behind
  `BoardDocument::encode_full_update_v1` / `apply_remote_update_v1`, then WebRTC data channels
  (direct only, no TURN), room UI, and peer count enforcement. No network calls exist today.
- **B02 — BYOK AI provider (P13).** Needs the user's provider choice, API key handling policy, and
  OS vault integration. Resume by implementing `AiProvider` against the chosen API with keys read
  only from the OS vault; never write keys to disk, logs, or board files.
- **B03 — ChatGPT OAuth (P14).** Needs a registered OAuth client and user consent flow; autopilot
  must not create accounts or use credentials. Resume with the device-code or PKCE flow approved by
  the user.
- **B04 — ACP agent containment (P15).** Needs sandbox/containment decisions and installed agent
  binaries. Resume only after the user chooses the containment boundary; the current AI boundary
  deliberately carries one question and no board/path/tool content.
- **B05 — OS vault + non-Windows pen pressure (P04a).** Windows pen pressure is implemented (D21)
  but cannot be verified without a tablet. Other platforms still use simulated pressure. The OS
  vault needs platform keychain APIs and a chosen credential policy. Resume with vault adapters
  behind a credential-store trait and native pointer probes per platform.
- **B06 — Packaging, signing, release (P17/P20).** Needs signing identities and store accounts.
  Resume with `cargo-bundle`/MSIX/notarization as chosen by the user; the window icon is already
  embedded from the approved PNG.
- **B08 — Excalidraw UI scoping report.** Resolved 2026-11-16: the delegated OpenCode / DeepSeek
  run returned a full report, saved at
  [`docs/reference/excalidraw-ui-spec.md`](docs/reference/excalidraw-ui-spec.md). It confirmed the
  toolbar arrangement and supplied the token, geometry, icon and typography data now being applied.
- **B07 — PR / release publication.** Checkpoints are pushed to `main` on
  `https://github.com/asterxsk/doodleverse` under explicit authorization. No pull request or
  release was created. Resume with a PR/branch review or a tagged release if desired.

## Close-out (2026-11-16)

**Shipped:** the Rune front end (`crates/doodle-app-rune`, binary `doodleverse-rune`) over the
existing core: Excalidraw-style floating islands, `B`/`V`/`H` tools, marquee/group/move/delete,
undo/redo, six board presets plus custom board and grid colors, grid spacing/opacity/mark size/line
width, brush size/pressure/smoothing/taper sliders, light/dark themes with theme-aware default ink,
zen mode, AI cards with streaming and pinning, scripted-dialog export/import, close confirmation,
the Windows pen-pressure hook, a secondary-monitor placement env var, and the restored app icon.
The eframe shell still builds and keeps its tests.

**Gate run in this workspace:** `cargo fmt --all -- --check` pass, `cargo clippy --workspace
--all-targets --locked -- -D warnings` pass, `cargo test --workspace --locked` 69 passed / 0 failed,
`cargo build --workspace --release --locked` pass. Rune scenarios were driven headlessly and
recorded in [`docs/verification.md`](docs/verification.md); the real window was checked on the
secondary monitor, including a clean `WM_CLOSE`.

**Review before relying on:** D22 (two shells), D23 (five crate patches and the `wgpu-hal` pin),
D24 (per-segment stroke width is an approximation of a tapered mesh), D27 (Rune row-vs-column scroll
gotcha). All are cheap to reverse and documented above.

**Still blocked:** the same product blockers as before — WebRTC transport (B01), real AI providers
and credentials (B02–B04), OS vault and non-Windows pen pressure (B05), packaging/signing (B06),
and PR/release publication (B07). Pushing checkpoints to the authorized GitHub remote is not
blocked and continues.

### D31 — Shapes, text and sticky are document objects, not chrome
- **Because:** the user asked for the full Excalidraw toolset. `ShapeObject { shape, width,
  height, flip_x, flip_y, style }` covers rectangle, diamond, ellipse, arrow and line in one
  kind; text and sticky reuse `StatementObject` with an appended `StatementKind`. Appending
  variants and trailing fields keeps old payloads readable; the schema version stays 1.
- **Alternatives:** rasterising shapes into strokes (loses editability and export fidelity);
  a new document format (breaks every existing board).
- **Reversibility:** additive — new variants and fields decode with defaults.
- **Risk if wrong:** older builds cannot read boards that contain shapes or the new statement
  byte; recorded as a deliberate forward-compatibility break.

### D32 — One foreground panel at a time
- **Because:** settings, help and the minimap each floated independently and overlapped each
  other, the brush pill and, at 880pt, the Ask Dibby card. Opening one now closes the other two;
  the brush pill hides while a panel is open; help repositions around an open card instead of
  closing it.
- **Alternatives:** allow stacking with z-order rules (still unreadable at 880pt); close cards
  when help opens (loses draft content).
- **Reversibility:** cheap — state transitions in `app.rs` and render guards in `view.rs`.
- **Risk if wrong:** users cannot compare settings and the minimap side by side.

### D33 — Lucide for UI icons
- **Because:** the hand-drawn icons were approximations. Lucide is ISC-licensed, free for
  commercial use, drawn on a 24px stroke grid, and its SVGs are simple path data. Nineteen SVGs
  are vendored under `assets/icons/lucide/` with the licence and a source note; a small SVG path
  parser (including arc-to-cubic) renders them through `rune::Path`.
- **Alternatives:** Feather (smaller set), Phosphor (heavier), bundling an icon font (licensing
  and extra dependency).
- **Reversibility:** cheap — swap the vendored SVGs and the name mapping.
- **Risk if wrong:** icon paths Rune cannot tessellate; mitigated by parser tests and screenshots.

### D34 — Tool-specific cursors, canvas-drawn eraser circle and tail, wheel-up zooms in
- **Because:** the user asked for a cursor per tool (precision crosshair for shapes, custom pen
  and note cursors, default arrow for select, hand for hand and middle-drag, I-beam for text,
  translucent circle for eraser) and a fixed-length translucent tail following the eraser. The
  eraser feedback is drawn on the canvas rather than as a cursor image, so the circle exactly
  matches the erase radius at every zoom and can carry a separate tail alpha; the OS cursor is
  hidden only over the board while the eraser is active. Wheel-up is defined as zoom-in, wheel-down
  as zoom-out, in both scroll conventions.
- **Alternatives:** an eraser cursor PNG (fixed pixel size, cannot track zoom or the real hit
  radius, and no tail); leaving wheel direction to the platform default (the reporter saw the
  opposite of the expected direction).
- **Reversibility:** cheap — cursor mapping in `app.rs`/`view.rs`, one canvas overlay, one wheel
  sign.
- **Risk if wrong:** a hidden cursor with no overlay on a platform where canvas overlays fail
  would leave the eraser invisible; the overlay is the same scene pass as strokes, so this is
  covered by the headless frames.

### D35 — Linear-cost scene batching and low-zoom stroke fidelity
- **Because:** the canvas rebuilt its whole `rune::Shape` by cloning once per grid mark and once
  per stroke segment (`shape = shape.clone().fill(...)`), so a frame cost O(N²) in marks. The grid
  allows up to 18,000 marks and the visible field grows as zoom decreases, which matches the
  reported "exponentially laggier the further out I zoom". The same stalls starve pointer events,
  so freehand strokes collapse to straight chords at low zoom. Grid marks and stroke segments are
  now batched into a few `Path`s (one fill for all dots, one stroke path for pluses/lines, width
  buckets for pressure-varying strokes), the mark budget is lowered and the LOD spacing made
  coarser at extreme zoom-out, and sparse pointer samples are subdivided/curved so curvature
  survives at 5–10 % zoom.
- **Alternatives:** keep per-mark shapes and only lower the cap (still O(N²) and visibly sparse);
  rasterise the grid into a texture (more platform surface, harder theming); leave strokes as
  straight segments and accept the fidelity loss (rejected by the user).
- **Reversibility:** cheap — the batching lives in `canvas.rs`; the grid budget is a constant in
  `doodle-render`.
- **Risk if wrong:** batching can merge joins visually (mitigated by round caps and width buckets)
  and over-smoothing can invent curvature (bounded interpolation, tests on sparse curves).

## Overnight close-out (2026-11-16, second session)

**Shipped:** the Excalidraw top bar in `1faf8be` (board menu top left, centred horizontal icon
toolbar, three-dot overflow top right) and the token/icon/accessibility pass in `91c38a1`: indigo
accent, radius-8 islands, 36×36 tool boxes, 22×22 swatches, `#121212` dark canvas, Rune-shape
icons, instant popovers, dialog focus traps with inert backgrounds, named/checked/disabled
controls, slider roles and values, first-frame viewport sizing, card and answer sizing, pinned-note
placement with shrink-to-fit and 12pt margins, narrow-width Export in the overflow, Escape overlay
priority, press-leak suppression, and the pluralisation fix.

**Verification:** ten independent fresh-subagent QA rounds at 1180×760 and 880×600 in light and
dark; each round's findings fixed by a separate implementer and re-tested by a new tester. Final
gate: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D
warnings`, `cargo test --workspace --locked` (69 passed), `cargo build --workspace --release
--locked` all pass. Release build launched in a real window: title-bar/taskbar icon present, new
chrome renders, `WM_CLOSE` on an empty board exits cleanly.

**Decisions to review first:** D28 (token table), D29 (icons from Rune shapes), D30 (recorded Rune
deviations: no pressed border, Assistant not bundled, coral survives as the Coral brush preset).

**Known residuals:** an extremely long pinned answer narrowed to 200pt can still exceed the free
board height and falls back to least-overlap; a group container counts as one status object; the
unsaved-board close dialog shares the verified pin-dialog code but cannot be raised headlessly.

**Still blocked:** B01–B07 unchanged (transport, real AI providers and credentials, OS vault and
non-Windows pen pressure, packaging/signing, PR/release publication). Checkpoints continue on the
authorized GitHub remote.

## coss ui restyle, tool panel and pressure pass (2026-11-19)

### D36 — coss ui neutral tokens replace the indigo Excalidraw accent
- **Because:** the brief asks for coss ui's neutral, crisp system rather than Excalidraw's
  indigo. The palette now uses white/`#161616` backgrounds, `#262626`/`#f5f5f5` primary,
  translucent black-8%/white-6% borders, muted-foreground `#606060`/`#808080`, and the ring
  tokens for selection. Surfaces share radius 10 and controls radius 8 through two constants.
  Light and dark are separate token sets, not inversions. Swatches derive their selected ring
  from the swatch's own luminance (light ring on dark ink, dark ring on pale ink) because the
  neutral ring token disappears on both extremes.
- **Alternatives:** keep indigo and restyle only radii (does not meet the brief); use pure
  black/white borders (harsher than coss's translucent borders).
- **Reversibility:** cheap — values live in `theme::palette` and two radius constants.
- **Risk if wrong:** a neutral primary makes the selected tool less colourful than before; the
  filled primary surface and primary-foreground icon/key badge carry the state instead.

### D37 — Tool-only top bar, left tool-settings panel, brush pill removed
- **Because:** the user asked for Excalidraw's layout in coss tokens: the top bar selects tools
  and the left panel edits the next drawing. The brush pill duplicated swatches and a size
  slider under the toolbar, so it was removed. Lock stays as the leading mode control, matching
  the supplied reference. The panel shows for Draw, Rectangle, Diamond, Ellipse, Arrow, Line,
  Text and Sticky; it is hidden for Hand, Select, Eraser and zen. Contents adapt: stroke always,
  background/fill for closed shapes, width presets except text/sticky, pressure for pen, opacity
  always; Sticky swaps the stroke row for six note tints. The duplicate Size and Pressure response
  sliders were removed from Canvas settings, which now keeps Smoothing and taper only.
- **Alternatives:** keep the brush pill (duplicates controls and crowds the toolbar); show the
  panel for every tool (navigation tools have no drawing style).
- **Reversibility:** cheap — `tool_panel_visible` and the view builder.
- **Risk if wrong:** the panel covers more board at small window sizes; the board's left inset,
  card clamp and pin placement now step past it, and Canvas settings shifts right while it is up.

### D38 — Shape fill is appended to `ShapeObject` under schema version 1
- **Because:** boards written before fill must keep decoding. `fill_rgba: [u8;4]` and
  `fill_style: FillStyle` are appended after `flip_y`, and the existing end-of-buffer visitor
  defaults them to transparent hachure. Alpha 0 means no fill, so old boards render unchanged.
  The schema version stays 1.
- **Alternatives:** a new schema version (breaks every existing board for an additive field);
  a separate fill object (complicates grouping and export).
- **Reversibility:** additive — older builds ignore the trailing bytes only if their decoder is
  EOF-tolerant; this build is forward-compatible for the new fields.
- **Risk if wrong:** a build that is not EOF-tolerant cannot read boards that carry fill; the
  schema note records the same deliberate forward-compatibility policy as D31.

### D39 — Hatch fills are clipped analytically and line-capped
- **Because:** Rune has no clip region. Hachure and cross-hatch are diagonal line families in
  screen space, clipped by polygon edge intersections (rectangle, diamond) or the analytic
  line/ellipse quadratic (ellipse). Spacing is `max(6px, line_width * 3)` and widens to keep a
  240-line cap per shape, so deep zoom-out and huge shapes stay linear-cost. Solid fill is one
  filled polygon. Strokes paint after fill.
- **Alternatives:** raster clip masks (not available in Rune's canvas); per-pixel or per-triangle
  hatching (far more geometry); no cap (a huge shape at high zoom could emit thousands of lines).
- **Reversibility:** cheap — `canvas.rs` owns the fill pass.
- **Risk if wrong:** very large shapes get wider hatch spacing to respect the cap; the pattern is
  still complete, just coarser.

### D40 — Pressure is speed-driven and can be switched off
- **Because:** the old mouse path mixed speed with an elapsed-time envelope, so later samples grew
  regardless of pace. The envelope is gone. A responsive speed EMA (alpha 0.35) becomes screen
  points per second via `speed_ema * 0.7 / min_distance` (the builder derives `min_distance` as
  `0.7 / zoom`), normalized at 300 px/s, and mapped to `1.6 - 1.3 * speed_norm`, clamped
  `0.25..=1.6`: slow is thick, fast is thin, at every zoom. The wider range is what makes the
  taper read at a glance; the first `1.2..0.4` mapping was nearly invisible on the board. The
  default sensitivity is 0.8.
  `BrushSettings::simulate_pressure` (default true) returns 1.0 for mouse samples when false;
  hardware pen pressure still wins. The left panel's Pressure Off/On switch drives this.
- **Alternatives:** keep the envelope (wrong direction for the reported feel); make the switch
  disable hardware pressure too (the brief says a real pen still wins).
- **Reversibility:** cheap — one field, one mapping, one panel toggle.
- **Risk if wrong:** the mapping is tuned for a 4pt default brush; very wide brushes need a faster
  stroke before they thin. The formula keeps the old width clamp and taper. A first tuning pass
  made normal strokes hairline, so the curve was recalibrated against real screen-speed drags and
  the sensitivity raised from 0.65 to 0.8.

### D41 — Opacity is baked into the next object's alpha, not a live style edit
- **Because:** the brief scopes the panel to the next drawing and keeps selection editing out of
  this pass. Opacity multiplies the chosen stroke and fill alpha at creation, so no new document
  field or re-projection path is needed.
- **Alternatives:** a per-object opacity field (schema change and style editing UI); live editing
  of selected objects (explicitly out of scope).
- **Reversibility:** cheap — `stroke_rgba` / `fill_rgba_effective` in `app.rs`.
- **Risk if wrong:** changing opacity after drawing does not affect existing objects; that matches
  the brief and the same limitation as stroke colour and width.

### D42 — Board and grid presets are neutral, not indigo
- **Because:** the coss theme is the whole surface, and the old defaults leaked Excalidraw's
  indigo into the board background, grid and the Slate/Ink presets. Light default is now
  `#ffffff` background with `#d4d4d4` grid lines; dark default is `#161616` with `#525252`.
  Slate is `#262626`/`#737373` and Ink is `#161616`/`#525252`, so the board and the chrome read as
  one neutral system in both themes.
- **Alternatives:** keep indigo board presets (they fight the neutral chrome and the reference
  asks for coss); remove dark presets (users still need a dark board in light theme).
- **Reversibility:** cheap — preset tables and `toggle_theme`.
- **Risk if wrong:** neutral boards are lower-contrast than the old indigo for some users; stroke
  ink defaults stay black/white and the width presets preserve legibility.

### D43 — Text and sticky colours are real state, not hardcoded defaults
- **Because:** the new panel offers stroke colour for Text and background colour for Sticky, but
  both committed with a fixed default style. Sticky now stores `note_rgba` (six neutral note
  tints), Text stores the stroke colour; both multiply by opacity at commit. Sticky ink picks
  light/dark text from the note's luminance so every note tint stays readable. The inline editor
  previews the same colours, so the draft matches the committed object.
- **Alternatives:** keep note colour fixed (the swatches would be dead controls); auto-derive
  sticky ink from theme only (fails on saturated note tints).
- **Reversibility:** cheap — `note_rgba`, `commit_editor` and the statement ink helper.
- **Risk if wrong:** a very saturated future note tint could read poorly; the luminance threshold
  is the guard and the current six tints all clear it.

### D44 — Dark chrome is the default
- **Because:** the user tests and prefers dark ("just let's keep it dark mode"), so first launch
  opens `light_theme: false` with the neutral `#161616` board and `#525252` grid. Light stays a
  full token set behind the Canvas settings toggle; nothing was removed.
- **Alternatives:** follow the OS theme (the owner's OS preference is not the app's default);
  drop light entirely (the token set costs nothing and remains useful for pale boards).
- **Reversibility:** cheap — one initialiser and the toggle.
- **Risk if wrong:** a light-theme user must flip one switch; the canvas settings row is visible
  in the same panel cluster as zoom and undo/redo.

### D45 — Material Symbols Rounded is the only icon source
- **Because:** the brief asked for professional icons sourced online, not hand-drawn ones. The
  filled rounded set matches the coss geometric feel, covers every chrome and tool glyph, and
  Apache-2.0 permits vendoring. Exact upstream path data is stored in `icons.rs` and drawn by
  Rune's SVG icon engine, which handles fill rules and transforms. Pen and sticky cursors are
  rasterised from the same glyphs by an ignored test, so cursor and tool button cannot diverge.
  Licence text: `assets/icons/LICENSE-material-symbols.txt`; selection: `docs/plans/icon-selection.md`.
- **Alternatives:** Lucide (outline only, thin at 16pt), Phosphor (heavier license bookkeeping),
  hand-drawn paths (rejected by the brief).
- **Reversibility:** cheap — swap the statics and regenerate cursors.
- **Risk if wrong:** the filled set reads bolder than an outline set at 16pt; the toolbar buttons
  are 32pt and the glyphs sit at 16pt, which the rendered frames showed as balanced.

### D46 — Ink drops to an antialiased polyline when every width is sub-pixel
- **Because:** the outline-ribbon stroke is exact while its width covers at least a screen pixel.
  Zoomed far out, widths clamp under 1px and the ribbon's filled edge staircase reads as
  pixelation (the user's report). When every sampled width is below one point, `draw_stroke`
  decimates the screen points to half-point spacing and emits one `Shape::stroke` polyline at
  1pt with round caps, so Rune's antialiased capsule pass fades the line smoothly, the same
  treatment a vector editor gives a scaled-down line. Taper is invisible at that size.
- **Alternatives:** MSAA (Rune exposes no sample-count control); rendering the board to an
  offscreen texture and downscaling (a whole render path for one edge case); leaving the clamp at
  `0.5` (the reported staircase).
- **Reversibility:** cheap — one branch in `draw_stroke`.
- **Risk if wrong:** at the crossover zoom a stroke settles to a uniform 1pt line instead of a
  tapered one; the crossing happens where the taper is already below a pixel.

### D47 — The chrome follows coss ui component anatomy and motion
- **Because:** the follow-up brief asked for every UI element to match coss, especially dark mode,
  with the same smooth animations. Tokens carry coss dark values (`#161616` background, `#1b1b1b`
  card, 6% border, 8% input, 14/500 button type, Tailwind radius scale: `rounded-lg` 8 for
  buttons, toggles and popovers, `rounded-md` 6 for menu items, `rounded-xl` 12 for the
  toolbar, `rounded-2xl` 16 for dialogs); Slider is track h-4, primary indicator, 16pt white
  thumb with input border and a 1.2 press scale; Switch is an 18x30 track with a 16pt thumb and
  a 150ms spring; Toolbar is a `rounded-xl` card with 32pt buttons and 1px group separators;
  a pressed tool or toggle is coss's `bg-input/64` translucent wash with the primary ink — not
  a filled primary chip; segmented choices are 28pt 14/500 Toggles; menus use `p-1` and the
  scale-and-fade popup motion (0.96–0.98), as do dialogs and floating panels, on Rune's
  `motion::SNAPPY`. Hover/press tokens are 4%/8% alpha overlays.
- **Alternatives:** keep bespoke chrome (the brief explicitly replaces it); animate with the
  default fade only (the brief asks for the same animations).
- **Reversibility:** cheap — `theme.rs` holds every token and component constructor.
- **Risk if wrong:** Rune's motion curve is not literally coss's CSS easing; it is the closest
  built-in spring, and reduced motion still disables it.

### D48 — Small interaction details from the second critic pass
- **Because:** a fresh critic scored the Build-record revision and named five gaps that read as
  polish defects to a user: the tool panel smeared between tool switches, the overflow trigger
  went white-on-white while its menu was open, the visible switch thumb swallowed clicks aimed at
  it, the slider thumb only scaled on press but not while dragging, and the tooltip used a
  bespoke dark box instead of the coss popover tokens.
- **Decision:** the tool panel gets `.instant_changes()` (no cross-fade between two different
  tool bodies) under its scale transition; the overflow and menu triggers keep the pressed wash
  while open so the glyph stays legible; the switch thumb loses `.hit()` so the row's press
  handler owns the gesture; the slider keeps a `slider_drag` target in app state and scales the
  thumb 1.2 while it is held (`motion::SNAPPY`); `view()` installs a `HelpLook` with popover
  fill, border ring, radius 6, 12pt text and 0.98 fade, matching coss Tooltip; the AI card uses
  `RADIUS_DIALOG` 16.
- **Alternatives:** swapping tool bodies on an identity key change (heavier state work for the
  same result); a pressed-only thumb scale (the drag continues after press settles, so it
  relaxed mid-gesture).
- **Reversibility:** cheap — each is one attribute or one field.
- **Risk if wrong:** `.instant_changes()` also suppresses the intended scale fade *within* one
  tool's body changes (e.g. opacity value), which is acceptable because the panel's outer
  transition still runs on show/hide.

### D49 — Ink weight plateaus, the stroke ends where the hand stops, and the pen takes a background
- **Because:** using the app, the owner reported three things: the pen's Background row had gone
  missing and fill could not be turned off; the pressure brush "keeps dying", because width kept
  growing as the hand slowed right down; and every stroke closed in a round blot that reads as a
  pool of ink rather than a lifted pen. The same report asked for Excalidraw-smooth edges at
  wide widths.
- **Decision:** the speed→pressure response is a plateau: below a walking pace the ink sits at
  its full 1.45× weight and stacks no further, thinning smoothly to 0.5× at a brisk stroke.
  The stroke's end has no cap disc — a short 3% end taper closes the band instead, so the line
  ends where the samples end. The pen's tool panel shows the Background row (transparent first)
  and `StrokeObject` carries `fill_rgba`: when a pen stroke comes back to its own start within
  about its own width, its interior is filled before the ink is drawn, matching Excalidraw's
  freedraw; an open scribble gets no fill. The old two-field stroke payload still decodes, via
  an EOF-tolerant visitor, and the fill is written for committed strokes so a reload keeps it.
  Edges: the ribbon fill is emitted as overlapping 32-point chained polygons (one ear-clipped
  polygon of a thousand near-collinear vertices produced the lumpy edge), a sub-2pt stroke uses
  one stroked polyline (chunk edges showed as a dashed fringe), and cap/joint discs take the
  smoothed band width rather than the raw sample width, which is what poked out of the outline.
- **Alternatives:** keep the continuous speed response and only clamp it (still reads as
  swelling on a slow stop); a round end cap sized by the band (a lifted pen does not leave a
  disc); storing pen fill only in app state (lost on reload).
- **Reversibility:** cheap — the plateau is two constants in `brush.rs`; the fill is one field
  and one predicate in `canvas.rs`; the chunk size and overlap are constants.
- **Risk if wrong:** a self-crossing pen loop fills its nonzero-interior region rather than each
  lobe separately, and a loop that does not quite close stays unfilled; both match freedraw
  behaviour rather than a closed-shape fill.

### D50 — The custom colour picker is an anchored popover, not a dialog
- **Because:** the first cut of the picker opened as a centred modal over a scrim, and the owner
  asked for Excalidraw's shape instead: a panel that opens beside the tool panel, keeping the
  board and the swatch rows visible while a colour is chosen.
- **Decision:** the last swatch of the Stroke and Background rows carries the last custom colour
  with a plus, and pressing it opens a 248pt card anchored to the right of the measured tool
  panel (`tool_panel_layout`), with a picker scrim beneath it that closes it on an outside press
  and swallows that press so it cannot start a stroke. The card holds twelve colours (the
  background row adds a transparent tile first), a five-step shade ramp mixed from the current
  colour, and a hex field with a check button; a colour tile applies and closes, the check
  applies and keeps the panel open for tweaking. Hex accepts `#rgb` and `#rrggbb`; alpha stays
  255 because opacity is the panel's own control.
- **Alternatives:** keeping the modal (owner rejected it); live-applying valid hex on each
  keystroke (the text field's accessor cannot read the new text inside the same edit); an
  eyedropper (no screen-sampling API in the Rune host).
- **Reversibility:** cheap — one view function and four fields on `DoodleApp`.
- **Risk if wrong:** the popover can overlap the panel on very narrow windows; it falls back to
  a fixed offset until the panel's layout is measured, which is the first frame only.

### D51 — Zoom range is 50%–800%
- **Because:** the owner asked for the zoom to cap at 800% with a 50% minimum: past 8× a pen stroke covers the view, and below half size a board is unreadable at the sizes people draw.
- **Decision:** `MIN_ZOOM` 0.5, `MAX_ZOOM` 8.0 in `doodle-types/src/limits.rs`; the camera's clamp, wheel zoom and fit all read those constants, so the whole app (Rune and eframe) inherits it.
- **Alternatives:** capping only the wheel handler (the zoom-to-fit and preset paths would still exceed it).
- **Reversibility:** trivial — two constants.
- **Risk if wrong:** boards larger than about two screens no longer fit at 50%; the minimap still covers navigation.
