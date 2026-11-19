# Icon set selection for the Doodleverse toolbar

Status: research complete, no code written.
Date: 2026-11-19.
Method: official repositories and license files fetched through Exa. Path data below was taken verbatim from the official SVG assets at commit `10ecc14790612ca8fd164e3f4ffe9c16fc50d5b7` (2026-10-23) of `google/material-design-icons`; no paths were hand-authored, simplified, or rescaled.

## 1. Recommendation

**Material Symbols Rounded, filled (`FILL 1`), 24 px optical size.**

Why this set:

- **Strong silhouettes.** Filled Material Symbols are solid shapes, not 2 px outlines. At 16-20 px they read as distinct tool marks, which directly fixes the "Lucide feels weak/undifferentiated" problem. Lucide, Tabler and Iconoir are all outline-first systems with 1.5-2 px strokes; swapping Lucide for another outline set would not change the visual result.
- **Complete coverage for a whiteboard toolset.** Real glyphs exist for every requested tool, including whiteboard-specific ones: `pan_tool`, `arrow_selector_tool`, `sticky_note_2`, `ink_eraser`, `lock`, `map`, `auto_awesome`.
- **One family, one grid.** All 27 glyphs below come from the same stylistic set (`materialsymbolsrounded`), so visual weight is consistent. Unfilled counterparts exist in the same directories (drop `_fill1` from the filename) if a quieter state is ever needed.
- **Permissive license.** Apache License 2.0, with no attribution required in the UI. The repository has no `NOTICE` file.
- **Official, copy-pasteable SVG.** Every asset is a single `<path>` element; no font extraction, no third-party CDN.

Costs, stated plainly:

- Material Symbols uses a 960-unit coordinate grid (`viewBox="0 -960 960 960"`), not a literal 24-unit grid. The asset is still a 24 px asset. A renderer that honors `viewBox` needs no conversion; a renderer that assumes `0 0 24 24` needs one wrapper transform (see §2.1).
- The style is recognizably Material. For a neutral/coss-UI chrome that is a feature (it reads as a coherent system), but it is not a boutique icon look.
- There is no `ellipse` glyph; `circle` is the substitute for the ellipse tool.

- License: **Apache License 2.0**
- Source: <https://github.com/google/material-design-icons>
- Browsable index: <https://fonts.google.com/icons> (set: Material Symbols, style: Rounded)
- Asset path pattern: `symbols/web/<name>/materialsymbolsrounded/<name>_fill1_24px.svg`
- Documentation: <https://developers.google.com/fonts/docs/material_symbols>
- Pinned revision: `10ecc14790612ca8fd164e3f4ffe9c16fc50d5b7`

Required attribution line (About / third-party notices):

```text
Material Symbols by Google, licensed under the Apache License 2.0.
https://github.com/google/material-design-icons
```

Apache-2.0 does not require UI attribution, but it does require that recipients receive a copy of the license and that copyright/license notices are retained. Vendor the license file; see §4.

## 2. Glyph path data

### 2.1 Rendering contract

All glyphs below are **fill-based**: one `<path>`, no `stroke` attributes, default **nonzero** fill rule. Render with `fill="currentColor"` and either no stroke or a zero-width stroke. Do **not** switch to `evenodd`; several glyphs (settings, help, lock, sticky note) depend on nonzero winding for their counters.

Default viewBox for this set: `0 -960 960 960`, nominal size 24x24. Exception: `auto_awesome`, whose official 24 px asset uses a 24-unit path and no viewBox — use `viewBox="0 0 24 24"` for it.

If the Rust path pipeline only understands a 24-unit space, wrap each 960-unit path once:

```svg
<svg viewBox="0 0 24 24">
  <g transform="scale(0.025) translate(0 960)">
    <path d="..." fill="currentColor"/>
  </g>
</svg>
```

Do not apply that transform to `auto_awesome`.

### 2.2 Tool mapping

| Requested glyph | Material Symbol | Note |
|---|---|---|
| hand | `pan_tool` | filled hand, pan tool |
| pointer / select | `arrow_selector_tool` | cursor plus selection corner |
| square / rectangle | `rectangle` | `square` also exists if a squarer mark is wanted |
| diamond | `diamond` | filled diamond |
| circle / ellipse | `circle` | no `ellipse` glyph; circle is the substitute |
| arrow | `north_east` | diagonal arrow, matches Excalidraw's arrow tool |
| line | `line_start` | horizontal line with endpoint dot; distinct from minus |
| pencil / pen | `draw` | pen nib; `edit` is the pencil alternative (path included) |
| text / type | `title` | capital T; `text_fields` is the wider alternative |
| sticky note | `sticky_note_2` | note with folded corner and ruled lines |
| eraser | `ink_eraser` | eraser with ink stroke |
| lock / locked | `lock` | closed padlock |
| menu / hamburger | `menu` | three rules |
| ellipsis / more | `more_horiz` | three dots |
| undo | `undo` | curved back arrow |
| redo | `redo` | curved forward arrow |
| plus | `add` | |
| minus | `remove` | |
| settings / gear | `settings` | |
| map / minimap | `map` | folded map |
| help / question | `help` | circled question mark |
| export / download | `download` | tray plus down arrow |
| sparkles / AI | `auto_awesome` | three sparkles; 24-unit path |
| close / x | `close` | |
| check | `check` | |

### 2.3 Tool glyphs

**Hand / pan** — `pan_tool` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M402-40q-30 0-56-13.5T303-92L67-438q-8-12-7-26t12-24q19-19 45-22t47 12l116 81v-383q0-17 11.5-28.5T320-840q17 0 28.5 11.5T360-800v320h80v-400q0-17 11.5-28.5T480-920q17 0 28.5 11.5T520-880v400h80v-360q0-17 11.5-28.5T640-880q17 0 28.5 11.5T680-840v360h80v-280q0-17 11.5-28.5T800-800q17 0 28.5 11.5T840-760v560q0 66-47 113T680-40H402Z"/>
```

**Pointer / select** — `arrow_selector_tool` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M606-105q-23 11-46 2.5T526-134L406-392l-93 130q-17 24-45 15t-28-38v-513q0-25 22.5-36t42.5 5l404 318q23 17 13.5 44T684-440H516l119 255q11 23 2.5 46T606-105Z"/>
```

**Rectangle** — `rectangle` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M160-160q-33 0-56.5-23.5T80-240v-480q0-33 23.5-56.5T160-800h640q33 0 56.5 23.5T880-720v480q0 33-23.5 56.5T800-160H160Z"/>
```

**Diamond** — `diamond` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M368-630h224L486-840h-12L368-630Zm82 474v-414H105l345 414Zm60 0 345-414H510v414Zm148-474h207l-83-166q-11-20-29.5-32T711-840H554l104 210Zm-563 0h207l104-210H249q-23 0-41.5 12T178-796L95-630Z"/>
```

**Circle / ellipse** — `circle` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M480-80q-83 0-156-31.5T197-197q-54-54-85.5-127T80-480q0-83 31.5-156T197-763q54-54 127-85.5T480-880q83 0 156 31.5T763-763q54 54 85.5 127T880-480q0 83-31.5 156T763-197q-54 54-127 85.5T480-80Z"/>
```

**Arrow** — `north_east` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M680-624 244-188q-11 11-28 11t-28-11q-11-11-11-28t11-28l436-436H400q-17 0-28.5-11.5T360-720q0-17 11.5-28.5T400-760h320q17 0 28.5 11.5T760-720v320q0 17-11.5 28.5T720-360q-17 0-28.5-11.5T680-400v-224Z"/>
```

**Line** — `line_start` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M180-380q-42 0-71-29t-29-71q0-42 29-71t71-29q31 0 56 17t36 43h568q17 0 28.5 11.5T880-480q0 17-11.5 28.5T840-440H272q-11 26-36 43t-56 17Z"/>
```

**Pen / freehand draw** — `draw` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M200-120q-17 0-28.5-11.5T160-160v-97q0-16 6-30.5t17-25.5l504-503q12-12 27-18t30-6q16 0 30.5 6t25.5 18l56 56q12 11 18 25.5t6 30.5q0 15-6 30t-18 27L353-143q-11 11-25.5 17t-30.5 6h-97Zm546-529 54-54-57-57-54 54 57 57ZM560-120q74 0 137-37t63-103q0-32-16-55.5T702-359q-14-10-30-10t-27 12q-11 12-11 29.5t14 27.5q14 11 23 20t9 20q0 23-36.5 41.5T560-200q-17 0-28.5 11.5T520-160q0 17 11.5 28.5T560-120ZM360-720q0 14-17.5 25.5T262-654q-80 35-111 63.5T120-520q0 26 12 46t31 35q13 11 29 9.5t27-14.5q11-13 10-29t-14-27q-7-5-11-10t-4-10q0-12 18-24t76-37q88-38 117-69t29-70q0-55-44-87.5T280-840q-45 0-80.5 16T145-785q-11 13-9 29t15 26q13 11 29 9t27-13q14-14 31-20t42-6q41 0 60.5 12t19.5 28Z"/>
```

**Pencil (alternative)** — `edit` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M160-120q-17 0-28.5-11.5T120-160v-97q0-16 6-30.5t17-25.5l505-504q12-11 26.5-17t30.5-6q16 0 31 6t26 18l55 56q12 11 17.5 26t5.5 30q0 16-5.5 30.5T817-647L313-143q-11 11-25.5 17t-30.5 6h-97Zm544-528 56-56-56-56-56 56 56 56Z"/>
```

**Text / type** — `title` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M420-680H260q-25 0-42.5-17.5T200-740q0-25 17.5-42.5T260-800h440q25 0 42.5 17.5T760-740q0 25-17.5 42.5T700-680H540v460q0 25-17.5 42.5T480-160q-25 0-42.5-17.5T420-220v-460Z"/>
```

**Sticky note** — `sticky_note_2` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="m560-200 200-200H600q-17 0-28.5 11.5T560-360v160Zm-360 80q-33 0-56.5-23.5T120-200v-560q0-33 23.5-56.5T200-840h560q33 0 56.5 23.5T840-760v367q0 16-6 30.5T817-337L623-143q-11 11-25.5 17t-30.5 6H200Zm120-280h120q17 0 28.5-11.5T480-440q0-17-11.5-28.5T440-480H320q-17 0-28.5 11.5T280-440q0 17 11.5 28.5T320-400Zm0-160h320q17 0 28.5-11.5T680-600q0-17-11.5-28.5T640-640H320q-17 0-28.5 11.5T280-600q0 17 11.5 28.5T320-560Z"/>
```

**Eraser** — `ink_eraser` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M690-240h150q17 0 28.5 11.5T880-200q0 17-11.5 28.5T840-160H610l80-80Zm-483 80q-8 0-15.5-3t-13.5-9l-73-73q-23-23-23.5-57t22.5-58l440-456q23-24 56.5-24t56.5 23l199 199q23 23 23 57t-23 57L532-172q-6 6-13.5 9t-15.5 3H207Z"/>
```

### 2.4 Actions and chrome

**Lock** — `lock` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M240-80q-33 0-56.5-23.5T160-160v-400q0-33 23.5-56.5T240-640h40v-80q0-83 58.5-141.5T480-920q83 0 141.5 58.5T680-720v80h40q33 0 56.5 23.5T800-560v400q0 33-23.5 56.5T720-80H240Zm240-200q33 0 56.5-23.5T560-360q0-33-23.5-56.5T480-440q-33 0-56.5 23.5T400-360q0 33 23.5 56.5T480-280ZM360-640h240v-80q0-50-35-85t-85-35q-50 0-85 35t-35 85v80Z"/>
```

**Menu / hamburger** — `menu` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M160-240q-17 0-28.5-11.5T120-280q0-17 11.5-28.5T160-320h640q17 0 28.5 11.5T840-280q0 17-11.5 28.5T800-240H160Zm0-200q-17 0-28.5-11.5T120-480q0-17 11.5-28.5T160-520h640q17 0 28.5 11.5T840-480q0 17-11.5 28.5T800-440H160Zm0-200q-17 0-28.5-11.5T120-680q0-17 11.5-28.5T160-720h640q17 0 28.5 11.5T840-680q0 17-11.5 28.5T800-640H160Z"/>
```

**Ellipsis / more** — `more_horiz` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M240-400q-33 0-56.5-23.5T160-480q0-33 23.5-56.5T240-560q33 0 56.5 23.5T320-480q0 33-23.5 56.5T240-400Zm240 0q-33 0-56.5-23.5T400-480q0-33 23.5-56.5T480-560q33 0 56.5 23.5T560-480q0 33-23.5 56.5T480-400Zm240 0q-33 0-56.5-23.5T640-480q0-33 23.5-56.5T720-560q33 0 56.5 23.5T800-480q0 33-23.5 56.5T720-400Z"/>
```

**Undo** — `undo` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M320-200q-17 0-28.5-11.5T280-240q0-17 11.5-28.5T320-280h244q63 0 109.5-40T720-420q0-60-46.5-100T564-560H312l76 76q11 11 11 28t-11 28q-11 11-28 11t-28-11L188-572q-6-6-8.5-13t-2.5-15q0-8 2.5-15t8.5-13l144-144q11-11 28-11t28 11q11 11 11 28t-11 28l-76 76h252q97 0 166.5 63T800-420q0 94-69.5 157T564-200H320Z"/>
```

**Redo** — `redo` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M648-560H396q-63 0-109.5 40T240-420q0 60 46.5 100T396-280h244q17 0 28.5 11.5T680-240q0 17-11.5 28.5T640-200H396q-97 0-166.5-63T160-420q0-94 69.5-157T396-640h252l-76-76q-11-11-11-28t11-28q11-11 28-11t28 11l144 144q6 6 8.5 13t2.5 15q0 8-2.5 15t-8.5 13L628-428q-11 11-28 11t-28-11q-11-11-11-28t11-28l76-76Z"/>
```

**Plus** — `add` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M440-440H240q-17 0-28.5-11.5T200-480q0-17 11.5-28.5T240-520h200v-200q0-17 11.5-28.5T480-760q17 0 28.5 11.5T520-720v200h200q17 0 28.5 11.5T760-480q0 17-11.5 28.5T720-440H520v200q0 17-11.5 28.5T480-200q-17 0-28.5-11.5T440-240v-200Z"/>
```

**Minus** — `remove` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M240-440q-17 0-28.5-11.5T200-480q0-17 11.5-28.5T240-520h480q17 0 28.5 11.5T760-480q0 17-11.5 28.5T720-440H240Z"/>
```

**Settings / gear** — `settings` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M433-80q-27 0-46.5-18T363-142l-9-66q-13-5-24.5-12T307-235l-62 26q-25 11-50 2t-39-32l-47-82q-14-23-8-49t27-43l53-40q-1-7-1-13.5v-27q0-6.5 1-13.5l-53-40q-21-17-27-43t8-49l47-82q14-23 39-32t50 2l62 26q11-8 23-15t24-12l9-66q4-26 23.5-44t46.5-18h94q27 0 46.5 18t23.5 44l9 66q13 5 24.5 12t22.5 15l62-26q25-11 50-2t39 32l47 82q14 23 8 49t-27 43l-53 40q1 7 1 13.5v27q0 6.5-2 13.5l53 40q21 17 27 43t-8 49l-48 82q-14 23-39 32t-50-2l-60-26q-11 8-23 15t-24 12l-9 66q-4 26-23.5 44T527-80h-94Zm49-260q58 0 99-41t41-99q0-58-41-99t-99-41q-59 0-99.5 41T342-480q0 58 40.5 99t99.5 41Z"/>
```

**Map / minimap** — `map` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="m574-129-214-75-186 72q-10 4-19.5 2.5T137-136q-8-5-12.5-13.5T120-169v-561q0-13 7.5-23t20.5-15l186-63q6-2 12.5-3t13.5-1q7 0 13.5 1t12.5 3l214 75 186-72q10-4 19.5-2.5T823-824q8 5 12.5 13.5T840-791v561q0 13-7.5 23T812-192l-186 63q-6 2-12.5 3t-13.5 1q-7 0-13.5-1t-12.5-3Zm-14-89v-468l-160-56v468l160 56Z"/>
```

**Help / question** — `help` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M478-240q21 0 35.5-14.5T528-290q0-21-14.5-35.5T478-340q-21 0-35.5 14.5T428-290q0 21 14.5 35.5T478-240Zm2 160q-83 0-156-31.5T197-197q-54-54-85.5-127T80-480q0-83 31.5-156T197-763q54-54 127-85.5T480-880q83 0 156 31.5T763-763q54 54 85.5 127T880-480q0 83-31.5 156T763-197q-54 54-127 85.5T480-80Zm4-572q25 0 43.5 16t18.5 40q0 22-13.5 39T502-525q-23 20-40.5 44T444-427q0 14 10.5 23.5T479-394q15 0 25.5-10t13.5-25q4-21 18-37.5t30-31.5q23-22 39.5-48t16.5-58q0-51-41.5-83.5T484-720q-38 0-72.5 16T359-655q-7 12-4.5 25.5T368-609q14 8 29 5t25-17q11-15 27.5-23t34.5-8Z"/>
```

**Export / download** — `download` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M480-337q-8 0-15-2.5t-13-8.5L308-492q-12-12-11.5-28t11.5-28q12-12 28.5-12.5T365-549l75 75v-286q0-17 11.5-28.5T480-800q17 0 28.5 11.5T520-760v286l75-75q12-12 28.5-11.5T652-548q11 12 11.5 28T652-492L508-348q-6 6-13 8.5t-15 2.5ZM240-160q-33 0-56.5-23.5T160-240v-80q0-17 11.5-28.5T200-360q17 0 28.5 11.5T240-320v80h480v-80q0-17 11.5-28.5T760-360q17 0 28.5 11.5T800-320v80q0 33-23.5 56.5T720-160H240Z"/>
```

**Sparkles / AI** — `auto_awesome` — fill, `viewBox="0 0 24 24"` (the only 24-unit path in this list)

```svg
<path d="M19 8.3q-.125 0-.262-.075Q18.6 8.15 18.55 8l-.8-1.75-1.75-.8q-.15-.05-.225-.188Q15.7 5.125 15.7 5t.075-.263Q15.85 4.6 16 4.55l1.75-.8.8-1.75q.05-.15.188-.225.137-.075.262-.075t.263.075q.137.075.187.225l.8 1.75 1.75.8q.15.05.225.187.075.138.075.263t-.075.262Q22.15 5.4 22 5.45l-1.75.8-.8 1.75q-.05.15-.187.225-.138.075-.263.075Zm0 14q-.125 0-.262-.075-.138-.075-.188-.225l-.8-1.75-1.75-.8q-.15-.05-.225-.188-.075-.137-.075-.262t.075-.262q.075-.138.225-.188l1.75-.8.8-1.75q.05-.15.188-.225.137-.075.262-.075t.263.075q.137.075.187.225l.8 1.75 1.75.8q.15.05.225.188.075.137.075.262t-.075.262q-.075.138-.225.188l-1.75.8-.8 1.75q-.05.15-.187.225-.138.075-.263.075ZM9 18.575q-.275 0-.525-.15T8.1 18l-1.6-3.5L3 12.9q-.275-.125-.425-.375-.15-.25-.15-.525t.15-.525q.15-.25.425-.375l3.5-1.6L8.1 6q.125-.275.375-.425.25-.15.525-.15t.525.15q.25.15.375.425l1.6 3.5 3.5 1.6q.275.125.425.375.15.25.15.525t-.15.525q-.15.25-.425.375l-3.5 1.6L9.9 18q-.125.275-.375.425-.25.15-.525.15Z"/>
```

**Close / x** — `close` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="M480-424 284-228q-11 11-28 11t-28-11q-11-11-11-28t11-28l196-196-196-196q-11-11-11-28t11-28q11-11 28-11t28 11l196 196 196-196q11-11 28-11t28 11q11 11 11 28t-11 28L536-480l196 196q11 11 11 28t-11 28q-11 11-28 11t-28-11L480-424Z"/>
```

**Check** — `check` — fill, `viewBox="0 -960 960 960"`

```svg
<path d="m382-354 339-339q12-12 28-12t28 12q12 12 12 28.5T777-636L410-268q-12 12-28 12t-28-12L182-440q-12-12-11.5-28.5T183-497q12-12 28.5-12t28.5 12l142 143Z"/>
```

### 2.5 Usage notes for the toolbar

- Render filled glyphs at 70-80 % foreground opacity as the default state and full accent color on an accent-tinted 28-32 px button for the active tool. Filled glyphs already carry the weight; no extra stroke is needed.
- At 16-20 px, the filled rectangle/diamond/circle set stays distinguishable by silhouette. If the rectangle and minus glyphs ever sit side by side at 16 px, use `square` instead of `rectangle` for the rectangle tool.
- `draw` is a pen nib and `edit` is a pencil. Pick one and keep it for the draw tool; do not use `edit` for "rename" as well.
- `line_start` is the line-tool mark; `remove` is the minus mark. Do not map both to the same path.
- `arrow_selector_tool` is the select tool. `pan_tool` is the hand tool. They are visually distinct at 18 px, which is the main thing Lucide failed at.

## 3. Runner-up sets

| Set | License | Tradeoff |
|---|---|---|
| **Phosphor Icons** (<https://phosphoricons.com>, <https://github.com/phosphor-icons/core>) | MIT | Best alternative. 1,248 icons, 6 real weights (Thin/Light/Regular/Bold/Fill/Duotone), more character than Lucide, and a genuine Bold weight for strength. Paths are 256-unit (`viewBox="0 0 256 256"`), so the same viewBox/scale work is needed as with Material. Canvas-object coverage is slightly thinner than Material's (no dedicated sticky-note-with-fold, eraser variants are fewer). Good fallback if the team rejects the Material look. |
| **Tabler Icons** (<https://tabler.io/icons>, <https://github.com/tabler/tabler-icons>) | MIT | Easiest integration: literal 24-unit paths, 2 px stroke, outline plus a growing `-filled` set, very large coverage. But it is Feather-derived, like Lucide — same 2 px outline feel and the same weak silhouettes. Choosing Tabler would keep the current look while adding icons, not fix the stated problem. |
| **Remix Icon** (<https://remixicon.com>, <https://github.com/Remix-Design/RemixIcon>) | **Remix Icon License v1.0 (since January 2026)** | Visually a strong 24-unit line/fill system with 3,200+ icons and canvas-adjacent glyphs. It was Apache-2.0, but the January 2026 license adds restrictions (no standalone icon-pack resale, no competing icon library, logo use prohibited) and is no longer MIT/Apache-2.0/ISC/CC0. Excluded under the permissive-only requirement; if legal approves the new license it becomes a credible contender. |

Also screened:

- **Lucide** (ISC) — current set; uniform 2 px outlines are the problem being solved.
- **Heroicons** (MIT) — solid/outline 24-unit set, but lacks whiteboard primitives (no diamond, no plain ellipse, no eraser, no sticky note).
- **Bootstrap Icons** (MIT) — designed on a 16-unit grid; shapes get soft when scaled to 20-24 px and coverage of canvas objects is thin.
- **Iconoir** (MIT) — 24-unit, 1.5 px stroke, lighter than Lucide; wrong direction.
- **Material Symbols Outlined/Rounded (unfilled)** — same family as the recommendation but outline-first; use as the quiet state only, not as the default toolbar weight.

## 4. License vendoring

Vendor the exact upstream license file, unmodified, at:

```text
assets/icons/LICENSE-material-symbols.txt
```

Source of that exact file:

```text
https://raw.githubusercontent.com/google/material-design-icons/master/LICENSE
```

It is the unmodified Apache License, Version 2.0 text. The file carries no project-specific copyright line (the copyright line in its appendix is the template placeholder). The repository contains no `NOTICE` file, so no additional NOTICE text is required. Add the copyright line in the suggested notice below when recording the third-party entry.

Suggested third-party notices entry:

```text
Material Symbols
Copyright 2014 Google LLC
Licensed under the Apache License, Version 2.0.
https://github.com/google/material-design-icons
```

Apache-2.0 obligations that apply here: retain the license and any copyright notices with the redistributed assets, ship a copy of the license with the application distribution, and do not use Google trademarks to imply endorsement. Attribution in the UI is optional but recommended in the About screen.
