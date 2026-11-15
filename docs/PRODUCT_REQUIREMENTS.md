# Doodleverse: Product Requirements Document

## 1. Overview

Doodleverse is a native, cross-platform, peer-to-peer multiplayer whiteboard written in Rust. It combines Miro's infinite canvas and collaboration model with Excalidraw's hand-drawn feel and brush behavior. There is no backend, no accounts and no cloud storage. The app is fully stateless: nothing persists inside it between sessions. Boards live only in memory during a live session and move in and out through file import and export.

**Name rationale:** A playful take on the whiteboard: doodling together in a shared universe of boards. The tone is fun and casual rather than corporate.

## 2. Goals

- Real-time multiplayer whiteboarding with friends, with no hosted server dependency for data.
- Stateless by design: no saved sessions, no internal library, no database.
- One native Rust codebase running on Windows, macOS and Linux.
- Import and export of boards as portable files using the OS file dialog.
- A drawing feel close to a real pen, with customizable canvas appearance.

## 3. Non-Goals

- No user accounts, authentication or profiles.
- No server-side persistence, history or cloud sync.
- No web or mobile targets in v1.
- No built-in chat, voice or video in v1.

* No app-hosted AI, no resold tokens and no shared AI credentials between peers in v1.

## 4. Architecture

- **Language and UI:** Rust, using the newest stable crates. A GPU-accelerated immediate-mode or retained UI such as egui on wgpu, or Iced, with a custom canvas renderer for the infinite board.
- **Shared state:** CRDT document using yrs, the Rust port of Yjs. All board objects live in one shared document so edits from any peer merge without conflicts.
- **Networking:** WebRTC data channels via a Rust WebRTC crate for direct peer-to-peer links between instances.
- **Signaling:** Peers only need a brief handshake to find each other. Support a shareable room link or code that carries the connection offer, with an optional public relay for the handshake only. After the handshake, all traffic is direct and no board data ever touches a server.
- **Topology:** Full mesh for small rooms, targeting up to about eight peers.
- **Files:** Native OS file dialogs through a cross-platform crate such as rfd.

* AI layer: a provider adapter trait with ACP, ChatGPT login and bring-your-own-key implementations, running off the UI thread and streaming text back to the card. AI traffic goes from the user's machine to their provider and never through peers.

## 5. Core Features

### 5.1 Rooms and Sessions

- Launching the app opens an empty infinite canvas immediately.
- A Share action generates an invite link or code. Friends paste it to join.
- Every peer sees live cursors with a name and color label.
- The room exists only while at least one peer is connected. When the last peer leaves, the board is gone unless someone exported it.
- Late joiners receive the full current document state from an existing peer on connect.

### 5.2 Import and Export

- **Export:** Saves the current board as a single portable file containing the serialized CRDT snapshot, with a version header for forward compatibility. Optional additional exports to PNG and SVG.
- **Import:** Opens a file through the OS dialog and applies it to the live shared document, so the board is merged for every peer in the room.
- Works identically on every supported OS.

### 5.3 Import Placement and Merging

- An imported board arrives as a single **group**, never auto-flattened.
- The first board imported into an empty canvas is placed at the origin.
- Any later import enters a placement mode: the board appears as a ghost preview attached to the cursor.
- The ghost turns red with an unavailable cursor whenever its bounding box overlaps an already occupied region. The user clicks to drop it in a free area.
- Occupied regions are computed live from current objects, not from stored import boxes, so ungrouping or deleting frees space again.
- If two peers drop at the same moment and overlap, nothing breaks. The CRDT merges both and the user can drag them apart.
- Pressing Ctrl+Shift+G ungroups an imported board into free objects, matching Figma and Excalidraw conventions. Ctrl+G regroups a selection.

### 5.4 Canvas and Background

- Infinite pan and zoom with smooth GPU rendering.
- Selectable grid styles: dot grid, plus grid, line grid, or none.
- Adjustable grid spacing, opacity, color, dot/plus size and line width.
- Fully customizable background color with a color picker and a few presets, including light and dark themes.
- Canvas appearance settings are local to each user and are not synced to peers.
- A floating Excalidraw-style toolbox: tool, ink and brush controls live in islands over the canvas
  instead of a fixed title bar. The UI can be hidden for a clean canvas with Excalidraw's Alt+Z
  zen-mode shortcut and restored with the same shortcut.

### 5.5 Pressure-Sensitive Brush

- Freehand brush that behaves like a real pen. Stroke width varies along the path based on input.
- With a stylus or tablet, width follows real pressure data taken from the platform's native pen
  or pointer events, not a simulated curve.
- With a mouse, simulate pressure from stroke velocity and progression, so a stroke builds up and ends heavier than it starts, with more ink toward the end than at the beginning.
- Strokes are smoothed using curve fitting so they look clean and hand-drawn.
- Brush settings: base size, color, opacity, pressure sensitivity amount, smoothing level and taper style for the start and end of strokes.

### 5.6 AI Chat Card

- A new canvas object, like a sticky note, that the user drops anywhere on the board. It holds a single question box.
- Submitting a question sends it to the connected AI. The replies flow downward inside the card as they stream in.
- The interaction is deliberately one question at a time, with no long threaded conversation.
- The user can generate several replies and select the one they like best. The selected reply becomes the card's final statement and stays pinned on the board.
- The pinned result is a normal synced board object, so every peer in the room sees it. Unpinned drafts and the AI connection itself stay local to the user who asked.
- Nothing is stored by the app. The pinned answer persists only through the board export file.

### 5.7 AI Connections

The user connects an AI provider once in settings. Credentials and sessions live only on that user's machine and are never synced to peers or written into board files. There are three connection options.

1. **Agent subscription via ACP.** Connect any local coding agent the user already has installed that supports the Agent Client Protocol, such as Claude, GitHub Copilot or others, the same approach T3 Code uses. Doodleverse launches the chosen agent as a local subprocess and talks to it over stdio using the official agent-client-protocol Rust crate. The agent runs inside its own native harness, with its own tools and behavior, contained by Doodleverse. The user keeps their own plan and no tokens are resold. Prefer the current protocol version and negotiate down to v1 for agents that have not migrated.
2. **Log in with ChatGPT.** Sign in with a ChatGPT account so the user can use their ChatGPT plan models, with ChatGPT acting purely as the model provider. It does not use the official Codex harness or any agent runtime. Requests go through Doodleverse's in-app minimal input-output harness only. Use OpenAI's official Sign in with ChatGPT flow, and verify the current terms and plan eligibility before shipping.
3. **Bring your own key.** Three fields only: base URL, API key and model name. It targets any OpenAI-compatible chat completions endpoint, which also covers local models, and runs through the in-app minimal harness, the same one used by ChatGPT sign-in.

### 5.8 Minimal Brainstorm Harness

- The in-app minimal harness is used by ChatGPT sign-in and bring-your-own-key connections. It makes the AI a plain text-in, text-out brainstorming partner, not a coding agent.
- Input is the question text, plus an optional short system prompt tuned for brainstorming. Output is streamed text.
- The in-app harness has no agent features: no file access, no terminal and no tool calls. ACP connections do not use it. They run inside the connected agent's own harness, contained by Doodleverse, with permission requests surfaced to the user and a throwaway working directory.
- Two harness paths sit behind one interface the card talks to. ChatGPT sign-in and bring-your-own-key use the in-app minimal harness, a single small adapter that takes a prompt and streams back text chunks. ACP connections use the connected agent's own harness. The card does not care which path is active.
