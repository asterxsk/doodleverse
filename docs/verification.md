# Verification

Last full gate: 2026-11-15, workspace at the README/docs checkpoint. A check is listed as passed
only if it ran in this workspace.

## Automated gate

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass |
| `cargo test --workspace --locked` | 57 passed, 0 failed |
| `cargo build --workspace --release --locked` | Pass |

Test distribution: `doodle-types` 6, `doodle-canvas` 12, `doodle-document` 12, `doodle-files` 8,
`doodle-ai` 11, `doodle-render` 3, `doodle-platform` 1, `doodle-app` 4.

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

## Known verification gaps

- **Hardware pen pressure** cannot be verified without a tablet. The Windows pointer probe compiles
  and is installed at startup; the fallback path (`None` → simulated pressure) is what the UI
  session exercised. No claim is made that a real pen was tested.
- **Export/import through the native dialogs** was not automated. Codec, validation, remapping, and
  atomic write are covered by tests; the dialog wiring is inspected only.
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
cargo run --release -p doodle-app
```

Close the app before rebuilding; the running executable locks the release binary on Windows.
