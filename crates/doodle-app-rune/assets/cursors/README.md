# Board cursors

Rendered from the app's own Material Symbols Rounded glyphs (Apache-2.0, licence in
`../icons/LICENSE-material-symbols.txt`), so the cursor and its toolbar button stay the same
shape. Each is a 64x64 RGBA PNG at 2x, shown as a 32x32 pt cursor: dark ink over a 2px white
halo, readable on both the dark board and a pale note.

| File | Glyph | Hot spot (pt) |
| --- | --- | --- |
| `pen.png` | `pen` | `[7.5, 24.5]`, the tip |
| `sticky-note.png` | `sticky` | `[6.3, 6.3]`, the note's top-left corner |

The PNGs are the source of truth. Regenerate them from the glyphs with:

```sh
cargo test -p doodle-app-rune regenerate_cursor_pngs -- --ignored
```
