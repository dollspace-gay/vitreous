---
title: "Implement vitreous_hot_reload and vitreous-cli"
tags: [design-doc]
sources: []
contributors: [unknown]
created: 2026-03-21
updated: 2026-03-21
---


## Design Specification

### Summary

Implement the hot reload system (file watcher + WebSocket protocol + runtime patch client) that enables sub-second style and layout updates without recompilation, and the CLI tool (`vitreous-cli`) that provides `vitreous new` for project scaffolding and `vitreous dev` for starting the hot-reload development server.

### Requirements

- REQ-1: File watcher using notify 8 monitors `.rs` source files for changes
- REQ-2: On file change: parse changed file, extract widget tree descriptions (style/layout changes), serialize as a diff message
- REQ-3: WebSocket server (tokio-tungstenite 0.29) sends diff messages to connected runtime clients
- REQ-4: Runtime client receives diff messages and patches the live widget tree without restarting the application
- REQ-5: Style changes (colors, spacing, fonts, etc.) applied without recompilation, visible within 200ms
- REQ-6: Layout changes (flex direction, alignment, padding, etc.) applied without recompilation, visible within 500ms
- REQ-7: Logic changes (signal creation, event handlers, new components) require full recompilation — the system detects this and triggers `cargo build` automatically
- REQ-8: `vitreous new <name>` scaffolds a new project with Cargo.toml, src/main.rs (counter template), and optional web target setup
- REQ-9: `vitreous dev` starts the hot-reload server, watches for changes, and manages the build/reload cycle
- REQ-10: Change message protocol defined with serde-serializable types for style patches, layout patches, and full-rebuild notifications

### Acceptance Criteria

- [ ] AC-1: File watcher detects `.rs` file save within 100ms (REQ-1)
- [ ] AC-2: Style-only change (e.g., changing a color constant) produces a StylePatch message, not a full rebuild (REQ-2, REQ-10)
- [ ] AC-3: Running app receives StylePatch and updates visuals within 200ms of file save (REQ-4, REQ-5)
- [ ] AC-4: Layout change (e.g., changing padding value) updates the running app within 500ms (REQ-6)
- [ ] AC-5: Adding a new `create_signal()` call triggers a full recompile notification (REQ-7)
- [ ] AC-6: WebSocket reconnects automatically if the server restarts (REQ-3)
- [ ] AC-7: `vitreous new my-app` creates `my-app/Cargo.toml` and `my-app/src/main.rs` with a working counter app (REQ-8)
- [ ] AC-8: `vitreous new my-app` project compiles and runs with `cargo run` immediately (REQ-8)
- [ ] AC-9: `vitreous dev` starts watcher + WebSocket server and prints connection URL (REQ-9)
- [ ] AC-10: Multiple connected clients all receive the same patch simultaneously (REQ-3)

### Architecture

### File Structure

```
crates/vitreous_hot_reload/src/
├── lib.rs          # Re-exports, feature flag for "server" vs "client" mode
├── server.rs       # FileWatcher + WebSocket server (behind "server" feature)
├── client.rs       # Runtime patch receiver (always compiled into debug builds)
└── protocol.rs     # Serde-serializable message types: StylePatch, LayoutPatch, FullRebuild

tools/vitreous-cli/
├── Cargo.toml      # depends on vitreous_hot_reload (server feature), clap for CLI
└── src/
    └── main.rs     # CLI entry point: new, dev subcommands
```

### Dependencies

- `notify` (8) — file system watching
- `tokio-tungstenite` (0.29) — WebSocket server/client
- `serde` (1) + `serde_json` — message serialization
- `syn` + `quote` — Rust source parsing for change classification (server only)
- `clap` — CLI argument parsing (vitreous-cli only)

### Change Classification

When a `.rs` file changes, the server:

1. Parses the file with `syn` to extract the AST
2. Compares against the previous AST for the same file
3. Classifies changes:
   - **Style-only**: Changes to numeric literals, color constructors, or string literals inside modifier chains (`.padding(X)`, `.background(Color::hex("X"))`) → emit `StylePatch`
   - **Layout-only**: Changes to layout modifiers (`.width()`, `.flex_grow()`, `.gap()`) → emit `LayoutPatch`
   - **Logic**: Changes to signal creation, effect bodies, event handler closures, new/removed functions → emit `FullRebuild`

This is a heuristic — conservative classification errs toward FullRebuild.

### Protocol Messages

```rust
#[derive(Serialize, Deserialize)]
pub enum HotReloadMessage {
    StylePatch {
        file: String,
        patches: Vec<PropertyPatch>,
    },
    LayoutPatch {
        file: String,
        patches: Vec<PropertyPatch>,
    },
    FullRebuild {
        reason: String,
    },
    Connected {
        server_version: String,
    },
}

#[derive(Serialize, Deserialize)]
pub struct PropertyPatch {
    pub node_path: Vec<String>,  // Path from root to target node
    pub property: String,         // "background", "padding", "font_size", etc.
    pub value: PatchValue,        // Serialized new value
}
```

### Runtime Client

In debug builds, the `vitreous` facade crate spawns a background task that:
1. Connects to WebSocket at `ws://localhost:3742` (configurable)
2. On `StylePatch`/`LayoutPatch`: walks the live node tree, finds matching nodes, updates properties, triggers re-render
3. On `FullRebuild`: prints "Recompiling..." and waits for the CLI to rebuild and restart the app
4. On disconnect: attempts reconnection every 2 seconds

The client is compiled out entirely in release builds via `#[cfg(debug_assertions)]`.

### CLI Subcommands

**`vitreous new <name> [--web]`**:
- Creates directory with `Cargo.toml` depending on `vitreous`
- Generates `src/main.rs` with counter example
- If `--web`: adds `wasm-pack` config, creates `index.html`

**`vitreous dev [--port PORT]`**:
- Starts file watcher on `src/`
- Starts WebSocket server on specified port (default 3742)
- Runs `cargo build` initially
- On file change: classify, send patch or trigger rebuild
- Streams build output to terminal

### Out of Scope

- Hot reload of proc macro output
- Hot reload across network (only localhost)
- Browser auto-refresh for web target (use existing tools like trunk or wasm-pack)
- IDE plugin for hot reload integration
- Hot reload of asset files (images, fonts) — only `.rs` source

