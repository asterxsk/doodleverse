# Blueprint — Excalidraw token, icon and type pass

Status: authorized by the 2026-11-16 overnight instruction ("continue implementation and
polishing, then keep committing checkpoints"). No prototypes; the reference is the scoping
report at [../reference/excalidraw-ui-spec.md](../reference/excalidraw-ui-spec.md).

## Brief

The top-bar layout now matches Excalidraw's arrangement. This pass aligns the visual language:
indigo accent, radius-8 islands, 36×36 tool boxes with 16px stroke icons, Excalidraw surface and
text tokens in both themes, and tighter toolbar geometry. Behaviour, keys and existing surfaces
stay unchanged.

## Settled decisions

- **Palette** — adopt the spec's light/dark token table for bg, card, border, text, muted,
  accent, accent_ink, selection, hover, press, shadow. Board presets keep their own colours and
  override the palette bg as today.
- **Geometry** — islands radius 8 with 4pt padding; tool boxes 36×36 radius 8 gap 4; dividers
  1×24; swatches 22×22 radius 4; toolbar total height ~44; modal radius 12.
- **Icons** — draw the three tool icons (brush/select/hand) and the overflow ⋮ from Rune `Shape`
  primitives inside a 16×16 canvas. Simplified geometry is acceptable where the SVG arc data is
  impractical with Rune paths; recognisability beats literal path fidelity.
- **Typography** — toolbar labels 12px/600, menu rows 14px/400, hint text 12px. Keep the system
  font stack; do not bundle Assistant (licence unverified).
- **States** — instant hover (no transition), selected tool container `#e0dfff` light /
  `#403e6a` dark with icon `#030064` / `#e0dfff`, hover `#f1f0ff` / `#2e2d39`. Swatches ring the
  selected one in `#4a47b1` light / `#a8a5ff` dark (`Palette::swatch_ring`); the spec gives only
  the light value, and `selected_ink` was invisible on the dark Ink swatch.
- **Known deviations** (no API in Rune): the spec's pressed state ("hover bg + 1px `#4440bf`
  border") and the ⋮ trigger's active inset `#4a47b1` cannot be expressed — `El` has a pressed
  fill overlay and pressed text colour, but no pressed border. Pressed tools keep the `press`
  fill; the ⋮ trigger's hover stays transparent as specified, and its open state uses the
  selected container fill. The six brush preset colours are out of scope alongside the board
  presets: they are artwork defaults, not chrome tokens, so the old coral RGB survives as the
  "Coral" swatch while the accent role is `#6965db` / `#a8a5ff`.
- **Keys unchanged** — `tool-Draw`, `tool-Select`, `tool-Hand`, `undo`, `redo`, `menu-*`,
  `overflow-*`, `settings`, `fit`, `theme-*`, `board-*`, `slider-*` must all survive.

## Acceptance criteria

1. `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --locked -- -D warnings`
   pass.
2. Light theme uses accent `#6965db`; dark uses `#a8a5ff`. No coral accent remains in the Rune UI.
3. Islands render at radius 8, tool boxes at 36×36, swatches as 22×22 rounded squares.
4. Toolbar shows icons for the three tools and the ⋮ button shows three vertical dots.
5. Dark theme uses canvas `#121212`, island `#232329`, text `#e3e3e8`.
6. Every key above still exists and every headless scratch script still runs.
7. No layout regression at 1180×760 or 880×600.

## Non-goals

- No bundling of Assistant, no SVG path parser, no new dependencies.
- No behaviour changes, no new features, no eframe shell work.
- No menu structure changes beyond what the tokens require.

## Workflow

1. **Implement** — one agent applies tokens, geometry, icons, typography in
   `crates/doodle-app-rune/src/{theme,view,cards}.rs`; runs fmt + build.
2. **Review** — two parallel reviewers: spec compliance and regression (keys, dark theme, narrow
   widths, settings scroll, cards, modals).
3. **Fix** — one agent applies confirmed findings.
4. **Fresh test** — a new subagent that has never seen the app tests headless snapshots at 1180
   and 880 in both themes plus the popover and settings states.
