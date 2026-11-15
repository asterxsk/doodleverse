# Doodleverse — decisions made while the user was away

Autopilot run: 2026-11-15. Request: implement `IMPLEMENTATION_PLAN.md` end to end, using the
laptop display as a scratch pad for testing. User unavailable for questions; every decision made
without them is recorded here.

## Decisions

### D01 — eframe 0.37 instead of a hand-rolled winit + egui-wgpu shell
- **Because:** the plan asks for a native Rust whiteboard with a GPU surface. eframe 0.37 on wgpu
  already owns the winit event loop, wgpu device, surface, and egui renderer; a custom shell would
  re-implement that plumbing and add a large failure surface for no product difference.
- **Alternatives:** winit + egui-wgpu + egui-winit directly (plan's literal wording); a pure wgpu
  renderer with egui overlays on top.
- **Reversibility:** moderate — app shell is one crate; canvas/render/document crates do not depend
  on eframe.
- **Risk if wrong:** eframe 0.37 is newer than the plan assumed and its API has drifted
  (`App::ui`, `Panel::top/bottom`, `Context::global_style`); worked around and verified by build.
  This supersedes plan decision D03.

### D02 — Local undo as command deltas, not CRDT undo
- **Because:** yrs' undo manager would capture remote edits too, and this build has no peers yet.
  Capturing before/after `ObjectDelta` per local command gives exact, predictable undo and redo,
  and remote-origin transactions are never recorded.
- **Alternatives:** `yrs::UndoManager` (would need remote-scope plumbing); snapshot-per-command
  (too much memory at 100k objects).
- **Reversibility:** cheap — history sits behind `BoardDocument`.
- **Risk if wrong:** when networking lands, remote changes may interleave with a pending redo
  stack; redo is cleared on any local command but not on remote updates. Revisit before P02/P07.

### D03 — Portable file carries a validated board snapshot, not a CRDT update blob
- **Because:** the requirement is a portable, self-contained `.doodleverse` file that any copy of
  the app can open; visible geometry (strokes, statements, groups) is the durable content. Encoding
  a yrs update couples the file to a CRDT implementation version and can smuggle deleted history.
- **Alternatives:** encode `encode_full_update_v1` bytes inside the envelope; hybrid snapshot +
  update.
- **Reversibility:** moderate — envelope version and schema are explicit; a new payload type can
  be added without breaking v1 readers.
- **Risk if wrong:** future collaborative history/undo metadata cannot round-trip through a file.
  The file is explicitly "board content", not "session state", so this matches the brief.

### D04 — Import always remaps IDs and normalizes placement
- **Because:** two boards must be able to coexist without identity collisions; imported content is
  untrusted and must not be able to seize an existing ID. Import validates the whole payload,
  rejects duplicate IDs, remaps all IDs (including group parents), and translates content to the
  origin or to a user-chosen ghost placement.
- **Alternatives:** preserve IDs when no collision exists (still unsafe across merges); reject
  imports on any collision (bad UX).
- **Reversibility:** cheap.
- **Risk if wrong:** imported groups cannot be diffed against their source board by ID. Acceptable;
  the contract only promises content.

### D05 — Empty board imports land immediately; non-empty boards use click-to-place ghost
- **Because:** first import should not demand a placement decision; later imports must not silently
  cover existing work. Ghost shows occupancy validity and refuses overlapping drops.
- **Alternatives:** always ghost; always center; always paste at cursor with overlap allowed.
- **Reversibility:** cheap.
- **Risk if wrong:** overlap tolerance is an epsilon, so a deliberately touching layout may need a
  nudge. Noticed in UI text.

### D06 — AI ships as an offline mock provider only
- **Because:** BYOK, ChatGPT OAuth, and ACP all require accounts, credentials, or containment work
  that autopilot must not perform. The mock exercises the real card lifecycle (submit, stream,
  cancel, alternatives, failure, pin) with zero network and zero stored secrets.
- **Alternatives:** implement provider interfaces without keys (dead code); leave AI cards out
  (loses the reviewed interaction).
- **Reversibility:** cheap — `AiProvider` is the seam; a real provider is another impl.
- **Risk if wrong:** users may expect real answers from "Dibby demo (offline)". The card labels the
  provider and the help window states the limitation.

### D07 — Pinning a completed answer creates a board statement with the exact answer text
- **Because:** the product requires explicit sharing of AI output to the board and nothing else.
  The pin modal previews exactly what will be written; the question, alternatives, and provider
  metadata stay session-local.
- **Alternatives:** pin all alternatives; pin a summary; keep AI text outside the board document.
- **Reversibility:** cheap.
- **Risk if wrong:** a statement is plain text, so the answer loses markdown structure. The plan's
  statement object is plain text; richer formatting is a later object type.

### D08 — One process, one board, RAM only
- **Because:** the product is explicitly stateless: no autosave, no recent-files database, no
  recovery cache. Closing without export loses the session, and the close confirmation says so.
- **Alternatives:** autosave to a temp file (directly contradicts the product); session restore.
- **Reversibility:** cheap but a product decision the user should confirm.
- **Risk if wrong:** accidental close loses work. Mitigated by close confirmation with
  Export-then-close, and a persistent unsaved marker.

### D09 — Settings are session-only
- **Because:** persisting theme/grid/brush would create a settings store, a file format, and a
  migration surface not requested by the plan. Defaults are good; the settings panel says
  "session-only".
- **Alternatives:** write a config file next to the executable or in the OS config dir.
- **Reversibility:** cheap.
- **Risk if wrong:** users re-pick their theme each launch. Recorded for later.

### D10 — Native file dialogs via rfd, atomic sibling temp + rename
- **Because:** no in-app file browser to build, and an interrupted write must not truncate the
  previous export. `write_atomic` writes a sibling temp file, fsyncs, then renames.
- **Alternatives:** drag-and-drop only; a custom file picker.
- **Reversibility:** cheap.
- **Risk if wrong:** Windows rename over an existing file relies on the platform; the files crate
  test covers the happy path, not a crash mid-write (cannot be simulated portably).

### D11 — Direct-only WebRTC is deferred; no room UI is shown
- **Because:** signalling, ICE, and peer transports need a server or manual signalling exchange and
  real multi-machine testing; none can be added or verified unattended. The types and CRDT update
  encode/apply APIs are in place so the transport can land behind them.
- **Alternatives:** a local two-window loopback test (proves nothing about WebRTC); a fake rooms UI
  (dishonest).
- **Reversibility:** cheap — no UI promises rooms today.
- **Risk if wrong:** the core product promise (shared doodling) is missing from this build. Stated
  honestly in the status bar and help.

### D12 — No TURN, no relay, no board bytes through a server
- **Because:** product requirement. Enforced by omission: no networking code exists in this build,
  so there is nothing to accidentally route.
- **Reversibility:** must remain a constraint when networking lands.
- **Risk if wrong:** direct-only connections can fail behind symmetric NAT; that is the accepted
  product trade-off.

### D13 — Windows are placed on the laptop display only through `DOODLE_TEST_WINDOW`
- **Because:** the user offered the secondary display as a scratch pad. Hard-coding monitor
  placement would be wrong for real users, so the test harness uses an env var parsed by
  `doodle_platform::test_window_placement`; normal launches use remembered/default placement.
- **Alternatives:** always start on the primary display (would cover the user's work); a CLI flag
  (env var is enough for tests).
- **Reversibility:** cheap.
- **Risk if wrong:** a user setting that env var gets an unusual position; it is documented as a
  test hook.

### D14 — Mascot frames come from the approved atlas manifest, never regenerated
- **Because:** assets are approved input. The app parses the schema-2 manifest, derives UVs, and
  animates by elapsed time; it never invents frames.
- **Alternatives:** per-state PNG files (more IO, no atlas benefit); CSS/HTML preview only.
- **Reversibility:** cheap.
- **Risk if wrong:** manifest and atlas can drift; `parse_manifest` validates schema, dimensions,
  and frame ranges, and the atlas is embedded at compile time.

### D15 — Reduced motion wins over every mascot state
- **Because:** accessibility priority confirmed in the assets contract; it also stops repaint
  scheduling while idle.
- **Alternatives:** reduced motion only disables hover/busy but keeps talking.
- **Reversibility:** cheap.
- **Risk if wrong:** card looks static while generating. Status text still changes.

### D16 — No credentials, vault, hardware pen pressure, packaging, or release automation
- **Because:** each needs installing software, OS keychain integration, signing identities, or
  hardware unavailable here. Mouse pressure is simulated from speed and stroke progress; the
  settings panel states hardware pressure is a later milestone.
- **Alternatives:** a plaintext config for keys (violates the OS-vault-only contract).
- **Reversibility:** all are additive behind existing seams.
- **Risk if wrong:** pen users get simulated pressure until P04a lands.

### D17 — Keep the workspace license at "MIT OR Apache-2.0"
- **Because:** the plan's workspace manifest assumes it; changing licensing is the user's call.
- **Alternatives:** none taken.
- **Reversibility:** cheap but legally meaningful; recorded as an assumption.
- **Risk if wrong:** user wanted a different license; single manifest edit plus file headers.

### D18 — Commit in phases; push checkpoints when explicitly authorized
- **Because:** autopilot may commit locally; pushing is outward-facing and needs explicit user
  authorization. The user later authorized checkpoints to `github.com/asterxsk/doodleverse`, so
  pushes are allowed for this run. History stays split so a failed later phase does not bury
  working earlier phases.
- **Alternatives:** one giant commit (harder to audit); no commits (loses the record).
- **Reversibility:** cheap (local history).
- **Risk if wrong:** none material; pushed content is public, so each checkpoint is verified first.

### D19 — Floating Excalidraw-style toolbox, zen mode on Alt+Z
- **Because:** the user asked for Excalidraw's floating toolbox and its keybind. Excalidraw keeps
  tools in floating "island" panels over the canvas and hides the UI with Alt+Z zen mode (verified
  in Excalidraw's `actionToggleZenMode`: Alt+Z, no Ctrl/Cmd).
- **Alternatives:** keep the fixed top toolbar; use `Alt+H` (a plugin convention, not core
  Excalidraw); hide the UI with a menu-only action.
- **Reversibility:** cheap — the islands are drawn by one app module and can become panels again.
- **Risk if wrong:** on QWERTZ layouts Excalidraw users sometimes report Alt+Y; this build follows
  the logical Z key, so keyboard layout mapping is left to the OS.

### D20 — Miro-like board theming
- **Because:** the user asked for any board color and adjustable dot size. The existing grid
  settings gained board-color presets (Paper, Snow, Sand, Mist, Slate, Ink), a free color picker,
  and dot/plus size plus line width controls, all session-local.
- **Alternatives:** fixed light/dark themes only; separate "canvas" and "grid" color systems.
- **Reversibility:** cheap.
- **Risk if wrong:** theme is not persisted across launches (see D09).

### D21 — Hardware pen pressure through the Windows pointer stack
- **Because:** the PRD already requires real stylus pressure (5.5); egui 0.37 exposes no pressure
  field, so the app subclasses its window procedure and reads `WM_POINTER` pen pressure via
  `GetPointerPenInfo`. Samples feed `RawSample.pressure`; mouse input keeps the simulated envelope.
- **Alternatives:** winit tablet events (not surfaced by eframe 0.37); Wintab (legacy, extra DLL);
  leaving pressure simulated (contradicts the PRD).
- **Reversibility:** cheap — the probe sits behind `doodle_platform::pen_pressure()`.
- **Risk if wrong:** non-Windows platforms fall back to simulated pressure until native pointer
  probes land; the API returns `None` there, which is the same path as using a mouse. The subclass
  only reads pointer messages and forwards every message unchanged.

## Assumptions

- **A01 — Rust 1.99.1 toolchain is acceptable.** `rust-toolchain.toml` pins it, matching the
  plan's rust-version 1.95 floor. Confirm with the user's CI image; adjust the channel if needed.
- **A02 — Only the visible board is exported.** Hidden parents/deleted CRDT state are not part of
  the file. Product language ("export the board") supports this.
- **A03 — 8 peers, 100k objects, 64 MiB snapshot limits are the intended product limits.** Taken
  verbatim from the plan; all enforcement points cite `doodle_types::limits`.
- **A04 — The user's laptop display is `DISPLAY1` at negative X when the primary is on the right.**
  Verified by a native-window screenshot during this run; the env hook is generic, not
  DISPLAY1-specific.
- **A05 — Statement text is plain text, one style.** Pinned answers and future text objects share
  `StatementObject`; rich text was not in the plan's object model.

## Blockers (not attempted, by autopilot rule)

- **B01 — Peer-to-peer mesh (P02/P07/P10/P12).** Needs signalling/ICE design, likely a signalling
  server or manual SDP exchange, and multi-machine testing. Resume with a transport crate behind
  `BoardDocument::encode_full_update_v1` / `apply_remote_update_v1`, then WebRTC data channels
  (direct only, no TURN), room UI, and peer count enforcement. No network calls exist today.
- **B02 — BYOK AI provider (P13).** Needs the user's provider choice, API key handling policy, and
  OS vault integration. Resume by implementing `AiProvider` against the chosen API with keys read
  only from the OS vault; never write keys to disk, logs, or board files.
- **B03 — ChatGPT OAuth (P14).** Needs a registered OAuth client and user consent flow; autopilot
  must not create accounts or use credentials. Resume with the device-code or PKCE flow approved by
  the user.
- **B04 — ACP agent containment (P15).** Needs sandbox/containment decisions and installed agent
  binaries. Resume only after the user chooses the containment boundary; the current AI boundary
  deliberately carries one question and no board/path/tool content.
- **B05 — OS vault + non-Windows pen pressure (P04a).** Windows pen pressure is implemented (D21)
  but cannot be verified without a tablet. Other platforms still use simulated pressure. The OS
  vault needs platform keychain APIs and a chosen credential policy. Resume with vault adapters
  behind a credential-store trait and native pointer probes per platform.
- **B06 — Packaging, signing, release (P17/P20).** Needs signing identities and store accounts.
  Resume with `cargo-bundle`/MSIX/notarization as chosen by the user; the window icon is already
  embedded from the approved PNG.
- **B07 — PR / release publication.** Checkpoints are pushed to `main` on
  `https://github.com/asterxsk/doodleverse` under explicit authorization. No pull request or
  release was created. Resume with a PR/branch review or a tagged release if desired.
