# Excalidraw UI spec for Doodleverse

Source: delegated OpenCode / DeepSeek scoping run, 2026-11-16. All numbers measured from the live
app at excalidraw.com (DOM `getBoundingClientRect` + computed styles) and its shipped compiled
stylesheet `index-VRsybfDG.css` (213,705 bytes, 1,605 rules). Confidence is high except where
flagged. Saved here as the reference for the Rune UI token/icon pass.

## 1. Layout: floating islands

One canvas, five overlay clusters. Everything floats on `--zIndex-layerUI: 4` (canvas 1, popups
1001, modal 1000, toast 999999).

Top row is a 3-column grid:

```
.App-menu_top { position:absolute; inset: 16px 16px auto 16px;
                display:grid; grid-template-columns: 1fr 2fr 1fr; grid-gap: 1rem; }
@min-width 1536px -> 1fr 1fr 1fr; grid-gap: 3rem (48px)
```

| Island | Anchor | Size at 1600×1000 | Notes |
|---|---|---|---|
| Hamburger / main menu trigger | top-left, col 1 | 36×36 | radius 8, fill `#ececf4`, 1px white inset highlight |
| Shape selection panel | left, top 76px | 195×615, max-width 200px | scrolls, padding 12 |
| Toolbar | top-center, col 2 | **542×44** | only real island in the middle |
| Top-right actions | top-right, col 3, end, gap 12 | plus-banner 147×36, Share 54×36, Library 36×36 | |
| Bottom bar | bottom 16, padding 0 16, space-between | left cluster 168×36 · shield 19×19 · help 36×36 | |

Island recipe: bg `--island-bg-color`; radius 8px; shadow
`0 0 1px rgba(0,0,0,.17), 0 0 3px rgba(0,0,0,.08), 0 7px 14px rgba(0,0,0,.05)`; padding 4px
(toolbar/menus; menus 8px). Dark menus add `0 0 0 1px #00000026`.

Toolbar left→right: `lock(36) divider(1×24) hand selection rect diamond ellipse arrow line draw
text sticky eraser ⋮(36)`. Tools 36×36, radius 8, gap 4. Order confirmed live.

Responsive:

| Width | Behaviour |
|---|---|
| ≥1536 | grid 1fr 1fr 1fr, gap 48 |
| <1536 | grid 1fr 2fr 1fr, gap 16 |
| ≤1200 | Share collapses to icon-only |
| ≤1000 | plus banner hidden, toolbar shrinks to 514 |
| 700→620 | toolbar stays centered but clamps left edge at x=80 so it never overlaps the hamburger |
| ≤~610 | mobile: toolbar bottom, 450 wide; hamburger 32 at 12,12; library 32 at right 12; zoom/minimap/help/undo/redo removed; undo/redo/Delete float above toolbar at y=904, 32×32 right-aligned |

Mobile toolbar: flex, gap 4, space-between, tools min 32×32 radius 4, no hover, keybindings hidden,
divider 1×24 margin 0 2. Tool set: hand, selection, freedraw, eraser, rectangle, arrow, text,
image, ⋮. The switch is JS-driven, observed between 610 and 620 CSS px.

## 2. Design tokens

### Light

| Role | Value |
|---|---|
| Canvas | `#ffffff` |
| Island fill | `#ffffff` |
| Surface low / hovered | `#ececf4` |
| Surface mid | `#f6f6f9` |
| Surface high | `#f1f0ff` |
| Border / divider | `#f1f0ff` |
| Dialog border | `#ebebeb` |
| Text primary | `#1b1b1f` |
| Muted / placeholder | `#c5c5d0`, `#d6d6d6`, `#7a7a7a` |
| Disabled | `#b8b8b8` |
| Keybinding badge | `#b8b8b8` |
| Accent / primary | `#6965db` |
| Primary darker / darkest | `#5b57d1` / `#4a47b1` |
| Brand hover / active | `#5753d0` / `#4440bf` |
| Primary light (menu selection) | `#e3e2fe` |
| Selected tool container | `#e0dfff` |
| On-primary-container | `#030064` |
| Button hover bg | `#f1f0ff` |
| Selection colour | `#6965db` |
| Select highlight | `#339af0` |
| Focus ring | `#a5d8ff` 2px |
| Modal scrim | `rgba(12,12,14,0.35)` |
| Danger | `#db6965` |
| Link | `#1c7ed6` |

### Dark

| Role | Value |
|---|---|
| Canvas | `#121212` |
| Island fill | `#232329` |
| Surface low / hover | `#2e2d39` |
| Surface mid | `hsl(240 6% 10%)` |
| Border / divider | `#2e2d39` |
| Dialog border | `#3d3d3d` |
| Text primary | `#e3e3e8` |
| Disabled | `#5c5c5c` |
| Keybinding badge | `#7a7a7a` |
| Accent / primary | `#a8a5ff` |
| Primary light | `#4f4d6f` |
| Selected tool container | `#403e6a` |
| On-primary-container | `#e0dfff` |
| Modal scrim | `rgba(12,12,14,0.65)` |

### Geometry

radius sm/md 6px (menu rows), lg 8px (islands, buttons, tool icons), modal 12px, swatch 4px,
gap unit 4px, icon 16px inside 36px box.

## 3. Typography

- Family Assistant (self-hosted, 400/500/600/700), fallback system-ui/Segoe UI/Roboto/Helvetica.
  Inter at ~1px smaller is the closest free stand-in.
- Toolbar keybinding badge 10px, `#b8b8b8`, absolute bottom 2 / right 3.
- Menu rows 14px/400; shortcut text 14px at opacity .5 pushed right.
- Menu group titles 12px/400.
- Hint viewer 12px muted, margin-top 8; kbd 10px monospace, 1px border `#b8b8b8`, radius 4,
  padding 1px 3px.
- Dialog title 21px/700; section labels 16px/600; field label 14px/600; input text 16px/400.
- Buttons 12px/600. Tooltip 13px/500 white on #000, padding 8, radius 6.
- Zen-exit button 12px/500.

## 4. Toolbox anatomy

```
ToolIcon                     inline-flex, radius 8, transparent
  .ToolIcon__icon            36×36, radius 8, centered
    svg                      16×16, stroke currentColor, round caps/joins
  .ToolIcon__keybinding      10px, absolute bottom 2 / right 3
```

| State | Icon container | Icon |
|---|---|---|
| rest | transparent | `#1b1b1f` / `#e3e3e8` |
| hover | `#f1f0ff` / `#2e2d39` | unchanged |
| checked | `#e0dfff` / `#403e6a` | `#030064` / `#e0dfff` |
| active/pressed | hover bg + 1px `#4440bf` border | on-primary-container |
| disabled | none | `#b8b8b8`, no pointer |

- Eraser inverts to white icon on hover; active fills with accent.
- Lock (keep-tool-active) first, same 36 box.
- Focus ring `0 0 0 2px #a5d8ff`.
- Dividers 1×24 (1.5rem), centered.
- Extra-tools ⋮ is text-less, transparent, hover stays transparent, active gets hover bg + 1px
  inset `#4a47b1`; selected-tool state gets `#e0dfff`.
- Swatches 22×22 radius 4, outline ring when needed, active inner ring `#4a47b1` at -1px inset;
  large 30×30 radius 6; collapsed trigger 26×26 radius 5; focus ring 3px `#a5d8ff`, offset 4.
  Top picks: `#1e1e1e #e03131 #2f9e44 #1971c2 #f08c00 #ffc9c9 #b2f2bb #a5d8ff`.
- Shortcuts: H hand, V selection, R rect, D diamond, O ellipse, A arrow, L line, P draw, T text,
  N sticky, E eraser, Q lock.

## 5. Menus and dialogs

Dropdown: max-width 320, island bg, column gap 1, overflow-y auto, max-height
`calc(100svh - 32px - 2.25rem)`, padding 8, radius 8. Items 32px tall, padding 0 8, radius 6,
14px/400, gap 10, icon 16. Hover `#f1f0ff`; active hover + 1px `#4440bf`; selected
`#e3e2fe` with darker icon; disabled opacity .5. Shortcut pushed right at opacity .5. Separator
1px with margin 6 0. Popover opens 8px away from trigger.

Hamburger order: Open (Ctrl+O), Save to..., Export image... (Ctrl+Shift+E), Live collaboration...,
Command palette (Ctrl+/) in accent 700, Find on canvas (Ctrl+F), Help (?), Reset the canvas,
separator, Excalidraw+, GitHub, Follow us, Discord chat, Sign up (accent), separator,
Preferences ▸.

Preferences submenu: Select on wrap/overlap, Input trackpad/mouse, toggles Tool lock (Q), Snap to
objects (Alt+S), Toggle grid (Ctrl+'), Zen mode (Alt+Z), View mode (Alt+R), Minimap, Canvas & Shape
properties (Alt+/), checked Arrow binding, Snap to midpoints, Toolbar hints.

Top right: Try Excalidraw+ banner, Share, Library. Footer: zoom text `100%` 60×36 no icon, minimap
36, undo, redo; center shield; right help `?`.

Dialogs: scrim fixed; content max-width, island bg, 1px dialog border, radius 12, stacked shadow;
modal padding 40; close 16 from top/right, 24×24 icon. Buttons radius 8, 1px border, 12px/600;
primary filled `#6965db` hover `#5753d0` active `#4440bf`; danger filled; disabled surface-low.
Inputs 48 tall, bg `#ececf4`, border `#ebebeb`, radius 8, padding 0 12; focus border `#4440bf`;
search icon 20 at left 12. Switches 16, radios 22.

## 6. Zen mode

- Alt+Z. Root class `excalidraw--zen-mode`.
- Remains: toolbar (interactive), hamburger trigger, footer-left zoom/minimap/undo/redo, exit
  affordance.
- Hidden: top-right cluster, footer centre shield, footer right help, selection-panel rail, and
  inside the toolbar the keybinding badges and hint viewer.
- Chrome melts: toolbar and zoom actions lose background and shadow; hamburger loses border/shadow
  until hover/focus.
- Zoom collapses: +/- hidden, reset becomes a pill and shifts left, only `100%` remains.
- Exit button "Exit zen mode" bottom-right 16, padding 10, 12px/500, 1px border, island bg, radius
  8, appears with `transition-delay: .8s` at 250ms total.
- Transitions 250ms ease-in-out, opacity/visibility/background/border/box-shadow.

## 7. Icon paths

All 16×16 drawn on screen, `fill:none`, `stroke:currentColor`, round caps/joins. ViewBox 24 unless
noted; stroke 1.5 shapes, 1.25 selection/hand/freedraw/undo/redo/minimap, 1.75 library.

| Icon | vb | Paths |
|---|---|---|
| Selection | 22 | `M6 6l4.153 11.793a.365.365 0 0 0 .331.207.366.366 0 0 0 .332-.207l2.184-4.793 4.787-1.994a.355.355 0 0 0 .213-.323.355.355 0 0 0-.213-.323L6 6z` + `M13.5 13.5l4.5 4.5` |
| Hand | 24 | `M8 13V5.5a1.5 1.5 0 0 1 3 0V12` · `M11 5.5v-2a1.5 1.5 0 1 1 3 0V12` · `M14 5.5a1.5 1.5 0 0 1 3 0V12` · `M17 7.5a1.5 1.5 0 0 1 3 0V16a6 6 0 0 1-6 6h-2 .208a6 6 0 0 1-5.012-2.7 69.74 69.74 0 0 1-.196-.3c-.312-.479-1.407-2.388-3.286-5.728a1.5 1.5 0 0 1 .536-2.022 1.867 1.867 0 0 1 2.28.28l1.47 1.47` |
| Rectangle | 24 | `<rect x=4 y=4 width=16 height=16 rx=2>` |
| Diamond | 24 | `M10.5 20.4l-6.9-6.9c-.781-.781-.781-2.219 0-3l6.9-6.9c.781-.781 2.219-.781 3 0l6.9 6.9c.781.781.781 2.219 0 3l-6.9 6.9c-.781.781-2.219.781-3 0z` |
| Ellipse | 24 | `<circle cx=12 cy=12 r=9>` |
| Arrow | 24 | `M5 12h14` · `M15 16l4-4` · `M15 8l4 4` |
| Line | 20 | `M4.167 10h11.666` |
| Draw | 20 | `m7.643 15.69 7.774-7.773a2.357 2.357 0 1 0-3.334-3.334L4.31 12.357a3.333 3.333 0 0 0-.977 2.357v1.953h1.953c.884 0 1.732-.352 2.357-.977Z` + `m11.25 5.417 3.333 3.333` |
| Text | 24 | polyline `5 20 11 4 13 4 20 20` + `M4 20h3` + `M14 20h7` + `M6.9 15h6.9` + `M10.2 6.3 16 20` |
| Sticky | 24 | `M6 4h12a2 2 0 0 1 2 2v9l-5 5H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2z` + `M14 20v-4a2 2 0 0 1 2-2h4` + `M8 9h8` + `M8 13h5` |
| Image | 20 | `M12.5 6.667h.01` + `M4.91 2.625h10.18a2.284 2.284 0 0 1 2.285 2.284v10.182a2.284 2.284 0 0 1-2.284 2.284H4.909a2.284 2.284 0 0 1-2.284-2.284V4.909a2.284 2.284 0 0 1 2.284-2.284Z` + `m3.333 12.5 3.334-3.333c.773-.745 1.726-.745 2.5 0l4.166 4.166` + `m11.667 11.667.833-.834c.774-.744 1.726-.744 2.5 0l1.667 1.667` |
| Eraser | 24 | `M19 20H8.5l-4.21-4.3a1 1 0 0 1 0-1.41l10-10a1 1 0 0 1 1.41 0l5 5a1 1 0 0 1 0 1.41l-9.2 9.3` + `M18 13.3l-6.3-6.3` |
| Undo | 20 | `M7.5 10.833 4.167 7.5 7.5 4.167M4.167 7.5h9.166a3.333 3.333 0 0 1 0 6.667H12.5` |
| Redo | 20 | `M12.5 10.833 15.833 7.5 12.5 4.167M15.833 7.5H6.667a3.333 3.333 0 1 0 0 6.667H7.5` |
| Hamburger | 24 | lines x4→20 at y 6, 12, 18 |
| More | 24 | circles r1 at (12,5), (12,12), (12,19) |
| Help | 24 | `<circle cx=12 cy=12 r=9>` + `M12 17h.01` + `M12 13.5a1.5 1.5 0 0 1 1-1.5 2.6 2.6 0 1 0-3-4` |
| Library | 24 | `M4 4m0 2a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2z` + `M15 4v16` |
| Zoom | — | text button showing percentage, 60×36, left corners radius 8 |
| Minimap | 24 | `M3 7l6-3 6 3 6-3v13l-6 3-6-3-6 3V7` + `M9 4v13` + `M15 7v13` |

## 8. Motion

- Tool hover/checked/active: instant (`transition: none`).
- Menus mount instantly.
- Hamburger and zoom triggers: opacity/visibility 250ms.
- Zen mode: 250ms ease-in-out on opacity/visibility/background/border/shadow.
- Zen exit button: opacity 500ms after 800ms delay.
- Modal scrim fade 100ms; content fade 25ms from translateY(10px).
- ExcButton hover 150ms ease-out.
- Reduced motion disables animations.

## 9. Rune verdict

Rune exists, is pre-release, is a full UI framework (own wgpu renderer, Parley text, Taffy layout,
AccessKit), not an egui widget library. No egui interop. Pin a `dev` rev. Do not confuse with the
crates.io `rune` language crate. This repo already uses Rune directly, which matches that verdict.

## 10. Unverified

- Styled tooltip visibility under automation (toolbar hints may be native `title`).
- Menu row hover transition timing (read from CSS, not observed).
- Dark island border reading as border vs shadow.
- Mobile breakpoint is JS-driven; width-only ports differ on touch laptops.
- GitHub source tree unavailable (rate limits/login/404); all values from shipped CSS.
- Zoom pill corner radii rule not located.
- Assistant font redistribution licensing not checked.
