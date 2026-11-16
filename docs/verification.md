# Verification

Last full gate: 2026-11-16, workspace with the Rune front end. A check is listed as passed only if
it ran in this workspace.

## Automated gate

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass |
| `cargo test --workspace --locked` | 69 passed, 0 failed |
| `cargo build --workspace --release --locked` | Pass |

Test distribution: `doodle-types` 6, `doodle-canvas` 15, `doodle-document` 16, `doodle-files` 11,
`doodle-ai` 11, `doodle-render` 5, `doodle-platform` 1, `doodle-app` 4, `doodle-app-rune` 0 (the Rune
shell is verified through its headless script mode).

What the tests cover:

- Snapshot validation: duplicate IDs, missing parents, non-group parents, cycles, depth, invalid
  geometry and text.
- Envelope: roundtrip, bad magic, truncation, bad checksum, unsupported version/schema/codec/flags,
  oversize payload, empty board.
- Import: ID and parent remapping, origin normalization, wrapper group, atomic write, failed write
  leaves the target untouched.
- Document: every command, local undo/redo, remote-origin updates excluded from history, group
  depth and cycle handling, projection ordering and derived bounds.
- Canvas: camera clamping and anchoring, brush pressure envelopes, stroke validity, hit clearance.
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
| Wheel pan, `Ctrl`+wheel zoom and pinch magnify handlers | Implemented; wheel pan and button zoom verified |
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
| Tokens: indigo accent, radius-8 islands, 36×36 tool boxes, 22×22 swatches, `#121212` dark canvas | Verified in light and dark screenshots |
| Menu and overflow popovers mount and unmount instantly; scrim and Escape dismiss | Verified; no ghost rows one frame after dismissal |
| Settings panel scroll with sticky header and close; help panel sticky header and Escape | Verified at 880×600 |
| Canvas wheel pan over the empty-board hint; wheel over panels scrolls them, not the board | Verified |
| AI card with wrapped long question: card inside the window, footer clickable, answer scrolls fully | Verified at both sizes |
| Pinned answers: note sized to its text, clear of chrome, never under the card | Verified for short and long answers, panned and unpanned |
| Presses that open or dismiss a popover never start a canvas stroke | Verified at 0.02–0.1s delays and a normal tap still draws one stroke |
| Chrome stays fixed when a card opens or closes | Verified by matching island bounds at 0.15s and 0.3s |
| Accessibility: checked states, slider role/value/actions, named and disabled buttons, dialog roles, scrims hidden | Verified in the element tree |
| Pin and close dialogs trap Tab/Enter, hide the background from accessibility, and restore focus with a visible ring | Verified for the pin dialog; the close dialog shares the code but cannot be raised headlessly |
| Disclosure buttons report expanded/collapsed; disabled buttons consume presses instead of drawing | Verified |
| First frame matches the viewport; completed answers start at their first line | Verified |
| Dark theme strokes, undo/redo, counts, group/ungroup, zen | Verified |

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
