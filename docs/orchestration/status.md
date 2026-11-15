# Delivery status

Updated 2026-11-15. This file tracks what is complete, what is in flight, and what remains before
the build can be called finished. It is not a changelog.

## Complete

| Area | State |
| --- | --- |
| Workspace, limits, versions, typed errors | Done, tested |
| CRDT document, commands, projection, local undo | Done, tested |
| Camera, brush, stroke geometry, hit testing | Done, tested |
| Grid renderer, stroke mesh cache | Done, tested |
| Portable file encode/decode/validate/remap, atomic write | Done, tested |
| Import placement ghost and single-command commit | Done |
| Native app shell, floating toolbox, zen mode | Done, screenshot-verified |
| Board presets, custom colors, grid sizing controls | Done, screenshot-verified |
| Windows pen pressure probe + mouse simulation | Done; hardware path needs a tablet to verify |
| AI card lifecycle, offline provider, pin flow | Done; streaming bug found and fixed in UI test |
| Mascot atlas animation, reduced motion | Done, tested |
| README, architecture, contracts, verification, decisions | Done |
| Local commits and GitHub checkpoints | In progress |

## In flight

1. **Excalidraw UI scoping** — browser-only research on how Excalidraw constructs its floating
   panels, tokens, typography, iconography, and interaction states. Output feeds the UI batch.
2. **Rune feasibility** — evaluate `anomalyco/rune` at the pinned dev revision against the current
   eframe/egui shell. Rune is a pre-release native Rust UI framework with its own renderer, so the
   decision is either a scoped migration or a recorded blocker; it is not a drop-in dependency.
3. **UI batch** — apply the scoping findings as one batch (chrome geometry, tokens, controls,
   polish), then build once and verify visually.

## Next

1. Finish Excalidraw scoping and Rune feasibility.
2. Implement the batched UI changes.
3. Reviewer pass over every UI surface and interaction: drawing, selection, move, marquee, group,
   undo/redo, import/export roundtrip, pin flow, settings, zen, close confirmation, reduced motion.
4. Fix review findings; rerun the full verification gate.
5. Push a final checkpoint and update this file with the outcome.

## Constraints in force

- No accounts, sign-ins, credentials, or network inference.
- No TURN/relay; no board bytes through a server.
- Boards stay in RAM; only explicit export writes.
- Do not leave test processes running; keep interactive test windows off the primary display.
- One writer per file while a batch is in flight.

## Known gaps

See [../verification.md](../verification.md) and [../../decisions.md](../../decisions.md) for the
authoritative list. Short version: no WebRTC transport, no real AI provider, pressure is Windows-only,
packaging/signing is absent, settings are session-only.
