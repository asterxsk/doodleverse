# Board cursors

Original assets drawn for Doodleverse. Each is a 64x64 RGBA PNG with a transparent
background, drawn at 2x so Rune shows it as a 32x32 pt cursor. Shapes are dark ink
(`#1c1a20`) over a white halo, so they read on both light and dark boards.

| File | Shape | Hot spot (pt) |
| --- | --- | --- |
| `pen.png` | Fountain-pen nib pointing down-left, barrel up-right | `[4.5, 27.5]`, the nib tip |
| `sticky-note.png` | Square note with a folded bottom-right corner and two ruled lines | `[3, 3]`, the note's top-left corner |

Regenerate with the script kept in `scratch/gen_cursors.py` (untracked); the PNGs are the
source of truth.
