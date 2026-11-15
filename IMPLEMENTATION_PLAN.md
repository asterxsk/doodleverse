# Doodleverse implementation plan

**Audience:** the implementation orchestrator and its delegated agents.  
**Prepared:** 2026-11-15.  
**Repository:** https://github.com/asterxsk/doodleverse.git  
**Status:** implementation plan and user-approved mascot asset handoff; native application implementation, repository initialization, commits, and deployment have not been performed.

## 1. Mission and authority

Build a native Rust desktop application for Windows, macOS, and Linux: an infinite, pressure-sensitive whiteboard that supports small peer-to-peer rooms and local AI brainstorming cards. Board content lives in RAM and moves between sessions only through explicit portable-file exports/imports. Ship a usable local drawing app before adding distributed behavior, then integrate collaboration and AI without compromising that local foundation.

The supplied **Doodleverse Product Requirements Document.md** is the product authority. At implementation bootstrap, preserve its exact contents as `docs/PRODUCT_REQUIREMENTS.md`; do not replace it with this plan. Preserve this plan, the decision log, contracts, and verification records in the repository so a replacement orchestrator can continue without chat history.

**Subsequent confirmed product direction:** the user selected the first generated concept, `assets/app-icon.png`, as both the app icon and the app's mascot. The mascot is the visual identity of the AI buddy, especially on canvas AI cards. His confirmed name is **Dibby**. Section 16 records the required card layout and proposed contextual appearances; these additions supplement the original PRD without changing its provider, privacy, or pinning behavior.

**Final mascot approval:** the user approved the delivered design and animation bundle after the working laptop was moved to Dibby's right and angled toward him. Integrate the prepared assets and behavior in section 16; do not restart icon/mascot exploration or substitute a new character. Production platform packaging and native integration remain implementation work.

One product clarification was explicitly confirmed by the user during planning:

> Remember AI credentials in the OS credential vault; never save boards or drafts.

Treat that as a narrow exception to statelessness. Authentication registration identifiers needed to reuse an approved connection belong in the same vault record. It does not authorize remembered boards, AI questions/answers, recent files, background preferences, room tokens, or recovery autosaves.

The planning workspace was empty and had no `.git` directory. The remote was reachable but had no advertised refs; GitHub also displayed an empty repository. This is a greenfield implementation, not a migration. Recheck before bootstrap because the user or another agent may add content. [Repository inspected during planning](https://github.com/asterxsk/doodleverse).

### 1.1 Scope of this handoff

This document describes future application implementation work. It is not a claim that any application spike, API login, sandbox, native app build, device interaction, network topology, or performance target has passed. The separate icon/mascot asset work is complete and user-approved, with its actual asset checks recorded below. Do not start the application build, initialize Git, commit, push, or deploy a relay solely because these instructions exist. Application implementation begins when the user authorizes the implementation orchestrator to execute the plan.

### 1.2 Required v1 and deliberate limits

| Area | Required result | Limit |
|---|---|---|
| Native application | One Rust workspace; native executable/window; GPU canvas | No Electron, Tauri webview UI, browser frontend, web, or mobile target |
| Startup | Empty, usable infinite canvas immediately | No sign-in gate, dashboard, library, or session restoration |
| Collaboration | Direct WebRTC data channels; up to eight peers; cursors, names, colors, late join | No accounts, hosted board storage, voice, video, or chat |
| Files | Versioned portable board export/import through OS dialogs | Native Doodleverse format first; Miro/Excalidraw file compatibility is not specified |
| Import placement | One wrapper group; origin on empty board; non-overlapping ghost placement thereafter | Local collision check, not distributed locking |
| Appearance | Dots, pluses, lines, none; spacing, opacity, grid/background colors, light/dark presets | Per-user and session-only |
| Brush | Real pressure when available; velocity/progression simulation for mouse; smoothing/tapers/settings | Actual hardware support must be demonstrated per platform |
| AI cards | Local question and alternatives, streaming, explicit pin of selected answer | One question per card; no threaded agent conversation |
| Connections | ACP subprocess, official ChatGPT sign-in, three-field OpenAI-compatible BYOK | Local user owns credentials and initiates inference; no pooled subscriptions |
| Minimal harness | Text input/output for ChatGPT and BYOK | No tools, filesystem, shell, automatic board context, or peer-triggered inference |
| Optional exports | PNG/SVG if implemented after core milestones | Not a blocker for the required portable-board milestone |

**Necessary editor mechanics proposed by this plan:** single/marquee selection, move, delete, select all, local undo/redo, group/ungroup, pan/zoom, freehand, and AI cards. These make the stated features operable. Shapes, standalone text, sticky notes, connectors, images, resize/rotate handles, clipboard object exchange, rich text, and embedded media are not silently included by the Miro/Excalidraw comparison. Record any later additions as scope changes with their own tasks.

## 2. Decisions, constraints, and early feasibility gates

The orchestrator owns `docs/decisions.md`. For every material choice, record its ID, status (`confirmed`, `proposed`, `validated`, `blocked`, `superseded`), reason, evidence, affected tasks, and migration consequence. The following are the initial entries.

| ID | Decision / starting position | Status and rationale |
|---|---|---|
| D01 | Native Rust on Windows/macOS/Linux | PRD requirement and user-confirmed language |
| D02 | Boards/drafts/session state in memory; AI connection records in OS vault only | User-confirmed exception; isolate these data classes in types and tests |
| D03 | `winit` + `egui`/`egui-winit` + `egui-wgpu` + custom `wgpu` canvas | Proposed baseline; direct event-loop ownership makes tablet integration and shutdown explicit |
| D04 | `yrs` shared document; ephemeral awareness outside exported content | PRD requirement; schema and origin rules below |
| D05 | Direct-only WebRTC, no TURN | Follows the prohibition on board traffic through servers; some networks cannot connect |
| D06 | Manual bidirectional offer/answer exchange always available; convenient short-code signaling is an explicitly gated optional service path | A one-way offer alone cannot finish a connection; no imaginary public rendezvous service |
| D07 | Groups are logical membership; leaf geometry uses world-space poses | Proposed to avoid concurrent nested-transform ambiguity in v1 |
| D08 | Import means an independent copy with fresh IDs, not live application of the imported CRDT history | Required for repeat imports and wrapper grouping |
| D09 | Whole unpinned AI card is local; pin shares the selected answer only by default | Conservative interpretation of draft privacy; show exactly what will be shared |
| D10 | Latest stable ACP protocol mutually supported; do not opt into a draft solely to satisfy “newest” | Stable v1 is the planning baseline; reverify before implementation |
| D11 | Real ACP containment is a release gate, not a property of a temporary working directory | PRD promises containment; protocol permissions alone cannot enforce it |
| D12 | Use the first generated concept as both app icon and mascot; retain its cream paper character, folded corner, friendly face, and coral/teal brush orbit | Final design and delivered mascot bundle are user-approved; production icon sizes/masks still need validation |
| D13 | Mascot represents the AI buddy; AI cards place him in a corner, followed by the question and answer below | User-confirmed; his name is Dibby, and the implementation must use that name consistently |
| D14 | Dibby gently floats when idle, jumps once on hover/focus, talks, thinks with a thought cloud, and works with a laptop on his right angled toward him; busy alternates thinking/working every three seconds | Final delivered animations are user-approved; hover lasts 600 ms without repeating; reduced motion uses neutral frame 0, and props do not imply tools or extra inference |

### 2.1 Native UI and dependency selection

Use `egui` for application controls/text entry and a dedicated canvas renderer for board geometry. The official project supports separate `egui-winit` and `egui-wgpu` integration, and `egui-wgpu` offers custom GPU paint callbacks. Prefer this explicit integration over assuming an application framework exposes all tablet events. These APIs exist; their suitability for Doodleverse still needs a spike. [Official egui integrations](https://github.com/emilk/egui#integrations), [egui-winit](https://docs.rs/egui-winit/latest/egui_winit/), [GPU callback interface](https://docs.rs/egui-wgpu/latest/egui_wgpu/trait.CallbackTrait.html).

Alternatives considered: `eframe` reduces window/render boilerplate but must prove native pressure interception; Iced is a valid Rust alternative but would change the UI/rendering boundary and should only replace the baseline before shared contracts freeze if the spike establishes a concrete advantage. Do not let different workers independently choose different frameworks. These are planning recommendations, not choices already approved by the user.

At bootstrap, select the newest stable Rust toolchain and mutually compatible stable crate releases that actually pass the spikes. Pin the exact toolchain in `rust-toolchain.toml`; set the workspace edition, resolver, and `rust-version` coherently. Commit `Cargo.lock`. Never use wildcard dependency requirements or nightly/main-branch dependencies as an unnoticed shortcut. Newest independently selected versions are not necessarily a compatible graph. Cargo's lockfile records the resolved dependency set. [Cargo dependency resolution](https://doc.rust-lang.org/cargo/reference/resolver.html).

Candidate dependency families, subject to feasibility and license checks:

| Concern | Starting candidate | Validation needed |
|---|---|---|
| Window / controls / GPU | `winit`, matching `egui`, `egui-winit`, `egui-wgpu`, compatible `wgpu` | Surface lifecycle, accessibility, DPI, text input, tablet hooks |
| Shared document | `yrs` | Origins, update codec, state vectors, undo capture, schema limits |
| Transport | `webrtc` from webrtc-rs | Actual current API, runtime, native builds, direct channels, bounded transfer |
| Runtime / cancellation | `tokio`, `tokio-util` or equivalent proven compatible | Runtime ownership, task cancellation, process lifecycle |
| Files | `rfd` | OS-thread requirements, Linux portal dependencies, cancellation |
| HTTP / auth | `reqwest` with selected TLS backend, reviewed OAuth/OIDC/JWT libraries | No handwritten cryptography; streaming; callback and token validation |
| ACP | Official `agent-client-protocol` Rust SDK | Stable version negotiation, permissions, subprocess transport |
| Credentials | `keyring` or equivalent native-vault adapter | Windows/macOS/Linux backends and locked/unavailable vault behavior |
| Serialization / identity | `serde`, a bounded binary envelope codec, UUID/random IDs | Versioning, finite numbers, entropy, bounded decoding |
| Geometry / indexing | Small pure Rust geometry layer; `lyon`/`rstar` only where useful | Correct stroke outlines, live spatial bounds, measured benefit |
| Diagnostics / testing | `tracing`, `thiserror`, property tests, `egui_kittest` if compatible | Redaction, no automatic content logs; deterministic tests |

Inspect upstream release notes rather than copying older tutorials. The current webrtc-rs repository describes an async layer over a Sans-I/O core and warns about pre-1.0 breaking changes; its examples must match the version selected by the spike. [Official webrtc-rs repository](https://github.com/webrtc-rs/webrtc).

### 2.2 Gate G0: feasibility before broad feature delegation

Run three independent investigations. A spike is small and disposable; it must produce evidence and a go/no-go decision, not quietly become an unreviewed architecture.

| Spike | Prove | Deliverable and exit criterion |
|---|---|---|
| F0 Native canvas/input | Window, GPU draw, controls, mouse, live tablet samples and no duplicate mouse-generated strokes | Exact dependency set; Windows runtime evidence; identified and compiled macOS/Linux paths; per-platform hardware validation schedule |
| F1 Direct transport | Two native processes exchange data using WebRTC after an actual offer/answer exchange | Direct candidate proof, cancellation/shutdown, NAT limitation description, wire-size/chunk feasibility |
| F2 AI and containment | Current provider eligibility/protocols, official ChatGPT request, ACP initialization and real isolation strategy | Official-source notes, mock contracts, live tests only with authorized credentials; OS sandbox proof or explicit blocker |

**Important pressure risk:** the presence of touch force or touchpad-pressure events does not establish graphics-tablet pressure on all desktop platforms. Check the stable window library's actual event API and its platform behavior. A beta API or unreleased pull request is not stable support. Where needed, isolate Windows pointer/tablet APIs, macOS tablet events, and Linux Wayland/X11 tablet integration behind `doodle-platform`. Platform FFI is acceptable in that narrow module with reviewed safety invariants; do not replace the native Rust UI with a webview. [Stable winit event documentation](https://docs.rs/winit/latest/winit/event/enum.WindowEvent.html), [winit release status](https://github.com/rust-windowing/winit/releases).

A failed spike must not freeze all unrelated work. For example, local drawing, document semantics, and BYOK fixtures can proceed while a network or sandbox blocker is investigated. But a required feature with an unresolved gate remains incomplete, and the orchestrator must not label a reduced build “complete v1.”

### 2.3 Connectivity reality and signaling decision

WebRTC requires signaling to exchange connection descriptions/candidates. STUN helps discover addresses; TURN relays application traffic. Consequently, banning TURN preserves the PRD's direct-only board path but limits reachability behind restrictive NAT/firewalls. Display this honestly and do not silently add TURN, a VPN product, or server-routed board traffic. [Official peer-connection guide](https://webrtc.org/getting-started/peer-connections), [WebRTC specification](https://www.w3.org/TR/webrtc/).

Baseline: share a versioned invite/offer code, have the joiner return an answer code, then establish a direct channel. Gather candidates completely for the copy/paste flow; expose progress, cancel, expiry, and errors. An invite is a capability credential, not an account. A native custom URI can be a convenience when installed, but paste-to-join is always usable without URI registration.

Optional convenient path: a short room code and automatic answer exchange require a real reachable handshake broker. The orchestrator must name the actual approved service, establish availability/terms/privacy, or propose a separately deployable minimal signaling component. Do not add infrastructure without authorization. The broker carries bounded short-lived SDP/candidate/admission messages only, never document updates, AI data, cursor traffic, or exported files. Encrypt signaling payloads using an invite capability where appropriate; a six-digit code alone is not adequate peer authentication. No service availability means the manual path still works. The UI must disclose when discovery services are contacted.

An offer is single-connection negotiation, not a permanent reusable room identity. Each join attempt gets its own exchange. A survivor can issue fresh invites after the original inviter leaves. Reusable codes, if implemented, require explicit ephemeral rendezvous refresh and admission semantics.

### 2.4 Official AI support and protocol constraints

Current OpenAI documentation describes direct ChatGPT-plan access for eligible open-source/local apps. Use Doodleverse's own minimal Rust harness; do not embed Codex or treat ChatGPT sign-in as an ACP connection. Eligibility, distribution/licensing, account access, and current terms still need an implementation-time check. The public GitHub repository alone is not evidence that all eligibility requirements are met. [Official introduction](https://developers.openai.com/cookbook/articles/sign-in-with-chatgpt), [OSS integration overview](https://developers.openai.com/siwc/token-sharing-open-source).

The documented flow requires retaining a stable host identifier and issued client registration across restarts. Store them with the authorized connection record in the OS vault; never regenerate host identity on every launch to simulate statelessness. If the vault is unavailable, do not silently fall back to plaintext. Session-only BYOK can still work; ChatGPT sign-in stays unavailable until its required stable record can be kept correctly. [Host/client lifecycle](https://developers.openai.com/siwc/token-sharing-open-source).

The two minimal-harness adapters use different wire protocols: BYOK targets Chat Completions; ChatGPT direct access uses the documented Responses endpoint, supported model discovery, `store:false`, and streaming. Normalize them at the provider event boundary rather than pretending they share a request serializer. Recheck preview restrictions and use the documented instruction field instead of unsupported request fields. [Models and inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference), [Preview limitations](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations).

Use latest mutually supported **stable** ACP. During planning, upstream identifies v1 as latest and v2 as draft; negotiate only implemented versions, with clear incompatibility errors. Do not advertise draft support by changing one version number. [ACP initialization](https://agentclientprotocol.com/protocol/v1/initialization), [official Rust library](https://agentclientprotocol.com/libraries/rust), [SDK v2 status](https://github.com/agentclientprotocol/rust-sdk/blob/main/md/protocol-v2.md).

## 3. Architecture and ownership boundaries

### 3.1 Workspace layout

Use a modest Cargo workspace that makes ownership enforceable. Avoid a crate for every helper. This is the proposed layout to establish after G0:

```text
doodleverse/
  Cargo.toml, Cargo.lock, rust-toolchain.toml
  AGENTS.md                       repository rules, short; plans stay in files
  README.md, CHANGELOG.md
  IMPLEMENTATION_PLAN.md
  crates/
    doodle-types/                  shared IDs, data types, limits, command/event contracts
    doodle-document/               yrs schema, commands, projection, grouping, undo
    doodle-canvas/                 pure geometry, camera, hit tests, brush, interaction
    doodle-render/                 wgpu board pipelines, caches, grid, export rendering
    doodle-network/                WebRTC, invites, mesh, awareness, sync transport
    doodle-files/                  envelope, decode/validate, independent import payloads
    doodle-ai/                     provider trait, mock, BYOK, ChatGPT, ACP, cancellation
    doodle-platform/               input hooks, OS vault, file-dialog/native integration
    doodle-app/                    binary, UI, controller, runtime integration and shutdown
  tests/
    fixtures/                     synthetic boards, pointer traces, fake AI streams
    integration/                  harness definitions and scenario fixtures
  tools/                          reproducible test/packaging orchestration, no secrets
  assets/                         approved icons/fonts/licenses when available
  docs/
    PRODUCT_REQUIREMENTS.md
    decisions.md, architecture.md, contracts.md
    orchestration/status.md, orchestration/tasks/
    verification/, checkpoints.md
  .github/workflows/
```

Root-level test fixtures are not automatically executable Cargo integration tests. Put actual test binaries under the owning crate's `tests/`, or define an explicit integration-test workspace member. The fixture/harness tasks must make the commands below real, not leave unregistered test files.

### 3.2 Dependency direction

`doodle-types` has no UI, GPU, network, credential, or provider dependencies. Document, canvas, files, network, and AI depend on these contracts as needed, not on the application binary. The renderer consumes immutable document projections and local previews, not live `yrs` transactions. Platform exposes narrow native capabilities. `doodle-app` composes everything and owns cross-module behavior. Never create cycles such as network importing app UI or document importing credential storage.

The document owner exposes encoded updates and normalized projections; the network owner does not manipulate schema fields. Files decode to a validated import payload; they do not mutate the live board. AI returns local events and never takes a reference to the shared document. Only the app/controller translates an explicit pin action into a shared document command.

### 3.3 Runtime and thread model

- Native main thread owns the window, UI, GPU surface, input dispatch, and app controller. Honor platform-specific requirements for dialogs and window APIs.
- One document executor on that thread owns `yrs::Doc` initially. All edits, remote updates, and export-snapshot requests pass through it; do not share a live document under arbitrary worker locks. If profiling later justifies a dedicated document actor, change the boundary in one reviewed ADR.
- A managed async runtime owns networking, HTTP streams, ACP I/O, timers, and cancellation. CPU-heavy decoding/tessellation runs in bounded worker jobs. File dialogs use their documented platform dispatch, not an assumption that every native API can run on a Tokio worker.
- Cross-thread channels are bounded. Durable document messages cannot be silently dropped; cursor/preview messages may be coalesced or dropped when stale. Backpressure must not stall pointer sampling or rendering.
- UI events carry session/request generations. Drop stale events after card deletion, provider replacement, room replacement, or shutdown. Wake/redraw on relevant changes; avoid a busy idle loop.
- Shutdown has one orchestrator: stop accepting actions, cancel providers/room tasks, resolve pending permissions, terminate owned ACP process trees, clear temporary workspaces, release the document and GPU resources, and exit within a documented timeout.

## 4. Contracts to freeze before parallel feature work

Write the concrete Rust signatures and serialization fixtures to `docs/contracts.md` and `doodle-types`. The names below define responsibilities, not a requirement to copy pseudocode that does not compile.

### 4.1 Identity and version boundaries

Use fresh cryptographically random room/invite capabilities and fresh per-process peer/client IDs. Distinguish `ObjectId`, `PeerId`, `RoomId`, `RequestId`, and `ImportId`; do not pass arbitrary strings between these domains. Never reuse an exported `yrs` client ID as an active replica identity.

Version these independently: application release, wire protocol, portable-file envelope, document schema, object/brush geometry schema, and provider adapter contract. A version header detects compatibility; it does not make an unknown future document safely editable. Reject unsupported major versions without changing the current board. Migrations must be explicit, fixture-tested, and performed in isolation.

### 4.2 Shared versus local state

| State | Lifetime/location | Must never enter |
|---|---|---|
| Finalized strokes, groups, pinned AI statements | Shared document; explicit board files | Provider requests unless explicitly typed by the user |
| Name/color/cursor, transient stroke/drag preview | Ephemeral room awareness with liveness expiry | Board snapshots or vault |
| Camera, grid/background, selection, tool/brush settings | Local RAM per session | Shared document or persistent preferences |
| Unpinned card position/question/draft alternatives | Local RAM | Room messages, portable files, logs, vault |
| API key, OAuth tokens, approved connection config and registration metadata | OS credential vault, local to that user | CRDT, exported boards, invites, telemetry, task output |
| ACP provider-owned auth/history | External agent's domain | Claims that Doodleverse erases or controls it |

Keep local draft-card types structurally separate from shared card types. Shared serializable structs must not contain a generic “provider settings” or “local metadata” map that could accidentally carry secrets.

### 4.3 Document schema and semantic rules

Recommended `yrs` roots: versioned document metadata and a map of objects keyed by immutable `ObjectId`. Each object is a field-granular shared map. Do not store the whole board or every object as one overwriteable JSON blob. Use atomic values only where a coherent unit is needed, such as a pose or finalized stroke payload.

| Kind | Required shared fields |
|---|---|
| Stroke | ID, kind/schema, parent ID, order key, atomic world pose, immutable canonical outline/path, style, brush-geometry version |
| Group | ID, kind/schema, parent ID, order key; descendants inferred from parent membership |
| Pinned statement | ID, kind/schema, parent ID, order key, world pose, chosen answer text, explicit display dimensions/style |

Group membership has one authority: the child's `parent_id`. Do not keep a second authoritative child list. Order keys plus `ObjectId` as a deterministic tiebreaker define rendering order. The document worker must choose and test an ordering-key algorithm that permits insertion without rewriting the entire scene.

**World-space group model:** leaf objects keep their complete world pose in one atomic field. Groups have no accumulated transform; bounds are derived from their current descendants. Moving a group computes a delta and applies it once to each current leaf descendant in one local transaction. Group/ungroup changes membership without changing leaf world positions. Nested groups are supported; ungroup affects the selected group level. This makes sequential grouping stable and avoids baking a remote parent's transform twice. A concurrent drag of the same leaf resolves by the CRDT winner for its atomic pose; it is not an additive merge of both drags.

Define a total deterministic projection for missing/deleted parents and cycles: treat missing/non-group parents as root; for each parent cycle, ignore the edge from the lexicographically greatest object ID in the cycle. Empty groups have no occupied bounds and are not rendered as content. Apply the same projection on every peer and during file validation; do not start competing background “repair” writes. Local commands prevent invalid hierarchies, while remote concurrency may still require the projection. Document the visual consequence of concurrent reparent/delete without claiming every concurrent intention is preserved.

Deleted objects cannot be resurrected by late drag or provider events. Track object incarnation/command preconditions so an old command cannot recreate a missing object. Finalized stroke payloads are immutable; changing style does not rewrite their geometry unless a separate explicit operation is defined.

Transaction origins distinguish local user commands, remote updates, imports, and internal maintenance. Use a local-only undo scope that excludes remote-origin transactions. A completed stroke, completed drag, import, group/ungroup, delete, or pin is one undo step. Stop undo capture at gesture boundaries. Repeated pointer samples must not become hundreds of undo steps. Yrs supplies origin-aware undo and shared-type primitives; application semantic invariants remain our responsibility. [Yrs UndoManager](https://docs.rs/yrs/latest/yrs/type.UndoManager.html), [Yrs data model](https://docs.rs/yrs/latest/yrs/).

### 4.4 App command and event boundary

Freeze commands equivalent to `CreateStroke`, `MoveObjects`, `DeleteObjects`, `GroupSelection`, `Ungroup`, `CommitImport`, `PinStatement`, `Undo`, and `Redo`. Include validation outcomes and changed-object IDs. Local camera/selection/appearance actions are not document commands.

Network outputs include `PeerConnected`, `PeerDisconnected`, `RemoteUpdate`, `AwarenessUpdate`, `SyncProgress`, and categorized failures. Document outputs include a versioned immutable `BoardProjection`, encoded local updates with their origin, and a bounded snapshot/state-vector interface. Only the controller routes these surfaces.

File interfaces provide `decode_and_validate(bytes) -> ImportPayload` and `encode(snapshot) -> bytes`; placement converts the payload into a document command. Platform interfaces provide normalized pointer events, native dialog results, and vault operations. Use injected implementations for headless tests.

### 4.5 Provider contract

Use one asynchronous interface with operations equivalent to connect/status, generate, cancel, disconnect, and capabilities. A request carries request/card generation IDs, exactly one user question, optional short brainstorming instruction, and a cancellation token. It does not contain a board/document handle, peer context, file paths, tools, or conversation history.

Normalize events to `Started`, `TextDelta`, `Completed`, `Failed`, `Cancelled`, and, only for ACP, a typed `PermissionRequested`. Completion is a terminal success event, not merely a closed socket. A partial stream followed by failure is visibly partial. Every request ends in exactly one terminal state, and every permission callback is resolved once.

**Initial proposed limits:** four simultaneous local generations, one per card; 64 KiB UTF-8 question text; 1 MiB answer text; bounded per-request events and stderr; reasonable timeouts distinct from idle-stream timeouts. These are tunable engineering budgets, not measured product capacity. Centralize them and show recoverable errors when exceeded.

## 5. Canvas, brush, and editor behavior

### 5.1 Camera and rendering

Represent world coordinates with `f64`, converting to camera-relative GPU coordinates near the visible region. “Infinite” means pannable/zoomable without fixed page edges, not unbounded memory or reliable rendering at every floating-point magnitude. Validate finite coordinates and choose documented safe magnitude/zoom limits in G0.

Provide screen/world conversions, cursor-anchored zoom, middle-button or Space-drag pan, trackpad wheel behavior, reset/fit-to-content, viewport clipping, and DPI-aware hit tolerances. Freeze shortcuts in the UI contract and ensure text input owns editing keys while focused. macOS uses Cmd equivalents where appropriate; preserve the explicitly requested Ctrl+G and Ctrl+Shift+G conventions on Windows/Linux.

Render backgrounds and grids procedurally with a density/LOD policy so zoomed-out grids do not alias or overwhelm the screen. Grid spacing is in world units; picker opacity/color are local. Selection handles/cursor labels remain readable in screen units. Dirty-object mesh caches and visible-region culling precede speculative optimizations. Drop/recreate GPU caches after surface/device loss without losing the in-memory board.

Keep interactive overlays outside exported geometry: selection, placement ghosts, remote cursors, AI drafts, and tool previews. Pinned statements use stable explicit dimensions and shared text, with local text layout at a documented scale/font set. Verify non-ASCII input and rendering; avoid silently dropping glyphs.

### 5.2 Pointer pipeline and pressure

Normalize input at one boundary: pointer ID/type, timestamp, screen/world position, optional normalized hardware pressure, buttons, tilt only if actually supported, and cancellation/focus state. `None` pressure means unavailable; zero is a valid pressure value. Suppress duplicate compatibility mouse events from an active pen. Buffer/coalesce samples safely during high-rate input without dropping the stroke endpoint.

Pipeline: sample collection -> outlier/duplicate filtering -> resampling -> pressure profile -> curve fitting/smoothing -> variable-width outline -> canonical stored geometry -> tessellation. Keep pure deterministic geometry functions independently testable. Render an immediate local preview during drawing; finalize and commit the durable stroke at pointer-up. Remote in-progress previews use bounded ephemeral messages, then disappear when the finalized object arrives. A crash before pointer-up need not recover an uncommitted stroke.

For mouse simulation, combine bounded inverse-velocity response with a progression envelope whose default ending is heavier than the start. Use elapsed time and arc length rather than frame count. Finalization can refine the progression envelope; ensure the preview/final change is small and documented. Explicit taper settings override the corresponding endpoint profile, so a user-selected end taper can intentionally produce a thin endpoint. The default preset must satisfy the PRD's heavier-ending behavior instead of inheriting a generic symmetric taper.

Store a canonical fitted path/outline and geometry version so later smoothing-code changes do not alter old strokes or produce divergent outlines on another peer. Retaining raw samples is optional and must be bounded; do not replicate every sample as a separate CRDT edit. Style includes base size, color, opacity, pressure sensitivity, smoothing, and start/end taper parameters. Bound settings and numeric values before rendering.

Required edge cases: tap/dot, short line, no time delta, stopped mouse, repeated coordinates, zero/max pressure, fast corners, high-rate pen, pointer leaving window, focus loss, cancellation, tool change mid-stroke, high DPI, extreme allowed zoom, device reconnect, and hardware without pressure. Define whether cancellation commits the sampled partial stroke or discards it; proposed default is discard for explicit cancel and finalize a valid partial stroke on ordinary focus loss, consistently on every OS.

### 5.3 Editing and group behavior

Topmost hit-testing respects draw order and stroke thickness. Click selects; modifier click toggles; marquee selects visible intersecting geometry according to one documented rule. A group selects as a unit unless an explicit nested-selection gesture is used. Delete, move, group, ungroup, undo, and redo work from both discoverable UI actions and shortcuts.

Keep drag previews local/ephemeral; commit one atomic pose per affected leaf at completion. Recheck selected object existence before committing. Concurrent remote updates do not trigger an automatic local undo capture. Do not trap keyboard focus on the canvas or allow global shortcuts to destroy text while the question box is active.

## 6. Portable files and import placement

### 6.1 Format and export

Proposed extension: `.doodleverse`. Specify a small binary envelope with magic bytes, envelope version, document-schema version, codec ID, flags, declared payload length, and integrity checksum. The payload is a serialized shared-document state update, not a debug dump of the application. Do not introduce compression before bounded uncompressed decoding is correct; if added, validate decompressed size before allocation growth. A checksum detects corruption, not malicious authenticity.

Export an immutable document snapshot at a known local revision, then encode/write off the UI path. Do not hold a document transaction while an OS dialog is open. Write to a sibling temporary file and atomically replace only after success where supported; a failed export must not destroy an existing file. Show cancel, access denied, disk-full, changed target, and encoding failures. Unsynced local shared edits are included; only the local replica's current state can be promised.

Files contain shared objects only. Do not include awareness, camera/grid/background choices, local AI cards/drafts/alternatives, provider connection records, source filenames, invite secrets, logs, or local undo stacks. A serialized CRDT can retain historical data beyond the visible projection; audit that behavior explicitly. Prefer a freshly materialized export document containing only current shared objects to avoid unintentionally carrying deleted objects or identifiers/history, while still exporting a valid CRDT snapshot. Document this export compaction semantic and test deleted sentinel text absence.

Native dialogs use the OS integration and its actual platform restrictions. Linux packaging must account for the selected portal/GTK backend; macOS dialog dispatch must respect the native application lifecycle. [rfd platform documentation](https://docs.rs/rfd/latest/rfd/index.html).

### 6.2 Import algorithm

1. Read the selected file with a hard byte limit before allocation, decode its header, reject unsupported versions/flags, and verify length/checksum.
2. Apply the decoded update only to an isolated temporary `yrs` document. Use bounded worker decoding and resource/time limits; untrusted data must not freeze the UI or mutate the active room. Test malformed updates and deep graphs. If the decoder cannot be safely bounded in-process, isolate file validation in a restricted helper process before shipping.
3. Materialize the current visible board and validate every field: finite geometry, supported kinds, text/point/object limits, parent references, and group depth. Never load scripts, external URLs, executable attachments, or arbitrary paths from a board.
4. Deep-clone to a neutral `ImportPayload`, generate fresh application object IDs, rewrite all parent/internal references, and create fresh CRDT values on commit. Do not reuse integrated shared references or imported replica identities. Updates are idempotent, so applying the original update directly would fail repeated-independent-import semantics. [Yjs document update behavior](https://docs.yjs.dev/api/document-updates), [Yrs preliminary versus integrated types](https://docs.rs/yrs/latest/yrs/#preliminary-vs-integrated-types).
5. Compute tight bounds including stroke width and all transformed descendants. Normalize the imported bounds' minimum corner to `(0,0)` while preserving relative positions. If the current board contains no visible shared leaves, propose placement at origin.
6. Otherwise enter local ghost placement. The payload/ghost is not in the shared document, and neither its geometry nor source filename is transmitted before commit.
7. On drop, re-evaluate occupied regions from the latest projection. If valid, translate leaf poses, create one new wrapper group containing the imported roots, and commit all cloned objects and membership in one transaction/undo step. Preserve nested groups and editable objects; never flatten to a bitmap.
8. Escape/cancel/file error discards the pending payload without any shared mutation. Empty imports produce an explanatory no-op. If another peer filled the board while origin placement was pending, switch to normal placement instead of forcing an overlap.

### 6.3 Occupancy rule

Use each current top-level leaf's world AABB and each top-level group's derived descendant AABB as occupied regions. This deliberately reserves interior gaps while grouped. Ungrouping exposes the children as separate regions and frees gaps; deletion and moving immediately update the spatial index. Ignore local draft cards, cursors, selection, and other placement ghosts for shared occupancy.

The incoming payload uses its full wrapper AABB for the overlap check. Touching edges is allowed within a small documented geometry epsilon; intersecting interiors are not. A red ghost and unavailable cursor explain invalid placement. Rebuild bounds from actual objects, never retain stale “import rectangles.”

Collision is an optimistic local rule. Two peers may both validate and drop overlapping imports concurrently; preserve both, converge, and let users drag them apart. Do not add a central lock, delete one import, or retroactively reject a valid remote edit.

### 6.4 Optional PNG/SVG scope

Only after portable files pass: export all shared content or the selected shared objects through a clearly labeled choice; exclude drafts and interaction overlays. Offer transparent or current local canvas background explicitly, bound PNG dimensions/memory, escape SVG text, and embed no active content/external references. Use canonical stroke geometry so visual exports match the canvas. Optional exports do not imply importing arbitrary SVGs/images.

## 7. Rooms, transport, and distributed behavior

### 7.1 Room state machine

Model the local session explicitly: `LocalOnly`, `CreatingInvite`, `Joining`, `Connecting`, `Synchronizing`, `Connected`, `Degraded`, `DisconnectedLocal`, `Leaving`, and `Closed`. Track per-peer handshake/sync/liveness state separately from overall room status. A socket existing is not proof that document synchronization finished.

- The launch board is immediately editable without networking. Sharing gives it a fresh room identity and ephemeral capability.
- Join from an empty local board normally. If the joining user has local content, show a clear choice to export/discard it or explicitly carry it into the room as an imported group after joining; never silently merge unrelated local state or destroy it.
- Admission and version/capability checks precede document disclosure. Every admitted peer can edit; there are no account roles or administrative permissions in v1.
- On late join, take a state vector/snapshot from a synchronized peer while allowing later incremental updates to queue. Reconcile again at the end. Do not declare ready based on one initial transfer if edits occurred concurrently.
- The inviter is not a server or permanent owner. Its exit must leave the remaining peers editing and capable of issuing fresh invitations.
- A lost link retains the local in-memory board and permits export. A deliberate Leave disconnects the room and keeps a local copy unless the user explicitly clears it. Closing/clearing the final process destroys unexported content.
- A network partition produces two in-memory components. Reconnecting while both still run merges their updates; closing an isolated component loses any unexported edits that never reached another peer. Never promise recovery after all replicas close.
- Presence names/colors are session-only, initially editable from a small room panel; they are labels, not authenticated real-world identities.

### 7.2 Full mesh establishment and repair

Target eight peers, which requires seven direct links per peer and 28 pairwise links overall. After an authenticated bootstrap link, exchange roster and connection negotiations through established encrypted peer channels. Establish missing direct pairs with deterministic initiator ordering by peer ID to avoid glare. The peer roster must converge through concurrent joins/leaves; do not treat the bootstrap peer as an irreplaceable coordinator.

Limit capacity to eight and make over-capacity admission recoverable. Concurrent joins can race: use a bounded provisional handshake pool and reconcile the admitted set deterministically before any additional document disclosure. An honest-network capacity rule is not protection from a malicious invited editor.

Track mesh completeness. A room with missing pairs is `Degraded`, not falsely fully connected. Retry missing links with bounded exponential backoff and fresh ICE negotiation where supported. A connected component can still edit; no server fallback is introduced. If direct repair fails, explain which peer link failed and offer a fresh invite or export.

Invites contain protocol/room identity, random capability, negotiation ID, expiry, and necessary peer/DTLS-fingerprint binding. Reject wrong-room responses, replayed negotiation IDs, expired codes, mismatched fingerprints, malformed SDP, and incompatible versions before importing document data. If an optional broker is used, it must not be able to substitute peers undetectably. Choose a reviewed cryptographic construction during F1 rather than inventing one in application code.

### 7.3 Data channels, reconciliation, and limits

Use reliable ordered traffic for handshake/control/document sync and a separate disposable channel or coalesced queue for cursor and drawing previews. Do not permit presence floods to starve a board snapshot. Message envelopes carry wire version, room/session generation, type, message/transfer ID, bounded payload length, and the authenticated sender association.

**Proposed starting limits, to validate in F1:** 16 KiB application chunks; 64 MiB board snapshot/payload; 100,000 visible objects; one million points/vertices per single stroke and a lower total geometry-memory budget; bounded nesting depth (for example 64); eight admitted peers; fixed per-peer outbound/inbound memory budgets and transfer deadlines. Final numeric limits must be coherent across files, wire reassembly, document projection, and rendering. Avoid accepting a file that cannot subsequently sync. Do not multiply a maximum allocation by every peer without a room-level budget.

Chunk reliable transfers with declared total size, offsets/sequence, transfer integrity check, timeout, cancellation, and bounded reassembly. Validate before allocating and stream where possible. For a slow peer, coalesce redundant document updates or schedule state-vector reconciliation; do not grow queues forever. Show transfer progress and allow leaving/canceling.

Locally authored document updates fan out to live peers. Applying a remote update must not trigger blind immediate rebroadcast. At connection and periodically or after gaps, exchange state vectors and missing updates with each live peer. This anti-entropy can propagate edits through surviving peers in a degraded mesh; it is still peer-to-peer traffic, never server traffic, and must be documented as repair behavior. Tag origins and bound reconciliation to avoid echo loops. Idempotent CRDT application alone does not prevent excessive network retransmission.

Awareness carries cursor position, label/color, selection/drag preview if included, and sequence/liveness information. Cap names, preview geometry, and update rates (proposed 20–30 Hz); expire stale entries and remove cleanly on leave. Use expiry/clock behavior rather than durable objects for cursors. [Yrs awareness](https://docs.rs/yrs/latest/yrs/sync/awareness/struct.Awareness.html).

### 7.4 Direct-only privacy and failure evidence

The app may contact discovery/signaling infrastructure and the selected AI provider for their stated purposes; “no backend” is not “no network services of any kind.” Discovery can expose network metadata. Never claim peers do not learn each other's addresses. Board data, pinned replies, presence, and previews must travel only over peer channels.

Verify selected ICE candidate types, not just a green UI indicator. Reject relay candidates/configuration for v1. Test two real networks, LAN, hotspot, offline launch, blocked UDP/firewall failure, and restrictive NAT where available. Record unsupported cases. A local loopback-only test does not establish internet connectivity.

Peers are authorized collaborators, not fully trusted parsers. Validate update framing and projected geometry, rate-limit traffic, isolate/bound decoders where necessary, and permit disconnecting abusive links. No schema validator can prevent an admitted editor from reading or exporting the shared board; do not invent access-control claims.

## 8. AI cards, providers, credentials, and containment

### 8.1 Card interaction and privacy

State machine: `LocalDraft -> Submitting -> Streaming -> Alternatives -> PinnedShared`; cancellation/errors return to a recoverable local draft/alternatives state. A card holds one question; “Generate another” creates another candidate for that question, not a conversation thread. Changing the question begins a fresh generation set. Keep old candidates only in RAM until the user discards them or the session ends.

**Confirmed card presentation:** use the selected mascot as a small avatar in a corner of the card/header, with the question/input below that header and the streamed answer below the question. Keep this vertical reading order: mascot, question, answer. Alternatives, regeneration, cancellation, and Pin controls remain secondary to that content. The mascot identifies the AI buddy across ACP, ChatGPT, and BYOK; do not substitute a different mascot for each provider. Show real provider/connection status separately rather than implying that the character is a connected model. Use his confirmed name, “Dibby,” in the card label and AI entry points, for example “Ask Dibby.” Further placement guidance is in section 16.

The entire unpinned card, including position and question, remains local. On pin, preview the exact text to be shared and create one ordinary shared statement at the card position with the selected answer. By default do not share the question, unused candidates, provider identity, request IDs, usage data, or hidden instructions. If the user later asks to share question text, make that a separate explicit option. Peers receive a normal movable/groupable/deletable statement with no connection to the originator's provider.

Only a local user action invokes an AI provider. Receiving a card, joining a room, importing a file, or replaying a CRDT update can never spend another user's API credits or subscription allowance. The request includes only the typed question and optional short instruction; no board scrape, selection context, peer names, automatic attachments, or background enrichment.

Cancel aborts provider I/O and marks partial text accurately. Card deletion, question replacement, provider replacement, disconnect, and shutdown invalidate request generations; late chunks cannot resurrect a card or overwrite another request. Pin is available for a completed selected candidate; if partial text is retained after a failure, an explicit “use partial text” action is required rather than silently presenting it as a completed answer. Undoing a pin removes its shared statement and does not regenerate anything.

### 8.2 BYOK adapter

Expose exactly the PRD's three fields: base URL, API key, model name. Treat the base URL consistently as the API root, document whether it includes `/v1`, normalize one trailing slash, and append the Chat Completions path once. Do not guess many endpoints or duplicate `/v1`. Allow a deliberately blank key for local unauthenticated endpoints.

Use HTTPS for remote endpoints. Support explicit HTTP for loopback/local providers; any broader plaintext endpoint needs a visible user choice, not silent downgrading. Do not send Authorization credentials across origin-changing redirects. Credentials in URLs are rejected. Explain that prompts go to the selected endpoint; a local address may have its own provider persistence policy.

Implement a real incremental SSE parser: split lines/events, partial UTF-8, empty deltas, comments, finish reasons, malformed JSON, provider error payloads, missing terminators, disconnects, HTTP 401/403/429/5xx, and cancellation. Never execute returned tool calls. A provider returning only tool content produces an unsupported-response error. Do not automatically retry a potentially accepted billable request without a deliberate user action. [Official streaming guidance](https://developers.openai.com/api/docs/guides/streaming-responses).

### 8.3 ChatGPT sign-in adapter

Perform the documented public-client registration/sign-in using the system browser, loopback callback, PKCE S256, fresh state and nonce, complete token/issuer/audience validation, and granted-scope checks. Reuse the issued client registration and stable host record. Never embed another app's client ID, read Codex credentials, scrape browser cookies, or call undocumented ChatGPT endpoints. Identity-only success does not prove inference access. [Registration and sign-in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in).

Discover models for the actual account and build only the currently supported Responses requests. The shared harness receives normalized text events; Responses-specific parsing stays in this adapter. A terminal provider error after text is an error with partial content. Test expiry, refresh, usage limits, denied inference scope, account/workspace restrictions, sign-in cancellation, callback timeout, occupied callback port, and malformed callback parameters. [Models/inference contract](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference).

Before live integration and again before release, verify the app's license/distribution model, user eligibility, local execution requirements, subscription-use restrictions, and no-charge conditions for this integration. Do not offer other peers a model proxy. If access or terms cannot be satisfied, record the precise blocker; do not secretly substitute BYOK or ACP and call the ChatGPT requirement complete. [Current SIWC terms](https://openai.com/policies/sign-in-with-chatgpt-terms/).

### 8.4 Vault lifecycle

Define a narrow vault service namespace keyed to Doodleverse and local provider profiles. Store secrets plus required registration metadata and connection configuration there, not plaintext settings files. Keep prompts/drafts/answers out of this record. Test Windows Credential Manager, macOS Keychain, and the selected Linux secret-service/keyring backend through the actual adapter; a crate compiling is not a vault integration test.

Vault locked/unavailable/write-denied results are explicit. Offer reconnect/session-only BYOK where possible, never a plaintext fallback. Synchronize refresh to prevent parallel token-refresh races. Disconnect cancels active generations before deleting connection secrets; “forget connection” removes its local record. Keep lifecycle-required nonsecret registration data only as required by the documented provider lifecycle, explain any retained identifiers, and provide a documented full reset path rather than repeatedly rotating identifiers on ordinary sign-out. Report local sign-out and remote revocation separately. [Accounts and sessions](https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions).

AI provider servers and external agent software may retain their own data under their policies. Doodleverse's no-board-storage promise does not imply those external systems are stateless or that OS swap/crash artifacts can be categorically prevented. Avoid app-created content logs and automatic crash-content uploads.

### 8.5 ACP adapter and actual containment

Connect a user-selected installed ACP-capable executable with explicit arguments; avoid shell interpolation. Probe the installed version/capabilities and initialize through the official Rust SDK over stdio. Negotiate supported stable protocol versions and handle auth/missing executable/process exit/incompatible agents clearly. Example agent names are not proof that every installed version speaks ACP directly; a bridge is a separate visible dependency.

Give each card question a fresh agent session when possible so “one question” does not accumulate a hidden conversation. Agent authentication may be retained by the agent outside Doodleverse. Launch in a fresh temporary working directory with a minimal explicit environment and no inherited Doodleverse API secrets. Disable optional client filesystem/terminal capabilities unless a reviewed feature actually needs them; do not remove the distinction between the agent's own harness and the minimal harness.

Surface permission requests with operation details and Allow once/Deny/Cancel semantics. No automatic approval from remote content, model output, previous unrelated sessions, or a generic trust flag. Bound stdout/stderr and never let stderr become protocol traffic. Resolve every pending permission on cancellation, agent exit, card deletion, and shutdown. Reap and terminate only owned subprocess trees; do not kill unrelated user agent instances.

**Containment gate:** ACP does not sandbox an agent, and a throwaway current directory does not confine its own tools. F2 must prove OS-enforced boundaries for each supported platform using a malicious test helper that attempts outside-directory read/write, child-process escape, environment credential access, and forbidden client capability requests. Define intended permitted networking/auth paths and temporary workspace access. Investigate Windows restricted-token/AppContainer/job controls, macOS supported sandbox mechanisms, and Linux namespace/sandbox tooling; choose based on demonstrated compatibility, not a security label. Job/process groups alone are process cleanup, not filesystem isolation. [ACP security considerations](https://agentclientprotocol.com/protocol/v1/security).

If no viable containment exists for a platform/agent combination, show ACP as unavailable there and keep the requirement blocked. Do not claim containment based on permission dialogs, and do not quietly run with the user's full privileges. A trusted-agent mode would change the PRD promise and requires a separate explicit user decision. Provider/BYOK integration and local drawing can continue while this gate is unresolved.

## 9. Orchestrator operating procedure

### 9.1 Integration ownership

The orchestrator owns the root manifest, lockfile, toolchain, `doodle-types`, shared contracts, central app controller/runtime, `.github/workflows`, milestone ledger, version numbers, integration commits, and final release evidence. Delegate feature implementation, but retain responsibility for the assembled behavior. A worker's unit-test pass is not integrated-app verification.

Only one agent edits a given file at a time. In particular, do not have canvas, networking, and AI workers all edit `main.rs`, a central enum, or the lockfile. Dependency additions and contract changes are requests to the orchestrator; after approval it updates the shared surface and tells consumers to rebase/re-read it. Avoid large speculative shared interfaces; freeze the smallest useful contract after the feasibility spikes.

Choose the execution isolation model before spawning writers:

- If the environment supports agent-owned worktrees, create one branch/checkout per work package through its supported delegation mechanism. Start from the named prerequisite/integration commit, not an unrelated default-branch snapshot. Integrate the branch's commits once; never both merge and cherry-pick the same changes.
- If agents share one checkout, enforce file/subdirectory ownership and a single Git writer. Workers modify only allowed paths and report diffs; the orchestrator stages/commits their files. No worker checks out branches, updates the index, or runs cleanup commands in that shared tree.
- Match concurrency to actual available slots and machine resources. A safe baseline is the orchestrator plus three workers. Do not assume an unlimited number of agents or native GUI sessions.

### 9.2 Required task brief

For every delegated task, write a brief to `docs/orchestration/tasks/<task-id>.md` with this structure:

```text
Task ID / goal:
Prerequisite commit(s), decision IDs, and frozen contract version:
Owned paths (exclusive), allowed new files, forbidden paths:
Expected behavior and non-goals:
Inputs/outputs and integration hook:
Acceptance scenarios and exact focused commands:
Dependency/license requests (do not edit root lockfile):
Required evidence and known hardware/service limitations:
Return: changed paths, commit SHA if isolated, commands/results, risks, handoff notes.

You are not alone in the repository. Do not revert others' edits.
Do not change shared contracts without the integrator.
Do not start nested subagents unless explicitly authorized in this task.
Do not commit user boards, prompts, credentials, tokens, or private diagnostic captures.
```

Tasks must return concrete implementation/evidence or a specific blocker. A stub, mock, unsupported login button, or dependency declaration is not the required feature. Mocks are appropriate for contract tests and must be labeled as such.

### 9.3 Completion and scheduling loop

1. Read the current status, last verified checkpoint, decision log, task returns, and `git status`; establish what is actually integrated.
2. Pick ready tasks whose dependencies passed, with disjoint ownership. Give each worker the exact base commit and contract snapshot.
3. While workers run, handle integration/fixtures/docs or an independent task. Respect the harness's delegation wait semantics; do not substitute polling loops for a requested wait mechanism.
4. Review each return for scope, behavioral evidence, privacy boundaries, and unhandled errors. Integrate one coherent unit, resolve conflicts, and run affected checks.
5. At a milestone, run the integrated scenario yourself and update the checkpoint ledger. Only then mark it verified and push its checkpoint tag.
6. If a task is blocked, continue independent ready work, record the blocker precisely, and ask only for information/access/scope decisions that cannot be resolved locally. Do not weaken requirements silently.

Keep a compact status file with each task's state (`ready`, `running`, `review`, `integrated`, `blocked`), owner, base/head SHA, test evidence, dependency links, and next action. Keep long plans/reports in files; user-facing chat should link to the appropriate file.

## 10. Dependency graph and execution waves

Task IDs are stable. All gates below are future work, not completed checks. G0 feasibility and G1 contract readiness precede broad implementation; later gates establish working checkpoints rather than calendar deadlines.

| Wave | Ready work / parallel lanes | Integrator action | Checkpoint |
|---|---|---|---|
| 0 | P00 bootstrap, then P01 native spike / P02 network spike / P03 AI-sandbox research | Resolve feasibility results, decisions, dependency set | CP00 specification baseline; G0 findings |
| 1 | P04 contracts, executable composition, CI; then P04a native services / P05 document / P06 canvas-input / P07 transport core, queued within available slots | Freeze contracts before feature lanes diverge; promote proven spike code deliberately | G1 contracts; CP01 native foundation after P04 |
| 2 | P08 rendering-editor integration / P09 portable files / P10 room-sync integration | Wire one local board through all paths; integrate P04a/P05/P06/P07 | CP02 local editor after P08; CP03 direct two-peer board after P10 |
| 3 | P11 import placement / P12 mesh-lifecycle / P13 AI core+BYOK | Validate local privacy and shared-state boundary | CP04 portable collaborative board; CP05a BYOK when P13 passes |
| 4 | P14 ChatGPT / P15 ACP containment+adapter / P16 native UX-accessibility | Keep adapter subdirectories disjoint; integrate only tested paths | CP05 AI feature checkpoints, individually |
| 5 | P17 cross-platform runtime+packaging / P18 failure-performance-privacy suite | Reserve platform UI ownership; consolidate evidence | CP06 release candidate |
| 6 | P19 optional exports if authorized/prioritized; P20 release/recovery | Full required-scope audit, approved assets, release notes | CP07 v1.0.0 or truthful blocked report |

Dependency details: P04 consumes P01–P03 results; P04a and P05–P07 require P04. P08 requires P04a/P05/P06. P09's codec can start after P05 and the file contract; real dialog acceptance also requires P04a. P10 requires P05/P07 plus the app controller. P11 requires P08/P09/P10. P12 requires P10. P13 requires the provider contract and P08's local-card surface; remembered connections need P04a, shared-pin acceptance needs P10, and its final file/privacy acceptance needs P09/P11. It can start before P11 if ownership permits, but CP05a waits for those checks. P14/P15 require P13, relevant P03 gates, and completed native vault/platform services. P16 follows working interactions, not speculative screens. P17 starts build coverage earlier but its final gate requires all required features. P18 grows from P05 onward; its final integrated run requires P11–P17. P20 requires P17/P18 and all required acceptance rows. P19 never displaces a required failed task.

The long pole is likely to be native pressure, network reachability/signaling, ACP containment, or unavailable platform access, not writing controls. Do not promise a calendar estimate until G0 is complete. Reduce context switching by assigning one owner per feature lane and reusing that owner for fixes.

## 11. Work packages with acceptance criteria

### P00 — Repository and specification baseline

**Owner:** orchestrator. **Prerequisites:** explicit implementation authorization.

Inspect local and remote state again. If still empty, initialize in the existing project folder, add the provided remote as `origin`, and create the initial default branch; if refs now exist, fetch and integrate that history instead of overwriting it. Preserve the PRD and this plan. Create decisions/status/checkpoint records, `.gitignore`, a concise README, and repository ownership rules. Agree on source licensing before relying on open-source-provider eligibility or public distribution; do not fabricate user approval of a license.

**Done:** the source documents and decisions are committed/pushed as CP00 with a recorded SHA; no product functionality is claimed. The user can locate the plan and baseline in GitHub. Git authentication failures are recorded without putting tokens in commands or files.

### P01 — Native UI, GPU, input, and platform feasibility

**Owner:** native-platform worker. **Owned paths:** temporary `spikes/native/`, `docs/verification/P01.md`; no root dependency edits without integrator.

Prove stable dependencies, native window/control/canvas composition, actual pointer delivery, high DPI and surface lifecycle. Explore missing tablet paths with small platform adapters. Compile on each available target OS and record actual runtime/hardware gaps. Check AccessKit/text entry and feature flags; do not enable storage/persistence features by default.

**Done:** the orchestrator can reproduce a native canvas and a captured pressure trace on available real hardware; every remaining OS/hardware gate is named with a concrete test path. Framework selection D03 is validated or replaced before P04, not after workers build competing integrations.

### P02 — Direct WebRTC feasibility and invitation contract

**Owner:** networking worker. **Owned paths:** `spikes/network/`, `docs/verification/P02.md`.

Build a two-process data-channel exchange with the proposed current crate/runtime. Demonstrate complete offer and answer handling, selected direct candidates, large-message chunking, cancellation, and clean close. Design room capability/fingerprint binding, mesh negotiation, limits, and manual invite UX. Determine whether any optional broker is actually available and authorized.

**Done:** reproducible native exchange plus packet/channel evidence; no TURN configuration; a documented unsupported-network outcome; exact wire/signaling recommendations for P04. Do not deploy infrastructure as part of this spike without separate authorization.

### P03 — Provider, vault, and ACP containment feasibility

**Owner:** AI/security worker. **Owned paths:** `spikes/ai/`, `docs/verification/P03.md`.

Recheck official ChatGPT eligibility/auth/inference docs and ACP SDK/protocol; select vault backends and test locked/unavailable behavior. Define OS sandbox options with malicious-helper tests. Use mocks where accounts are unavailable and distinguish those results from live provider success. A real eligible user's login is performed through the official local flow; never ask them to paste account passwords into the agent conversation.

**Done:** supported/blocked matrix for all three provider paths and OS containment, scoped dependency requests, auth-record schema, and a protocol fixture. Required live or sandbox proof still missing is a blocker entry, not a passed spike.

### P04 — Workspace, contracts, test harness, and CI

**Owner:** orchestrator. **Owned paths:** root workspace/lock/toolchain, `doodle-types`, central `doodle-app` composition, CI, architecture/contracts.

Create the actual Cargo workspace, main-thread event loop, managed async runtime, bounded event queues, typed commands/events, limits, fake platform/provider/transport interfaces, fixture registration, and deterministic shutdown. Establish fmt/clippy/tests/build jobs on Windows/macOS/Linux. CI must exercise default required adapters and compile feature combinations intentionally; do not hide a required adapter behind a never-built feature.

**Done:** empty native canvas executable, no provider/network gate at launch, typed mock events round-trip to UI, shutdown test, CI green on available runners, committed toolchain/lockfile and frozen G1 contracts. App does not write app-owned content/preferences on close.

### P04a — Production native input, dialogs, and credential-vault services

**Owner:** native-platform worker. **Prerequisites:** P04 contracts plus P01/P03 findings. **Owned paths:** `doodle-platform/src/input/**`, `dialogs/**`, `vault/**`, their tests, and assigned native-build configuration; sandbox paths reserved for P15.

Promote the verified platform spike into production normalized pointer adapters, pen/mouse duplicate suppression, native file-dialog dispatch, and the OS-vault abstraction. Keep native UI-thread hooks explicit and expose injected fakes for headless tests. Implement stable installation/client-registration vault records separately from per-provider account records so forgetting credentials does not accidentally rotate provider host identity. Handle vault locked/unavailable states and platform callback cancellation.

**Done:** P08 can receive real pointer samples, P09 can open/cancel actual OS dialogs, and P13/P14 can use a real native credential vault without plaintext fallback. Unit/native smoke evidence distinguishes available platforms from remaining hardware gates. The worker hands main-loop hooks to the orchestrator rather than editing shared controller files concurrently. Schedule this task within worker capacity; consumers may build against its frozen mock interface but cannot close native acceptance before it lands.

### P05 — CRDT document, projection, grouping, local undo

**Owner:** document worker. **Owned paths:** `crates/doodle-document/**`, its tests/fixtures; no shared enums/manifest edits.

Implement the schema and validated commands, deterministic ordering/hierarchy projection, world-space group behavior, transaction origins, update/state-vector interfaces, export materialization, and local undo/redo. Include property tests for concurrent operations, duplicate/reordered updates, move/delete, reparent cycles, missing parents, and undo around remote edits.

**Done:** several independent in-memory replicas converge to equivalent normalized visible content; grouping/ungrouping preserves sequential geometry; bad projection data cannot panic the renderer; local undo does not undo a peer's edits; fresh export excludes deleted sentinels. Do not compare raw CRDT bytes as the definition of convergence.

### P06 — Camera, input, brush, hit testing, pure interaction

**Owner:** canvas worker. **Owned paths:** `crates/doodle-canvas/**`, recorded synthetic pointer fixtures. Platform adapter changes are handed to the platform owner.

Implement camera conversions, stable geometry, pressure/smoothing/taper pipeline, stroke bounds, hit testing, local gesture state, and normalized input consumption. Use recorded mouse/pen traces for deterministic geometry tests. Return render-ready canonical outlines and commands; do not mutate `yrs` directly.

**Done:** draw dots/curves/corners; default mouse trace ends heavier than it begins; hardware pressure uses actual values; explicit taper precedence works; zoom/DPI do not change stored stroke width; cancel/focus behavior matches the contract; selection/group bounds account for thickness.

### P07 — Transport core, wire framing, awareness

**Owner:** networking worker. **Owned paths:** `crates/doodle-network/**`, network tests and transport fixtures.

Promote proven spike behavior into the bounded transport interface. Implement direct connection establishment, manual signaling, authenticated room admission, version checks, chunk/reassembly/backpressure, reliable sync/control, ephemeral awareness, and cancellation. Provide deterministic fault injection without opening real connections for every unit test.

**Done:** two processes exchange bounded payloads and ephemeral presence; malformed/oversized/replayed/wrong-room messages fail safely; large transfer can cancel midstream; remote-origin events do not echo endlessly; no document schema access from the transport crate.

### P08 — Working local canvas and editor integration

**Owner:** render/UI worker for `doodle-render/**` and `doodle-app/src/ui/canvas/**`; orchestrator owns controller hooks.

Integrate GPU stroke rendering, grids/background controls, selection/move/delete, group/ungroup, undo/redo, tool settings, and local AI-card placeholders with real interaction boundaries. Reuse actual commands and geometry; placeholders must be visibly local and cannot claim a provider is connected. Handle GPU errors and idle redraws.

**Done:** a clean launch reaches a drawable canvas; all required local editing actions work through UI and shortcuts; text fields receive keys safely; grid choices are local/session-only; no dead controls; empty state remains an unobtrusive canvas hint. CP02 is a real local editor.

### P09 — Portable file codec and safe native export/import parsing

**Owner:** files worker. **Owned paths:** `crates/doodle-files/**`, codec fixtures; orchestrator/platform owner supplies dialog hooks.

Implement envelope versioning, isolated/bounded decoding, semantic validation, fresh-ID payload cloning, visible-state export, migrations/rejection behavior, and atomic writes. Add synthetic golden fixtures independent of machine-specific paths. Do not implement placement in the file codec.

**Done:** a fresh process round-trips strokes/groups/pinned statements; same file decodes into separate independent ID sets; corrupt/unsupported/truncated/oversized files leave the current board unchanged; failed writes preserve existing files; secret/draft/deleted-content sentinels are absent from exports.

### P10 — Two peers editing the same board

**Owner:** orchestrator with document/network owners fixing their modules. **Owned paths:** controller/sync glue and integration tests.

Route local updates, remote application, state-vector reconciliation, initial sync, and awareness between actual processes. Validate edit-during-sync behavior and honest status transitions. Add join-with-existing-board and clean Leave behavior. Keep UI responsive during a large transfer.

**Done:** two native app instances draw/move/delete and see equivalent boards; a late joining replica receives current content and concurrent edits; local undo preserves remote changes; link loss keeps a local exportable board; CP03 includes actual process evidence, not only in-memory transport mocks.

### P11 — Shared import grouping and live placement

**Owner:** document/interaction worker, after P09 integration; reserve `doodle-app/src/ui/import/**` and import-controller hook. Coordinate any document changes through its owner.

Connect native Open, validated import payload, live bounds/spatial index, ghost/cursor feedback, origin rule, final atomic commit, and group/ungroup shortcuts. Recompute from current content when peers edit during placement.

**Done:** first import on empty board goes to origin; subsequent invalid overlap is visibly blocked; moving/deleting/ungrouping frees the correct space; cancel makes no change; same board imported twice yields independent groups; two peers simultaneously dropping overlapping imports keep both; imported nested geometry remains editable.

### P12 — Eight-peer mesh, host departure, reconnect, signaling UX

**Owner:** networking worker. **Owned paths:** network mesh/session modules plus reserved room UI; orchestrator integrates controller changes.

Complete roster convergence, pairwise negotiation/repair, capacity/admission, inviter departure, stale invite errors, bounded reconnect, partition reconciliation, and presence expiry. Implement an optional broker only if a concrete approved choice exists; manual exchange remains tested.

**Done:** eight real processes converge with the expected 28 links on a suitable network; founder exits and survivors still edit; a survivor admits a late peer; partition/rejoin merges correctly; ninth-peer and concurrent-admission behavior is deterministic; degraded mesh is visible; board traffic never uses a server. Update CP04 after P11/P12 both pass.

### P13 — Local AI-card controller, provider interface, mock, and BYOK

**Owner:** AI-core worker. **Owned paths:** `doodle-ai/src/core/**`, `providers/mock/**`, `providers/byok/**`, tests; reserved AI-card UI owned by UI worker with frozen events.

Implement the local state machine, one-question alternatives, stream/cancel handling, limits, pin boundary, and minimal harness. Connect a deterministic fake stream first, then real Chat Completions. Add vault profile hooks through the platform owner; keep exactly the requested BYOK fields.

Coordinate with the UI owner to implement the confirmed mascot-corner -> question -> answer layout from sections 8.1 and 16. The avatar is a local UI asset, not provider input or a new CRDT field. Keep all question/draft content local until the existing explicit pin action shares the selected answer.

**Done:** generate several candidates, select/pin one, see only the selected text on peers after P10 integration, export/import it as ordinary content after P09/P11 integration; deleting or changing a streaming card cannot resurrect it; error-after-delta remains partial; local/mock and an authorized real/local compatible endpoint are distinguished in evidence; room activity never triggers provider requests. Local/provider implementation can land earlier, but full acceptance and CP05a wait for these shared-pin/file checks.

### P14 — Official ChatGPT provider and remembered connection

**Owner:** ChatGPT-adapter worker. **Owned paths:** `doodle-ai/src/providers/chatgpt/**`, provider-specific tests/docs. No edits to ACP/BYOK/shared contract without review.

Implement official registration, validated browser OAuth, stable vault metadata, model discovery, Responses streaming, refresh/sign-out, eligible-account errors, and preview-compatible input. Capture only redacted protocol evidence. The orchestrator owns dependency/terms/eligibility decisions.

**Done:** an authorized eligible user signs in through the real official flow, generates and pins an answer with no Codex runtime, relaunches into an empty board while the connection can be reused, and signs out safely. Mock auth success alone does not close the task. External eligibility/access blockers remain explicit.

### P15 — Contained ACP subprocess integration

**Owner:** ACP/platform worker. **Owned paths:** `doodle-ai/src/providers/acp/**` and assigned `doodle-platform/src/sandbox/**`; no conflicting platform/vault edits.

Implement proven per-platform containment, SDK lifecycle/version negotiation, executable configuration, minimal environment, temporary workspace, permission UI bridge, cancellation and process-tree cleanup. Validate both a test ACP agent and at least one supported real installed agent. Restrict supported agent/platform combinations to what passes the containment contract.

**Done:** prompt streams through the external agent's own harness; permissions remain local; malicious-helper containment tests pass on each claimed platform; timeout/exit/cancel resolves pending requests; no owned process/temp workspace is left after normal shutdown. Unverified containment means blocked ACP support, not an unrestricted fallback.

### P16 — Native interaction, accessibility, and complete states

**Owner:** UI worker. **Owned paths:** `doodle-app/src/ui/**` except reserved active work-package paths; design tokens and UI tests.

Polish the working canvas without expanding tool scope. Preserve a playful hand-drawn character through the drawing surface and restrained controls; the canvas, not a marketing dashboard, is the focus. Use native conventions, readable typography, visible focus, keyboard navigation, tooltips/shortcut hints, and accessible labels. Background/grid customization is functional product content, not decorative filler.

Apply the selected mascot consistently in the AI-card corner and app icon. Consider the contextual placements in section 16 only where they help communicate an action or state; do not turn the mascot into a permanent canvas overlay. Adapt the approved identity for small UI sizes without creating a replacement character. His confirmed name is Dibby. The user has authorized animated idle, one-shot hover jump, talking, thinking with a thought cloud, and working with a laptop; use the prepared mascot bundle and state contract in section 16. These visual states do not expand AI behavior.

Exercise empty, joining, syncing, degraded, offline, loading, streaming, permission, provider-error, vault-error, import-error, export-error, and unsupported states. Keep light/dark controls legible and never sync local appearance. Warn before closing/clearing an unexported changed board with Export/Discard/Cancel; do not add autosave. A warning must not claim an app crash can be prevented.

**Done:** required actions are discoverable and keyboard-operable; text editing/IME works; no orphaned modal or silent error; another peer cannot drive local permissions/provider actions; AI cards use the confirmed mascot/question/answer hierarchy; no claimed visual QA based solely on source inspection. The icon/mascot direction is selected; production-size/platform validation remains required.

### P17 — Cross-platform builds, runtime QA, and packaging

**Owner:** release/platform worker. **Owned paths:** packaging/tools and reserved platform fixes; CI changes through orchestrator.

Build native artifacts on native Windows/macOS/Linux runners. Proposed first support matrix: Windows x86-64; macOS arm64 and x86-64 when supported runners exist; Linux x86-64 with documented Wayland/X11 coverage. Publish no extra architecture claims without builds/runtime evidence. Set exact minimum OS/GPU requirements from the pinned stack, not guesses in this plan.

Package Windows binary/installer, macOS app bundle, and Linux portable/archive or desktop package appropriate to dependencies. Verify file dialogs, vault, URI handling if supplied, process cleanup, fonts, TLS, tablet input, and native runtime dependencies on clean machines. Keep signing/notarization credentials in CI secret stores. Unsigned builds are labeled unsigned; absent signing accounts are a release blocker for signed distribution, not a reason to fake it.

**Done:** install/uninstall and clean-launch evidence per claimed OS; empty-board relaunch; pressure evidence on actual supported hardware; package checksums/SBOM/dependency licenses; no development path or credential baked into binaries. If a platform is unavailable, report its runtime QA as unverified and leave the all-platform release gate open.

### P18 — Integrated failure, privacy, fuzz, and performance campaign

**Owner:** verification worker; read/review by default, focused fixes assigned back to owners. **Owned paths:** test harness, synthetic fixtures, verification reports.

Run the matrix in section 13 against the assembled app, including real multi-process and network tests, malicious files/messages, privacy sentinels, provider failure/cancellation, sandbox probes, and shutdown. Fuzz file/wire decoders with bounded resources and retain minimal non-sensitive regressions. Measure declared performance fixtures on named hardware.

**Done:** repeatable evidence identifies exact commit/toolchain/platforms; required rows pass or have explicit release-blocking issues. Cross-module bugs are reproduced and fixed, not explained away by workers' unit tests. CP06 is a release candidate only when required gates pass.

### P19 — Optional PNG/SVG exports

**Owner:** renderer/files worker. **Prerequisites:** CP04, stable geometry/file contracts, no displaced required blockers, and prioritization by the implementation orchestrator within authorized scope.

Implement the explicit selection/full-board and background behavior from section 6.4; use an offscreen path that does not depend on a giant visible window. Bound dimensions and memory, escape SVG, and compare output with rendered canvas fixtures.

**Done:** independent viewers open exports correctly, drafts/overlays are absent, large-content limits are recoverable, and optional behavior is documented. Omitting this optional task does not mean required board export is absent.

### P20 — Release audit, recovery drill, and handoff

**Owner:** orchestrator. **Prerequisites:** all required tasks and real platform/provider gates; approved icon assets for branded release.

Audit each PRD requirement against evidence, reconcile decision/task records, verify no credentials/content in source/history/artifacts, prepare release notes and user docs, and run the fresh-checkout recovery drill. Include manual offer/answer instructions, NAT limits, file format compatibility, statelessness/vault exception, provider privacy, and ACP support limits.

**Done:** reproducible build from the final verified tag, complete acceptance ledger, artifact checksums and honest platform limitations. Create a `v1.0.0` release only after its required scope passes and publication is authorized; otherwise hand off the latest verified checkpoint and explicit remaining blockers. Do not silently declare a partial subset to be full v1.

## 12. Git strategy, checkpoints, and recovery

### 12.1 Protect usable history

The user specifically wants commits and recoverable checkpoints. Treat Git history as part of the deliverable, not an end-of-project upload. After implementation is authorized, commit coherent tested units and push them to the supplied repository. Verify the actual remote/default branch before any write. No force push, destructive reset, overwritten history, deletion of unrelated files, or blanket staging.

Recommended workflow: `main` holds integrated passing work; `feat/<task-id>-<topic>` or `fix/<task-id>-<topic>` holds isolated work. Use PRs when available; keep small task changes reviewable and link task/evidence. If branch protection is available, require CI before integration. Configure it only within granted repository permissions; do not block basic checkpoints waiting for optional repository administration.

In a shared checkout the orchestrator is the only Git writer. Stage explicit owned paths after reviewing `git diff` and the index. Do not use `git add .` or `git commit -a`. Keep `target/`, logs, tokens, local vault files, provider sessions, user board exports, private packet captures, and platform temporary workspaces out of Git. Synthetic fixtures can be committed; real user content cannot be used as test data without explicit consent.

Useful commit style: `feat(P05): add origin-filtered document undo`, `fix(P11): preserve nested groups on repeat import`, `test(P12): cover inviter departure during late join`. A progress commit may be incomplete on its feature branch, but name it honestly and do not promote it to a verified checkpoint.

### 12.2 Checkpoint policy

Annotated tags use `checkpoint/NN-description`; use SemVer tags separately for actual releases. Create a checkpoint only from an integrated commit whose stated checks passed. Push its branch/commit before its tag. Tag annotations and `docs/checkpoints.md` record scope and known limits. Do not reuse or move a checkpoint tag after publication.

| ID / proposed tag | Required working behavior | Required evidence |
|---|---|---|
| CP00 `checkpoint/00-specification` | PRD, plan, confirmed decisions, ownership rules preserved | Documentation/source inspection only; explicitly no functional build |
| CP01 `checkpoint/01-native-foundation` | G0 findings, pinned toolchain/dependencies, native empty canvas and managed shutdown | Native build/smoke coverage; list hardware/platform gates still outstanding |
| CP02 `checkpoint/02-local-editor` | Drawing, camera, appearance, selection/move/delete/group/undo | P05/P06/P08 tests and real UI use; empty-board relaunch |
| CP03 `checkpoint/03-two-peer-board` | Direct two-peer edits, awareness, late join, disconnect handling | Real processes and selected candidate evidence; convergence/local-undo tests |
| CP04 `checkpoint/04-portable-mesh-board` | Portable files, independent grouped imports/placement, eight-peer lifecycle | Round trip, corruption, simultaneous placement, 28-link mesh/founder-exit evidence |
| CP05a `checkpoint/05a-ai-byok` | Local AI cards, mock+real BYOK, pin/privacy boundary | Stream/cancel/failure/privacy tests and endpoint evidence |
| CP05b `checkpoint/05b-ai-chatgpt` | Official eligible-account ChatGPT inference and remembered connection | Live OAuth/inference/vault tests; terms/eligibility review |
| CP05c `checkpoint/05c-ai-acp` | Verified contained ACP on explicitly listed platforms/agents | Real agent plus malicious-helper sandbox and process-cleanup tests |
| CP06 `checkpoint/06-release-candidate` | All required v1 features integrated across target OSes | Full acceptance/failure/privacy/performance matrix and packaged runtime QA |
| CP07 `v1.0.0` | Release audit and approved final assets | Fresh-checkout reproduction, checksums, release notes, required publication authorization |

CP05 sub-checkpoints may land in a different order because adapters are independent. Numeric names communicate scope, not permission to skip dependencies. CP01–CP05 can retain clearly documented platform gates; CP06/full v1 cannot claim three-platform support without corresponding runtime evidence. No checkpoint is a board autosave: it recovers application source/builds, while users recover board content only from explicit exports.

Each ledger entry contains: tag, commit SHA, date, task IDs, exact Rust/dependency lock reference, commands/results, runtime platforms and hardware, live versus mocked services, synthetic fixture hashes, known limitations/blockers, artifact/checksum links if any, and the next tasks. Do not store bearer tokens, prompts, SDP secrets, or user files in evidence.

### 12.3 Required verification commands

Once the workspace exists, the baseline PowerShell commands from the repository root are:

```powershell
Set-Location -LiteralPath 'D:\Apps\projects\doodleverse'
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked
git diff --check
```

Run applicable focused tests before every task integration. Run the baseline for checkpoints in CI on the platform matrix. Add explicit feature-matrix jobs if adapters/backends use features; do not assume `--all-features` is meaningful when platform backends are mutually exclusive. No live provider credentials in ordinary PR CI. Keep real provider/network/tablet tests separate, labeled, and repeatable.

The test-harness owner must add documented commands for `two-peer`, `eight-peer`, `partition-rejoin`, `import-race`, `privacy-sentinels`, and sandbox probes; scripts must be implemented before the tasks that cite them can pass. CI tests alone do not replace actual window, tablet, dialog, and clean-machine packaging checks.

### 12.4 Regression and recovery procedure

1. Stop new integration and record the failing commit, scenario, last good checkpoint, and working-tree state. Preserve unrelated user/agent changes.
2. Reproduce the last good checkpoint in a fresh sibling clone. Do not switch branches beneath running workers or overwrite the active checkout.
3. Compare failing and known-good behavior; use focused history/bisect in an isolated investigation checkout if useful. Keep diagnostics synthetic/redacted.
4. Prefer a forward fix. If necessary, revert the specific faulty integrated commit(s) with a new commit and full relevant checks. Never use `reset --hard`/force push as routine shared-history recovery.
5. Add the regression fixture, update task/checkpoint evidence, and tag a new verified checkpoint; do not relabel the old one.

Example recovery drill after at least one checkpoint has actually been pushed:

```powershell
Set-Location -LiteralPath 'D:\Apps\projects\doodleverse'
git fetch origin --tags
$doodleCheckpoint = git tag --list 'checkpoint/*' --sort=-creatordate | Where-Object { $_ -ne 'checkpoint/00-specification' } | Select-Object -First 1
if (-not $doodleCheckpoint) { throw 'No functional checkpoint tag is available.' }
$doodleRecoveryPath = Join-Path 'D:\Apps\projects' ('doodleverse-recovery-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
if (Test-Path -LiteralPath $doodleRecoveryPath) { throw 'Recovery path already exists.' }
git clone --branch $doodleCheckpoint --single-branch 'https://github.com/asterxsk/doodleverse.git' $doodleRecoveryPath
if ($LASTEXITCODE -ne 0) { throw 'Recovery clone failed.' }
Set-Location -LiteralPath $doodleRecoveryPath
cargo test --workspace --locked
if ($LASTEXITCODE -ne 0) { throw 'Checkpoint tests failed.' }
cargo build --workspace --release --locked
if ($LASTEXITCODE -ne 0) { throw 'Checkpoint build failed.' }
```

Choose a functional checkpoint from the ledger for this drill; CP00 has no Rust workspace and cannot pass a build. The script deliberately leaves the recovery directory available for inspection. It does not launch an app or prove runtime behavior. The implementation orchestrator must also run the relevant native smoke scenario from that clone before marking recovery verified.

## 13. Verification matrix and release gates

### 13.1 Testing layers

Use pure unit/property tests for geometry, file envelopes, CRDT semantics, state machines, SSE parsing, and limit enforcement. Use deterministic fake clocks/transports/providers for races and failures. Use actual processes for shared-document/mesh lifecycle, and real target systems for GPU/input/vault/dialog/sandbox behavior. Keep a small set of high-value end-to-end scenarios instead of generating tests that merely mirror every function.

For CRDT convergence, normalize the visible object graph, order, poses, geometry, and shared text and compare those values after all messages settle. Exercise duplicate, delayed, reordered, and temporarily missing updates. Include deterministic seeds/reproduction traces in failures. Do not rely on raw update byte equality or a screenshot alone.

### 13.2 PRD traceability and acceptance

| Requirement | Owner tasks | Acceptance scenario / evidence |
|---|---|---|
| PRD 1–4: native Rust, three desktops | P01/P04/P17 | Native artifacts and real runtime checks on each claimed OS; no browser/webview dependency in UI |
| Stateless board, no accounts/storage | P04/P04a/P16/P18 | Draw/use cards/join/close/relaunch: empty board, no drafts/session; inspect app-owned file writes; vault exception enumerated |
| 5.1: immediate empty canvas | P08/P17 | Launch clean and with remembered AI login; drawing works before provider/network initialization |
| 5.1: invite/join | P02/P07/P12 | Complete real offer/answer path; paste works without URI registration; stale/wrong/malformed/canceled invite states |
| 5.1: live cursors/labels | P07/P10/P12 | Distinct local names/colors visible; movements coalesce; disconnect/timeout removes cursors; no awareness in files |
| 5.1: late join/current state | P10/P12 | Join while existing peers draw/import/delete; all normalized projections converge after sync |
| 5.1: room lifetime | P12/P18 | Inviter departure preserves survivors; late join from survivor works; final close loses unexported board; link loss keeps local RAM |
| Architecture: eight-peer direct mesh | P07/P12/P18 | Eight processes, 28 direct links on a suitable network; candidate types recorded; ninth/concurrent joins bounded |
| Architecture: no server board traffic | P02/P12/P18 | Synthetic packet/transport inspection shows no board/presence on broker; no TURN/relay candidates; honest NAT failures |
| 5.2: native portable export/import | P09/P11/P17 | Fresh-process and cross-OS round trip; native dialogs cancel safely; denied paths/disk-full preserve board/target |
| 5.2: schema/versioning | P05/P09 | Known older fixture migrates if supported; unknown major/truncated/checksum-invalid/oversized file rejected with unchanged live board |
| 5.3: imported group | P05/P09/P11 | Same file twice and import into source board produce separate independent objects, fresh IDs, nested groups intact |
| 5.3: origin/ghost placement | P11 | Empty canvas import at origin; later ghost follows pointer; overlap red+unavailable; Escape has no shared effects |
| 5.3: live occupancy | P08/P11 | Delete/move/ungroup frees space; grouping reserves derived bounds; transformed/thick strokes count; touching-edge rule consistent |
| 5.3: concurrent drop | P11/P18 | Both peers drop in the same gap before receiving each other; both imports survive and can be separated |
| 5.3: shortcuts | P08/P16 | Ctrl+G/Ctrl+Shift+G and platform equivalents preserve geometry; text input does not intercept destructive canvas actions incorrectly |
| 5.4: smooth infinite camera | P06/P08/P18 | Cursor-anchored zoom, pan, fit, DPI/window resizing and safe extremes; no fixed page edge or GPU precision collapse in supported range |
| 5.4: background/grid settings | P08/P16 | All four grids, spacing/opacity/color/background work locally; another peer remains unchanged; relaunch resets preferences |
| 5.5: real pressure | P01/P06/P17 | Trace from real tablet on each claimed OS shows nonconstant pressure and responsive width; no duplicate pen/mouse strokes |
| 5.5: simulated pressure | P06/P08 | Recorded mouse traces demonstrate heavier default end, bounded speed response, continuity, and explicit taper precedence |
| 5.5: brush controls/smoothing | P06/P08 | Base size/color/opacity/sensitivity/smoothing/start+end taper exercised; tap/fast/canceled strokes valid; peer geometry identical |
| 5.6: one-question alternatives | P13/P16 | Submit, stream, regenerate alternatives, select, cancel, error, change question; no implicit conversation history |
| Confirmed mascot/card layout | P13/P16/P17 | Selected first concept is used as app icon and AI-card corner avatar; question appears above streamed answer; avatar never obscures content; his confirmed name Dibby is used consistently |
| Confirmed mascot motion | P13/P16/P17 | Idle floats; hover jumps once for 600 ms on entry/focus and retriggers only on re-entry; thinking shows a thought cloud and working shows a laptop; busy switches every three seconds; talking has priority; reduced motion uses neutral frame 0; busy/talking stop with the request/card; native renderer uses the schema-v2 atlas/manifest or explicit rig rather than assuming browser SVG animation support |
| 5.6: local draft/shared pin | P13/P18 | Peers and files never receive local question/drafts; pin creates exactly one selected shared answer, movable/groupable/exportable |
| 5.7: ACP | P03/P15/P18 | Supported real agent initializes/streams through its own harness; negotiated version/cancel/permissions/process exit handled |
| 5.7: ACP containment | P03/P15/P17 | Per-platform malicious-helper tests demonstrate stated isolation; no full-privilege fallback described as containment |
| 5.7: ChatGPT login | P03/P14/P18 | Actual eligible account uses official flow and direct inference; no Codex harness/private endpoint; identity-only/ineligible states honest |
| 5.7: BYOK | P13/P18 | Exactly three fields; authorized real/local endpoint streams; blank local key works; URL/auth/HTTP errors recover |
| 5.7: credential privacy/retention | P03/P04a/P14/P17/P18 | Vault survives restart, board does not; locked vault and forget/sign-out handled; secret sentinel absent from files/peers/logs/artifacts |
| 5.8: minimal harness | P13/P14/P18 | Request inspection contains only typed question/instruction; no tools/context/history; tool-call responses never execute |
| 5.8: no peer-paid inference | P13/P18 | Replayed imports/card changes/room joins trigger zero local provider requests; only explicit local Generate does |
| Optional PNG/SVG | P19 | Independent image/vector viewers match canonical geometry; no drafts/overlays/active SVG/external resources |

### 13.3 Mandatory concurrency and failure regressions

- **Same-object concurrency:** two drags, move versus delete, group versus child move, simultaneous regrouping, missing parent/cycle projection, undo after remote change, late updates after deletion.
- **Synchronization:** edits during snapshot, duplicate chunk/CRDT update, reordered delivery, packet loss, abrupt peer death, malformed frame, transfer timeout, slow consumer, partial mesh, founder exit during newcomer negotiation, partition and reunion.
- **Import/export:** identical source imported repeatedly, source imported into itself, nested groups, empty board, canceled dialog/ghost, peer edit changes placement validity, overlap race, interrupted export, unsupported schema, decompression/resource bomb if compression exists.
- **Input/rendering:** focus lost mid-stroke/drag, pen disconnect, pointer leaving/re-entering, high sample rate, zero pressure/time delta, display/DPI change, minimized window, GPU surface loss, very large content, idle CPU.
- **AI/auth:** canceled login, denied scope, token expiry/revocation, two requests refreshing at once, stream fails after text, malformed or excessive output, deletion/provider switch during stream, retry without accidental duplicate billing, pending ACP permission on shutdown, process-tree escape probe.
- **Privacy:** unique sentinel strings for key/token, question, unchosen answer, pinned answer, deleted pinned answer, room capability, and peer label. Decode/inspect exports, application envelopes, logs, vault records, and build artifacts as applicable; only deliberately pinned visible text belongs in shared board content. Use synthetic values, not real secrets.
- **Persistence:** instrument app-owned filesystem writes under a temporary user profile; allow only explicit exports, approved vault registration/credentials, and ephemeral ACP working directories cleaned on normal shutdown. No board caches, auto-recovery files, recent-file lists, draft/provider-transcript logs, or persisted GUI state. External agent/provider behavior is reported separately.

### 13.4 Proposed performance budgets

These are starting engineering targets to agree/validate against named hardware after P01, not observed results or guarantees. Record CPU, GPU, RAM, OS, display resolution/DPI, build profile, fixture sizes, and transport conditions with every measurement.

| Fixture | Proposed target / measurement |
|---|---|
| Clean launch | Drawable canvas within 2 seconds on the named reference system; no network/provider dependency |
| Local drawing | Pointer-to-visible preview p95 under 30 ms; collect end-to-end evidence rather than infer from loop timing |
| Active viewport | Aim for p95 frame time under 16.7 ms at 1080p on the reference GPU; no sustained stalls while drawing |
| Idle board | Event-driven repaint, low idle CPU; measure rather than forcing a universal percentage |
| Large scene | 10,000 varied strokes with a declared total geometry size; culling/caches remain responsive; memory plateaus after settled edits |
| Late join | 10 MiB synthetic visible board transfers/reconciles within 5 seconds on a measured LAN, without UI freeze |
| Eight peers | 30-minute mixed editing/awareness soak; bounded queues and stable memory; traffic recorded per peer |
| Local link latency | Measure edit-to-remote-visible p50/p95 on LAN and one real internet path; target under 150 ms p95 on suitable LAN conditions |
| Shutdown | Normal shutdown within 3 seconds; bounded forced termination of owned stuck ACP children within documented deadline |

If targets fail, first profile the bottleneck and revisit measurable budgets openly. Never hide a missing feature, drop durable updates, or persist boards to meet a target. Avoid “unlimited objects” or “works on all networks” claims.

### 13.5 Definition of complete v1

All required traceability rows pass on the release commit, every claimed OS has native runtime evidence, pressure and ACP containment claims match actual tests, official ChatGPT access/terms are validated, and remaining optional work is listed explicitly. The latest verified source checkpoint is reproducible from a fresh clone. No required provider path is merely mocked. Icon/branding assets must be approved before a final branded release, while placeholder assets are clearly labeled in development builds.

If an external dependency, unavailable hardware/platform, or eligibility requirement prevents completion, deliver the working checkpoint plus a precise blocker report and next action. That is an honest partial handoff, not completed v1.

## 14. Risk register and escalation boundaries

| Risk | Consequence | Mitigation / decision trigger |
|---|---|---|
| Stable Rust GUI path lacks desktop tablet events | Brush requirement cannot be claimed across OSes | F0 pressure gate; narrow platform adapters; actual hardware tests before support claims |
| Restrictive NAT/firewall | Direct internet join fails | No TURN fallback; honest status/manual retry; document environment limits |
| No approved signaling service | No automatic short-code rendezvous | Keep complete manual exchange; only add real handshake broker with authorization |
| ChatGPT eligibility/terms/protocol drift | Login or inference unavailable | Official-doc/live gate at start and release; explicit blocker, no private-endpoint workaround |
| Persistent auth lifecycle violates broad stateless wording | Misleading privacy behavior | Apply confirmed vault exception only; boards/drafts remain volatile; stable registration metadata accounted for |
| ACP agent escapes temporary cwd | Uncontained filesystem/process access | Verified OS isolation and malicious helper; block unsupported combinations |
| CRDT converges but hierarchy/pose is invalid | Broken groups/rendering or lost user intent | World-space leaf poses, one membership authority, deterministic projection, semantic property tests |
| Reusing imported CRDT identities | Reimport silently merges instead of copying | Isolated decode, fresh IDs, reference remap, repeat-import fixtures |
| Serialized history leaks deleted content | Export carries more than visible shared board | Fresh visible-state export materialization, deleted-sentinel inspection |
| Untrusted files/peer updates exhaust memory | UI freeze/crash | Common hard limits, bounded queues/decoders, fuzzing, helper isolation if needed |
| Slow peer / eight-way fanout | Memory/bandwidth growth | Chunk/backpressure, coalesced presence, state-vector repair, room-level budgets |
| Parallel agents edit shared plumbing | Integration churn or overwritten changes | Ownership, frozen contracts, single Git writer in shared tree, staged integration |
| Cross-platform access/signing unavailable | Unsupported release claims | Build/runtime matrix and named gaps; separate unsigned dev artifacts from signed releases |
| UI interpretation expands to all of Miro | Scope overwhelms core product | Required/proposed tool list; separate change requests for extra object types |
| User expects board recovery from Git checkpoints | Unexported artwork still disappears | Explain source checkpoints versus explicit board export; close warning, no hidden autosave |

Escalate only decisions that genuinely change scope/requirements, require a user-owned account/secret or infrastructure choice, or cannot be resolved from the repository and official documentation. Continue independent work while those are pending. Use the environment's secure credential mechanism for secrets; never ask for passwords/tokens in plain chat. Do not ask the user again to approve the already confirmed OS-vault choice.

## 15. Ready-to-use implementation-orchestrator brief

The user can give the following brief to the implementation model after authorizing the build:

> Build Doodleverse from `docs/PRODUCT_REQUIREMENTS.md` and `IMPLEMENTATION_PLAN.md` in the native Rust workspace at `D:\Apps\projects\doodleverse`, using `https://github.com/asterxsk/doodleverse.git` for incremental commits and verified checkpoints. Re-read current repository state before bootstrap; do not overwrite existing history. Preserve the confirmed rule: remember AI connections only in the OS credential vault; never persist boards or drafts except explicit board export. Start with P00 and the G0 feasibility investigations. Record decisions, blockers, task ownership, exact verification, and checkpoint SHAs in repository files. Freeze shared contracts before broad delegation; use disjoint worker ownership and the orchestrator as integrator/Git writer where the checkout is shared. Follow the dependency waves, integrate real end-to-end behavior, and retain the direct-only/no-TURN networking constraint. Use official ChatGPT direct access only when eligibility/current terms and live behavior are validated; ACP needs actual OS containment. Keep all required features tracked even when externally blocked. Do not call mocks or unit tests proof of runtime completion. Continue through required v1 validation, preserving recoverable checkpoints. Keep long plans/reports in Markdown files and link them in chat. Use `assets/app-icon.png` as the selected app icon and mascot reference. Follow section 16 for Dibby’s AI-card placement and contextual roles; use his user-confirmed name consistently.

## 16. Selected app icon, mascot, and placement

### 16.1 Confirmed identity

The user selected the **first generated concept** (`assets/app-icon.png`) as both the app icon and Doodleverse's mascot. His user-confirmed name is **Dibby**. “Doodle Planet” remains the original concept label, not his name in the product; the retained icon is now `assets/app-icon.png`. The mascot is male in the user's description. Use “Dibby” and “Ask Dibby” where appropriate; do not invent a backstory, voice/personality system, or new character design during implementation.

Retain the core visual identity: warm cream, softly irregular paper body; folded corner; dark friendly eyes/smile; coral-and-teal painted orbit. The full orbit is appropriate for the app icon and larger brand appearances. A simplified face/body crop may be used for a compact avatar if it preserves the same recognizable character; validate it at actual display size. Do not require every app capability to be explained by the mascot alone. Multiplayer remains clearly communicated by actual peer cursors, name/color labels, and room UI. Collaborator-cursor tips on the orbit were discussed as a suggestion but were not selected as a required icon edit.

### 16.2 Required primary placement: canvas AI card

The mascot is the AI buddy's visual identity. Place him at a small, consistent size in a corner of the AI sticky-note/chat card, with the question and answer in a straightforward vertical layout:

1. Small Dibby avatar in the card corner/header, alongside an accessible “Dibby” label and actual connection state where useful.
2. The user's question/input below the header.
3. The AI's streamed answer below the question.
4. Secondary alternatives/regenerate/cancel and select-and-pin controls below or adjacent to the answer without disrupting that reading order.

Keep the question and all draft replies local, exactly as specified in section 8. The selected answer becomes the ordinary shared highlighted/pinned statement when explicitly pinned; do not sync the local question just because it remains visible above the answer. The pinned statement may retain a small decorative mascot corner mark to indicate its AI origin, but must remain readable/editable according to existing object behavior without that decoration and must never trigger provider activity. Reuse a bundled asset for decoration rather than storing image bytes in every object; any shared visual-style choice must be deterministic across peers. Do not change the one-question interaction into a threaded chat.

The mascot represents Doodleverse's AI interface, not a separate hosted model or always-running autonomous agent. He does not watch the board, initiate requests, gain tools, bypass permissions, or create extra inference traffic. Actual provider capabilities, privacy boundaries, credentials, and errors remain unchanged.

### 16.3 Other proposed contextual appearances

These are suggested placements for P16 to assess, not a requirement to put the character on every surface:

| Placement | Purpose | Restraint |
|---|---|---|
| App icon, launcher, About | Establish the selected identity | Use validated platform sizes/masks; avoid duplicating large branding around the canvas |
| AI entry point / empty AI card | Make “Ask Dibby” recognizable before a first question | One small avatar with a real action; no fake connected state |
| AI working/cancel/error state | Give the existing state a friendly visual cue | Use the requested state bundle; alternate thinking/working every three seconds while busy; state text remains explicit and reduced motion is static |
| First-run canvas hint or contextual help | Introduce drawing, sharing, or the AI card | Brief/dismissible/session-only; do not delay the immediate usable canvas |
| Small export confirmation or recoverable connection hint | Reinforce a meaningful success/recovery message | Optional; never cover artwork or replace an actual error explanation |

Do not add a permanent floating assistant, cursor-following character, idle popups, gamification, unsolicited coaching, or persistent mascot preferences. Those would be new features, not consequences of adopting a mascot. Plain UI tool icons and collaborative peer cursors do not need mascot faces.

### 16.4 Prepared asset bundle and native handoff

**Approved baseline:** the final delivered Dibby icon, vector companion, and animations are approved by the user, including the corrected inward-facing laptop. Use the existing files as implementation inputs. The other concept directions are retired; no further naming, icon selection, or mascot redesign is pending.

Only the selected app icon and its mascot/animation assets remain in `assets`; the two unused concept PNGs and concept exploration notes have been removed. The selected original pixels are retained as `assets/app-icon.png`. Editable vector artwork is in `assets/mascot/dibby.svg`; its transparent native animation atlas and authoritative motion metadata are `assets/mascot/dibby-atlas.png` and `assets/mascot/dibby-motion.json`. The schema-version-2 atlas contains 181 row-major 160 px frames in a 12-column by 16-row image (1920 by 2560 px); use its manifest for timing and UVs. Browser-animated exports are `dibby-idle.svg`, `dibby-hover.svg`, `dibby-talking.svg`, `dibby-thinking.svg`, `dibby-working.svg`, and `dibby-busy.svg` in that same mascot directory. Integration instructions are in `docs/DIBBY_ASSETS.md`, and `docs/dibby-preview.html` provides an interactive local preview. Development generation tools are `tools/dibby-rig.js` and `tools/build_dibby_assets.py`; these are not shipped-app runtime dependencies.

The SVG is a hand-authored vector recreation of the chosen paper character and orbit, not a pixel-identical recreation of generated paper/brush texture. Keep the original PNG as the app icon. The user authorized five animated states: idle gentle float, hover one-shot jump, talking, thinking with a visible thought cloud, and working with a visible laptop. The working laptop sits on Dibby's right in a three-quarter perspective, with its screen angled toward him and keyboard toward his hands, as explicitly requested by the user; do not place a viewer-facing laptop across his torso. The separate neutral pose is static; the master SVG includes hidden thought-cloud/laptop vector parts. No app code or provider functionality has been implemented by creating this asset bundle.

### 16.5 Confirmed motion behavior

| State | Approved presentation | Playback |
|---|---|---|
| Neutral | Static Dibby, no cloud or laptop | Reduced motion or deliberately static use; atlas frame 0 |
| Idle | The gentle floating motion originally shown as hover | 2,400 ms loop while visible |
| Hover | One little jump, landing back in place | 600 ms once per pointer/focus entry; no repeated jumping while hovered |
| Talking | Mouth movement and small conversational bob | 1,200 ms loop when the UI is presenting streamed text; no voice feature implied |
| Thinking | Thought cloud above Dibby, with his thinking expression | 3,000 ms phase of the busy cycle |
| Working | Laptop on his right, screen angled inward toward him, keyboard toward his hands; gaze down/right | 3,000 ms phase of the busy cycle |

While an AI request is busy, start with thinking and its visible thought cloud, switch to working and its visible laptop after 3,000 ms, then alternate every 3,000 ms until the request ends. This is a visual cycle only: the app does not need to infer model reasoning/tool phases, and switching the animation never starts another provider request. The cloud/laptop are expressive props, not actual reasoning, files, tool activity, or permission state. Keep real status labels truthful; a working pose does not mean a text-only provider has file/terminal access. Talking can represent streamed answer text and does not imply a voice feature.

Presentation priority is reduced-motion neutral frame 0, then talking, then an active hover/focus-entry jump, then the current busy thinking/working phase, then animated idle. Hover is a 600 ms one-shot jump: record pointer/focus entry, advance `hoverElapsedMs`, clamp its final frame instead of modulo-looping, and return to busy/idle after 600 ms even while still hovered. Leaving and entering/focusing again retriggers it. The resolver uses `hovered` and `hoverElapsedMs` (default 0), so passing a permanently true flag without elapsed time is not sufficient. Hover/talking overrides do not reset the busy clock. Complete/error/cancel/card deletion/provider change/shutdown ends relevant busy/talking state and unused redraw work; visible idle may continue floating. A new request starts a fresh thinking phase. Keep animation local and session-only; do not write state changes into the CRDT or board files.

Native Rust SVG renderers must not be assumed to execute CSS/SMIL or browser DOM animation. Use the supplied frame atlas and JSON metadata with the existing native texture API, or implement the named vector-part transforms explicitly. Load resources once, schedule only necessary visible repaints (including animated idle), and use neutral frame 0 without animated props for reduced motion while retaining truthful separate status text. The browser preview is an asset demonstration, not a native-app dependency or native runtime verification.

Before production packaging, validate the selected silhouette and contrast at small sizes, light/dark surroundings, and Windows/macOS/Linux masks. Proper ICO/ICNS/platform icon exports remain packaging work. Preserve the selected original PNG, use Dibby consistently in UI/accessibility/documentation, and do not add new states, character concepts, persistent mascot preferences, or an autonomous floating assistant as part of this handoff.

### 16.6 Integration acceptance for P13/P16/P17

- Reuse the approved bundle; load the atlas and schema-v2 manifest together. Do not use old frame offsets or expect the static master SVG to animate automatically.
- Keep the card order Dibby in the corner/header, then the question, then the answer and secondary controls. Preserve the existing local-draft/shared-selected-answer boundary.
- For native vector integration, use the corrected laptop pivot `[1050, 990]` from `rig.part_pivots.laptop`, not the character pivot `[630, 640]`. Preserve the laptop's inward-facing perspective and working gaze.
- Exercise idle, one-shot hover/re-entry, talking, the three-second busy alternation, priority overrides, and reduced motion in the actual Rust UI. Stop state-driven activity on completion, cancellation, card deletion, provider replacement, and shutdown.
- Existing asset checks are reproducible with `python tools/check_dibby_assets.py`; regeneration uses `python tools/build_dibby_assets.py`. These development tools and their Python/Chrome dependencies are not shipped-app requirements.
- Asset validation has passed, including the corrected laptop: original icon hash preserved; all seven SVGs parsed; 181 transparent unclipped atlas frames checked; state/prop visibility, busy timing, hover completion/retrigger, reduced motion, and preview controls verified. Independent visual review accepted the corrected orientation. Native app integration and platform packaging still require their own evidence.

## 17. Planning verification record

- Read the full supplied PRD and mapped every numbered feature section to required work/acceptance criteria.
- Confirmed the user's credential-vault preference and kept board/draft persistence prohibited.
- Inspected the empty local directory and empty reachable GitHub repository; no repository writes were performed.
- Consulted official Rust GUI/Cargo/WebRTC/CRDT/ACP/OpenAI sources linked beside the relevant claims. Provider policies and crate versions are time-sensitive and must be rechecked during implementation.
- Used independent planning reviews for CRDT/networking, AI feasibility, and requirements/parallel-work risks. These are planning reviews, not implementation tests.
- Final review corrected missing production-platform ownership and premature checkpoint/dependency timing. Subsequent AI and CRDT/network/file reviews found no material contradictions within their reviewed scope.
- Static document checks passed: balanced code fences, unique work-package IDs, no undefined task references, all PRD feature sections 5.1–5.8 represented, and no unresolved placeholder markers. These checks do not validate future application behavior.
- The initial planning phase created only this plan file. Subsequent user-authorized asset work prepared Dibby's selected icon, editable SVG, animation exports/atlas, preview, generation/check tools, and integration notes; the final design and corrected laptop pose are user-approved.
- The asset checker and visual reviews passed for the final bundle. No native application build/tests, real provider login, sandbox checks, infrastructure deployments, commits, or pushes are claimed.
