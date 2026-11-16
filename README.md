# Doodleverse

A native, local-first whiteboard written in Rust. Infinite canvas, pressure-aware ink,
portable boards, and a small offline AI sketch partner named Dibby — no accounts, no
cloud, no autosave.

The interface now runs on [Rune](https://github.com/anomalyco/rune), anomalyco's Rust UI
library. The chrome follows Excalidraw's arrangement — a centred horizontal toolbar, a board
menu top left, and a three-dot menu top right — while board theming stays Miro-style. The
original eframe/egui shell stays in the workspace as a second front end over the same core.

![Doodleverse on Rune](docs/images/rune-window.png)

## What works today

- **Infinite canvas** with cursor-anchored wheel zoom (plain wheel, `Ctrl`/`Cmd`+wheel, or
  pinch), `Shift`+wheel horizontal panning, and hand- or middle-drag panning, over a bounded
  level-of-detail grid (dots, pluses, lines, or none).
- **Pressure-aware brush.** Mouse strokes simulate pressure from speed and progress. On
  Windows, tablet pen pressure is read from the native pointer stack and applied per sample.
- **Excalidraw toolset.** Rectangle, diamond, ellipse, arrow and line shapes (Shift constrains),
  text blocks and sticky notes with an inline editor, and a whole-object eraser where one drag
  is one undo step. Tools are `Q` lock, `H` hand, `V` select, `R` rectangle, `D` diamond,
  `O` ellipse, `A` arrow, `L` line, `P` pen (`B` also works), `T` text, `N` sticky, `E` eraser.
  The pointer matches the tool (crosshair for shapes, pen and note cursors, I-beam for text,
  grab for hand and middle-drag); the eraser paints its hit disc plus a short translucent tail.
  Freehand point samples are curved at low zoom, and the grid and stroke paths are batched so
  deep zoom-out stays linear-cost.
- **Selection and structure.** Marquee/click selection, move, delete, group, ungroup,
  select all, and command-level undo/redo that never records remote CRDT edits.
- **Board theming** with presets and custom colors, grid spacing/opacity/color, dot/plus
  size and line width.
- **Excalidraw-style chrome.** A centred icon toolbar with key badges and a lock that pins it,
  a board menu top left, a three-dot menu top right, and a bottom-left cluster holding zoom,
  canvas settings, the minimap and undo/redo. Icons are [Lucide](https://lucide.dev) (ISC).
- **Portable files.** `.doodleverse` export uses a versioned envelope with CRC validation.
  Import validates the whole payload, remaps every identity, and places the board at the
  origin (empty canvas) or as a validated ghost preview (non-empty canvas).
- **Dibby AI cards** that stream answers from a fully offline demo provider, support
  alternatives, cancellation, and pin a single completed answer to the board as a normal
  statement object. Questions and drafts never leave the session.
- **Dibby mascot** animated from the approved atlas and motion manifest, with reduced-motion
  support and a neutral frame fallback.
- **Stateless by design.** Nothing is written unless you export. Closing the window asks
  first and offers export-then-close.

## Run it

Rust is pinned by `rust-toolchain.toml` (1.99.1 with rustfmt and clippy).

```bash
cargo run -p doodle-app-rune      # Rune UI (primary)
cargo run --release -p doodle-app # eframe UI (reference shell)
```

The Rune build pulls the pinned `rune` git revision and its five `[patch.crates-io]` entries
(winit, parley_engine, wgpu-hal, accesskit_windows, accesskit_macos) from the workspace root.
`Cargo.lock` pins `wgpu-hal` to Rune's patched 30.0.1 so both front ends resolve one copy.
On Windows set `DOODLEVERSE_WINDOW_POS=x,y` to open at physical desktop pixels, e.g.
`-1920,0` for a left-hand second monitor.

Checks that gate the repository:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked
```

For unattended UI runs, `DOODLE_TEST_WINDOW="x,y,w,h"` places the window explicitly so a
test never covers the primary display.

## Controls

| Input | Action |
| --- | --- |
| `Q` / `H` / `V` / `E` | Lock toolbar / Hand / Select / Eraser |
| `R` / `D` / `O` / `A` / `L` | Rectangle / Diamond / Ellipse / Arrow / Line |
| `P` (or `B`) / `T` / `N` | Pen / Text / Sticky note |
| Left drag | Draw, draw a shape (Shift constrains), move selection, marquee, or erase whole objects |
| Middle drag | Pan the board with any tool |
| `Shift` + click | Toggle selection |
| `Ctrl+A` | Select all top-level objects |
| `Delete` / `Backspace` | Delete selection |
| `Ctrl+G` / `Ctrl+Shift+G` | Group / ungroup selection |
| `Ctrl+Z` / `Ctrl+Y` or `Ctrl+Shift+Z` | Undo / redo |
| Hand tool drag | Pan the board |
| Wheel or trackpad scroll | Zoom at the cursor |
| `Ctrl`/`Cmd` + wheel, or pinch | Zoom at the cursor |
| `Shift` + wheel | Pan horizontally |
| `F` / `0` | Fit content / reset view |
| `Alt+Z` | Toggle Excalidraw-style zen mode |
| `Esc` | Cancel stroke, drag, marquee, erase, or pending import |

## The board file

A `.doodleverse` file is a self-contained snapshot of the visible board:

```
magic "DVRSE\0\0\0" | envelope version | schema version | flags
payload length | CRC32(payload) | postcard(BoardSnapshot)
```

Import never trusts a file: size, header, payload length, CRC, schema, object validity,
duplicate IDs, group parents, and depth are all checked before anything is committed.
Imported content is remapped to fresh IDs so two boards can coexist. See
[docs/contracts.md](docs/contracts.md).

## Workspace

| Crate | Role |
| --- | --- |
| `doodle-types` | IDs, geometry, objects, commands, limits, versions |
| `doodle-canvas` | Camera, brush pipeline, stroke geometry, hit tests |
| `doodle-document` | yrs CRDT document, projection, undo history, snapshots |
| `doodle-render` | Grid frames and cached screen-space stroke meshes |
| `doodle-files` | File envelope, validation, ID remapping, atomic writes |
| `doodle-ai` | AI card state machine, provider trait, offline demo provider, mascot |
| `doodle-platform` | Native dialogs, pen pressure, test-window placement |
| `doodle-app` | eframe/egui application shell and canvas UI |
| `doodle-app-rune` | Rune application shell (`doodleverse-rune`): toolbox, theming, cards, dialogs |

Dependencies point inward: app shells → render/files/ai/document/canvas → types. See
[docs/architecture.md](docs/architecture.md) for the data flow and invariants.

## Honest limitations

- **Networking is not implemented.** There is no signalling, WebRTC transport, room UI, or
  remote cursor. The CRDT document and update APIs are ready for a transport, but no board
  byte is sent anywhere in this build.
- **AI is an offline demo.** BYOK, ChatGPT sign-in, ACP agents, and the OS credential vault
  are not included. No credentials are stored in any form.
- **Pen pressure is wired on Windows only.** Other platforms use the mouse simulation path
  until native pointer probes land.
- **Packaging/signing** (installer, MSIX, notarization, app-store metadata) is not done; the
  release binary is a plain native executable.
- **Settings are session-only.** Theme, grid, and brush choices reset when the app closes.

These are recorded with rationale and resume steps in [decisions.md](decisions.md).

## Privacy

No telemetry, no accounts, no network calls. Boards live in RAM until you export. The AI
card boundary carries one typed question and an optional short instruction — never the
board, file paths, peer IDs, or conversation history. The Windows pen probe only reads
pointer pressure inside the app window.

## License

MIT OR Apache-2.0. UI icons are from [Lucide](https://lucide.dev) under the ISC licence; the
notice and the vendored SVGs live in `assets/icons/`.
