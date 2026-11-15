# Dibby mascot assets and animation handoff

Dibby is Doodleverse's user-selected app icon and AI buddy. The chosen cream paper character, folded corner, friendly face, and coral/teal orbit define his identity. The name **Dibby** is confirmed. Keep “Ask Dibby” consistent across provider choices.

## Deliverables

| File | Purpose |
|---|---|
| `assets/app-icon.png` | The original selected app-icon PNG, preserved without changing its pixels |
| `assets/mascot/dibby.svg` | Editable transparent vector recreation with separate character parts for animation |
| `assets/mascot/dibby-idle.svg`, `dibby-hover.svg`, `dibby-talking.svg`, `dibby-thinking.svg`, `dibby-working.svg` | Self-contained browser-animated versions of the five requested states |
| `assets/mascot/dibby-busy.svg` | Self-contained browser-animated thinking/working cycle, switching every three seconds |
| `assets/mascot/dibby-motion.json` | Authoritative motion/state and native atlas metadata |
| `assets/mascot/dibby-atlas.png` | Transparent raster animation frames for the native application |
| `docs/dibby-preview.html` | Local interactive preview of the mascot states and busy cycle |
| `tools/dibby-rig.js` | Deterministic vector-part motion sampling used by the preview/build pipeline |
| `tools/build_dibby_assets.py` | Asset generation pipeline using headless Playwright to rasterize the vector artwork |

Unused concept images and their exploratory notes are removed from `assets`. The SVG is a hand-authored vector recreation of the selected mascot, not a pixel-identical reproduction of the generated paper/brush texture. Keep the original PNG for the app icon; use the SVG/atlas for the animated UI mascot.

Platform-specific ICO/ICNS sizes and launcher masks remain packaging work. Do not treat a square PNG or vector recreation as evidence that those exports have been prepared or validated.

## Animation states

There are five animated states plus one neutral static state. Motion manifest **schema version 2** distinguishes the animated `idle` from the static `neutral` pose:

| State | Use |
|---|---|
| `neutral` | Static Dibby for reduced motion or a deliberately motionless presentation |
| `idle` | The original gentle floating motion, looping while visible without a higher-priority state |
| `hover` | A small one-shot jump on pointer entry or equivalent keyboard focus, lasting 600 ms |
| `talking` | Dibby's conversational response while presenting streamed answer text; it does not imply generated speech |
| `thinking` | First visual phase of an active busy/request cycle, with a visible thought cloud |
| `working` | Second visual phase of that same busy/request cycle; laptop sits on Dibby's right, angled inward so its screen faces him and its keyboard extends toward his hands |

The user explicitly requested that **thinking and working alternate about every three seconds**. Implement this as presentation timing, not model reasoning detection:

- A new active request starts with thinking.
- At 3,000 ms, show working.
- At 6,000 ms, show thinking again; continue every 3,000 ms until that request ends.
- Do not invoke another provider request or agent tool when the visual phase changes.
- Do not imply that “working” means a terminal/file operation is happening. The minimal harness remains text-only.
- The thought cloud and laptop are expressive vector props. They do not show model reasoning, actual files, tools, or permission state.
- Use truthful surrounding status text such as “Generating…” throughout both visual phases; show actual provider errors, cancellation, or completion explicitly.

Use a monotonic clock and an explicit busy-start timestamp. The requested three-second interval selects the state; animation loops within each state follow the durations in `dibby-motion.json`. Do not confuse those two clocks.

## State priority and lifecycle

When several inputs are active, select the presentation in this order:

1. Reduced motion: show the neutral/static frame; keep actual status labels visible.
2. Talking: the answer/interaction currently requires the talking presentation.
3. Hover jump: the local user has just entered/focused the mascot's actionable target, and fewer than 600 ms have elapsed since that transition.
4. Busy: choose thinking/working using elapsed time since the active request began.
5. Idle.

Hover/talking temporarily override the visible busy phase without restarting the busy clock. The jump plays once, then returns to the underlying busy state or idle even if the pointer remains over Dibby. Leaving and entering again, or a new focus-entry transition, retriggers the jump. Do not restart it every render frame. Capture the transition timestamp and pass the elapsed duration to the selector: the rig resolver accepts `hovered` and `hoverElapsedMs` (default `0`); a continually true flag without advancing elapsed time would incorrectly keep restarting the hover presentation.

When an override ends, return to whichever busy phase the request has reached. Completing, failing, canceling, deleting the card, switching providers, or leaving the session ends the relevant busy/talking state. A later new request begins a fresh thinking phase.

Animate only while visible and needed. Animated idle needs scheduled repaints while visible; neutral, hidden/minimized/offscreen cards, and reduced-motion mode do not. Do not let animation completion callbacks mutate board content or resurrect a deleted card. UI state remains local and session-only.

## Native Rust integration

The SVG is an editable vector source. Do **not** assume that a Rust SVG loader runs CSS, SMIL, DOM scripts, or browser animation APIs. Most application SVG rasterization paths need explicit animation scheduling. Choose one of these approaches:

### Recommended first integration: frame atlas

1. Load the bundled `dibby-atlas.png` once as an RGBA texture. Load `dibby-motion.json` as the source of truth for atlas dimensions, frame indices, frame counts, and timing; do not duplicate guessed layout constants in the app.
2. Select the state using the lifecycle rules above. Determine its frame from elapsed state-animation time, looping only states marked as looped by the manifest. Use the neutral frame for reduced motion; idle is now animated. Hover is a one-shot animation and must not be modulo-looped.
3. Draw the specified texture region at a stable size in the AI-card corner. Keep aspect ratio, enough visual padding, and linear filtering appropriate to the UI scale. Do not create one texture per frame or reload the PNG on every tick.
4. Request repaint only when the visible frame changes or another UI action needs it. Cancel animation scheduling when the card disappears.
5. At small/high-DPI sizes, inspect the actual rendered character before claiming visual acceptance. If a future UI requires larger close-up animation, regenerate a suitably sized atlas from the vector source rather than enlarging tiny frames indefinitely.

This does not require a webview or a new runtime animation library. The native agent can use the existing UI texture/image API. Treat manifest schema/version validation and out-of-range frame rejection like any other bundled data contract.

The schema-version-2 atlas uses **160 × 160 px frames**, **12 columns × 16 rows**, a **1920 × 2560 px image**, and **181 occupied frames**. Frames are packed row-major across the whole atlas, not one row per state. The remaining slots are unused. The atlas is straight-alpha RGBA (`premultiplied_alpha: false`); use the native texture loader's appropriate alpha path rather than assuming premultiplied input.

| State | First frame | Frame count | Duration | Loop |
|---|---:|---:|---:|---|
| `neutral` | 0 | 1 | 0 ms | No; static |
| `idle` | 1 | 36 | 2,400 ms | Yes |
| `hover` | 37 | 36 | 600 ms | No; one-shot jump |
| `talking` | 73 | 36 | 1,200 ms | Yes |
| `thinking` | 109 | 36 | 3,000 ms | Yes |
| `working` | 145 | 36 | 3,000 ms | Yes |

Read these values from `atlas` and `states` in the manifest. For a looping state, choose local frame `floor((elapsed_ms mod duration_ms) / duration_ms × frame_count)`, clamped below `frame_count`, then add `first_frame`. For the nonlooping hover state, clamp elapsed time to its duration and clamp the frame to the final local index; never modulo-loop it. The controller returns to the underlying state after 600 ms. For neutral, use frame 0 without division. Given global frame index `i`, column is `i mod columns` and row is `floor(i / columns)`; obtain the pixel rectangle from those coordinates and `frame_width`/`frame_height`, then normalize by atlas `width`/`height` for UVs. Validate index bounds before drawing.

The manifest's `controller.busy.sequence` selects thinking for 3,000 ms and working for 3,000 ms, looping independently of overlays. Do not add a second “busy” atlas row: busy is a controller selecting two existing states. Use the original busy elapsed time to retain phase after talking/hover overrides.

### Alternative: native vector rig

Use the SVG's separate named parts and the motion data as an authoring reference. A native renderer can transform/tessellate these layers with its existing graphics path, retaining back-orbit/body/face/front-orbit painter order. The static master also includes initially hidden thought-cloud and laptop parts; reveal them only for their corresponding state. Keep pivots and motion values in SVG coordinates before applying the final UI scale. Do not rotate a front-only ring independently in a way that breaks its occlusion against the body.

The working laptop uses an inward-facing three-quarter perspective on Dibby's right, with keyboard and hands toward his body. Preserve that orientation rather than replacing it with a front-facing screen across his torso. Its local pivot is `[1050, 990]`, recorded in `rig.part_pivots.laptop`; the main character pivot remains `[630, 640]`. The working gaze is slightly down/right toward the laptop.

The master `dibby.svg` is static. The state-specific and busy SVG exports include browser CSS animation for standalone visual use, but that animation is not assumed supported by a Rust rasterizer. The preview demonstrates visual behavior; the implementing agent still wires state selection, drawing, repaint scheduling, and provider cancellation into the Rust app. The generation tools are development tooling, not dependencies that the shipped Rust app must execute.

## Placement and accessibility

Use a small Dibby avatar in the AI card's corner/header, then the question, then the answer. Regenerate/alternatives/cancel/pin controls remain secondary. The question and unpinned replies stay local; pinning shares only the selected answer through the existing document command. A decorative mascot mark does not grant the card a provider connection or trigger inference.

Label an actionable mascot/entry point “Ask Dibby” or its actual action, expose keyboard focus, and keep status/error text separate from the animation. Decorative repetitions can be hidden from the accessibility tree where the surrounding control already has a useful label. Never rely on movement alone to explain busy, talking, failure, or completion.

Reduced motion shows neutral Dibby at frame 0 without bobbing, mouth cycling, orbit motion, bouncing, or animated cloud/laptop props. Keep real busy/provider status in separate explicit text rather than adding another reduced-motion pose. Hover still gets a visible non-motion focus/interaction treatment from the UI. Do not persist a new mascot preference or add an always-floating assistant as part of this asset integration.

## Verification boundaries

This is an asset handoff, not a native app implementation. Validate the SVG/PNG/manifest together, inspect the preview at normal and small sizes, and exercise all states, override priority, three-second busy switching, and reduced motion. Record actual verification results separately; this document does not claim that native window rendering, screen-reader integration, GPU packaging, or provider behavior has been tested.

## Rebuilding and verified checks

Development tooling used for this handoff: Python 3.14.10, Pillow 12.3.0, Playwright 1.63.0, and the installed Google Chrome channel (`chrome`). These are build/verification dependencies, not requirements for the shipped Rust app. The build script does not download dependencies or a browser. With those prerequisites available, run from the repository root:

```powershell
python tools/build_dibby_assets.py
python tools/check_dibby_assets.py
```

The build regenerates the atlas, animation SVGs, motion manifest, and self-contained HTML preview from `assets/mascot/dibby.svg` and `tools/dibby-rig.js`. It never modifies `assets/app-icon.png`. The CSS animation exports and deterministic atlas sampler are separate delivery formats of the same poses; the native atlas/manifest is the authoritative first integration path.

The schema-version-2 assets were rebuilt and checked on 2026-11-15. `python tools/check_dibby_assets.py` passed against the new outputs:

- All seven SVGs parse as vector artwork, with unique IDs and no embedded raster images or scripts.
- All 181 occupied atlas frames are indexed exactly once, retain transparency, and fit within their cells. Animated states contain changing frames.
- Idle uses the former gentle floating motion. Hover rises and lands in 600 ms, stops while hover/focus remains active, and retriggers on a new entry. Both deterministic sampling and live preview focus/re-entry were exercised.
- Thinking shows only its thought cloud; working shows only its laptop. SVG-export and rig prop visibility checks passed. After the user-requested laptop perspective correction, the SVGs/atlas were rebuilt, the complete checker passed again, and rendered inspection confirmed the right-side, inward-facing laptop.
- Busy selection and the standalone busy SVG switch at the 3,000/6,000 ms boundaries. Talking/hover priority, completion back to the underlying state, and reduced-motion neutral behavior passed.
- Preview controls, unique inline SVG IDs, and the layout at 390 px width passed. Rendered inspection confirmed the updated character and props.
- The original app icon is still byte-identical, with SHA-256 `0f10fc0e471899454346759399719374ad3c3e4728fd438e602142212dd50afb`.

The new atlas and manifest must be shipped together; do not reuse schema-version-1 frame offsets. Native Rust app integration, live provider state wiring, native accessibility, and platform icon packaging remain the implementation agent's work.
