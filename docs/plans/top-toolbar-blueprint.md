# Blueprint — Excalidraw-style top bar for the Rune shell

Status: implemented in `1faf8be` and pushed to `main`. Headless snapshots at 1180×760 and 880×600
confirm the layout: menu island top-left, centred toolbar, three-dot island top-right, both
popovers render, no collision at the minimum window width. Ask Dibby and Export leave the menu bar
below 1160pt as designed.

## Brief

Rework the Rune UI chrome to match Excalidraw's arrangement:

- A normal horizontal toolbar at the top instead of the left-pinned vertical tool island.
- The main menu stays at the top left (Doodleverse / Ask Dibby / Export).
- The top-right stack (Group, Ungroup, Fit, Settings) collapses into one three-dot menu.
- No other floating clusters beyond the existing bottom status and zoom pills.

## Settled decisions

- **Toolbar contents:** Brush, Select, Hand, divider, brush size label + slider, one row of six
  color swatches, divider, Undo, Redo. Existing element keys stay unchanged so headless scripts
  and muscle memory keep working.
- **Three-dot menu contents:** Group, Ungroup, Fit to content, Canvas settings, Shortcuts & about,
  Hide UI (Alt+Z). It opens a popover anchored below the button; a full-window scrim closes it.
- **State:** a new `overflow_open` flag next to `menu_open`; entering zen closes both, and the
  scrim closes both.
- **Placement:** top-left menu bar at `(12, 10)`, toolbar centered on the top edge at `y = 10`,
  three-dot island pinned top-right at `(10, 12)`. The left bar drops the Ask Dibby / Export
  buttons below 1000pt window width so the centered toolbar never collides with it.
- **Bottom islands stay:** status at bottom-left, zoom at bottom-right.
- **Theme helpers only:** reuse `theme::island`, `theme::active_button`, `theme::flat_button`,
  `theme::swatch`; add a vertical divider helper if needed.

## Acceptance criteria

1. `cargo fmt --all -- --check` passes.
2. `cargo build -p doodle-app-rune` passes with no new warnings.
3. No left `tool_island` or right `action_island` remains in the view tree.
4. Toolbar renders horizontally: three tools, size slider, six swatches, undo/redo.
5. Three-dot menu opens and every previous action is reachable; scrim and Esc close it.
6. Zen hides toolbar, menu bar, overflow button, status and zoom; a hint remains.
7. Existing keys (`tool-Draw`, `tool-Select`, `tool-Hand`, `undo`, `redo`, `menu-*`, `settings`,
   `fit`, `theme-*`, `board-*`, `slider-*`) are unchanged.
8. Settings panel, AI cards, close/pin modals and canvas input are untouched.

## Scoping input (arrived mid-implementation)

The delegated Excalidraw scoping report landed and is saved at
[../reference/excalidraw-ui-spec.md](../reference/excalidraw-ui-spec.md). It confirms: toolbar
542×44 centered with 4px gaps and 36px tool boxes; islands radius 8 with a three-layer shadow;
accent `#6965db`; 16px stroke icons; instant hover states.

It also shows Excalidraw keeps its own ⋮ inside the toolbar and a Share/Library cluster top-right.
The user explicitly asked for the three-dot menu at the top right holding Group/Ungroup/Fit/Settings,
so the user's arrangement wins over Excalidraw's exact composition. Token and icon alignment
(indigo accent, radius 8, real SVG icons) is a follow-up pass, not part of this workflow.

## Non-goals

- No new icon set, no theme/token overhaul, no animation changes.
- No app launch, mouse use, or screenshot testing while the user is at the computer.
- No commits from workflow agents; the orchestrator reviews and commits.

## Workflow

1. **Implement** — one agent refactors `view.rs` / `app.rs`, runs fmt + build.
2. **Review** — three parallel reviewers: layout math/narrow widths, interaction and state bugs,
   regression of keys and existing surfaces. Structured findings.
3. **Fix** — one agent applies the confirmed findings, re-runs fmt + build.

Run ID `wf_babce087-b83`. Deferred until the user leaves: headless screenshots, real-window
checks, full workspace gate, commit and push.
