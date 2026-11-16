# Delivery status

Updated 2026-11-16. This file tracks what is complete, what is in flight, and what remains before
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
| eframe app shell, floating toolbox, zen mode | Done, screenshot-verified |
| Rune app shell (`doodleverse-rune`): islands, Excalidraw-style toolbox, zen | Done, headless-screenshot-verified |
| Rune Excalidraw-style top bar: board menu, centred icon toolbar, three-dot overflow | Done, headless-screenshot-verified |
| Excalidraw token/icon pass, accessibility semantics, instant popovers | Done, six fresh-tester QA rounds |
| Rune board theming, settings panel scrolling, dark-board ink | Done, headless-screenshot-verified |
| Rune file dialogs, export/import, pin flow, AI cards | Done, scripted-dialog-verified |
| Rune app icon, secondary-monitor placement, clean `WM_CLOSE` | Done, real-window-verified |
| Board presets, custom colors, grid sizing controls | Done, screenshot-verified |
| Windows pen pressure probe + mouse simulation | Done; hardware path needs a tablet to verify |
| AI card lifecycle, offline provider, pin flow | Done; streaming bug found and fixed in UI test |
| Mascot atlas animation, reduced motion | Done, tested |
| README, architecture, contracts, verification, decisions | Done |
| Local commits and GitHub checkpoints | In progress |

## In flight

Nothing. The Rune shell is implemented and the full gate passes; the remaining items are the
recorded product blockers (transport, providers, packaging).

## Next

1. Optional: a real-window reviewer pass over surfaces a headless script cannot reach — close
   confirmation buttons, native file dialogs, hardware pen pressure.
2. Optional: mirror any Rune-only UI fixes back into the eframe shell if it is kept.
3. Reviewer pass in a real window over the surfaces a headless script cannot reach: close
   confirmation buttons, native file dialogs, hardware pen pressure.

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
