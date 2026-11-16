# Icon assets

## Lucide

`lucide/` holds the Lucide outline icons used by the Rune shell, copied unmodified from
[lucide.dev](https://lucide.dev) on 2026-11-16. Each SVG is 24×24 with a 2-unit stroke, round
caps and joins, and `stroke="currentColor"`; `crates/doodle-app-rune/src/icons.rs` embeds the
path data and recolours it per use.

The download did not record a Lucide release number; the files are the set published on that
date. Licence: **ISC** — see [`LICENSE-lucide.txt`](LICENSE-lucide.txt), copied alongside the
icons. That file also carries Lucide's MIT notice for the icons derived from Feather.

Used icons: `arrow-right`, `book-open`, `circle`, `circle-question-mark`, `diamond`,
`ellipsis-vertical`, `eraser`, `hand`, `lock`, `map`, `minus`, `mouse-pointer-2`, `pencil`,
`redo-2`, `square`, `sticky-note`, `type`, `undo-2`, `x`.

`circle`, `circle-question-mark`, `ellipsis-vertical`, `lock` and `square` are built from
`<circle>`/`<rect>` elements in the source SVGs; `icons.rs` stores path equivalents with the
same geometry (`arc`/`round` shapes) beside the icon's own `<path>` data.
