---
title: "Create workspace scaffold with all crates and correct dependency wiring"
tags: [design-doc]
sources: []
contributors: [unknown]
created: 2026-03-21
updated: 2026-03-21
---


## Design Specification

### Summary

Bootstrap the full Cargo workspace with all 10 library crates, 1 facade crate, 1 CLI tool, and 4 example crates. Every `Cargo.toml` has correct dependencies, every `lib.rs` has module declarations and public re-exports. The workspace compiles to empty libraries. This is the foundation all other phases build on.

### Requirements

- REQ-1: Workspace root `Cargo.toml` declares all members under `crates/`, `examples/`, and `tools/`
- REQ-2: Each library crate has a `Cargo.toml` with correct inter-crate and external dependencies matching the dependency graph in `vitreous-architecture.md`
- REQ-3: Each `lib.rs` declares its module structure with `pub mod` statements and placeholder re-exports
- REQ-4: Foundation crates (`vitreous_reactive`, `vitreous_style`, `vitreous_events`) have zero workspace-internal dependencies
- REQ-5: The facade crate `vitreous` re-exports all public APIs and uses `cfg(target_arch)` to gate desktop vs web backends
- REQ-6: All external dependencies pinned to latest stable versions: taffy 0.9, wgpu 29, winit 0.30, cosmic-text 0.18, accesskit 0.24, accesskit_winit 0.32, rfd 0.17, arboard 3.6, wasm-bindgen 0.2, web-sys 0.3, js-sys 0.3
- REQ-7: Workspace uses Rust edition 2024
- REQ-8: Example crates depend only on the `vitreous` facade crate

### Acceptance Criteria

- [ ] AC-1: `cargo check --workspace` exits 0 (REQ-1, REQ-2, REQ-3)
- [ ] AC-2: `cargo metadata --format-version=1 | jq '.packages[] | select(.name == "vitreous_reactive") | .dependencies | length'` returns 0 (REQ-4)
- [ ] AC-3: `cargo metadata --format-version=1 | jq '.packages[] | select(.name == "vitreous_style") | .dependencies | length'` returns 0 (REQ-4)
- [ ] AC-4: `cargo metadata --format-version=1 | jq '.packages[] | select(.name == "vitreous_events") | .dependencies | length'` returns 0 (REQ-4)
- [ ] AC-5: `vitreous` facade crate has `#[cfg(not(target_arch = "wasm32"))]` gate on `vitreous_platform` dep and `#[cfg(target_arch = "wasm32")]` on `vitreous_web` (REQ-5)
- [ ] AC-6: All `Cargo.toml` files specify `edition = "2024"` (REQ-7)
- [ ] AC-7: Each example's `Cargo.toml` lists only `vitreous` as a dependency (REQ-8)
- [ ] AC-8: Directory structure matches the crate map in `vitreous-architecture.md` exactly

### Architecture

### Directory Structure

```
vitreous/
├── Cargo.toml                     # [workspace] members = [...]
├── crates/
│   ├── vitreous/                  # Facade: src/lib.rs
│   ├── vitreous_reactive/         # src/{lib,signal,memo,effect,resource,context,runtime,batch,graph,scope}.rs
│   ├── vitreous_widgets/          # src/{lib,node,primitives,containers,control_flow,virtual_list,callback,into_nodes,router}.rs
│   ├── vitreous_style/            # src/{lib,color,theme,style,dimension,animation,font}.rs
│   ├── vitreous_layout/           # src/{lib,tree,compute,boundary}.rs
│   ├── vitreous_a11y/             # src/{lib,tree,focus,keyboard,roles,warnings}.rs
│   ├── vitreous_events/           # src/{lib,types,propagation,hit_test}.rs
│   ├── vitreous_render/           # src/{lib,commands,pipeline,atlas,damage,diff}.rs + src/shaders/{rect,text,image,shadow}.wgsl
│   ├── vitreous_platform/         # src/{lib,window,text_engine,dialogs,clipboard,event_loop,system_info}.rs
│   ├── vitreous_web/              # src/{lib,dom,styles,aria,events,mount,web_apis}.rs
│   └── vitreous_hot_reload/       # src/{lib,server,client,protocol}.rs
├── examples/
│   ├── counter/src/main.rs
│   ├── todo/src/main.rs
│   ├── dashboard/src/main.rs
│   └── file_explorer/src/main.rs
└── tools/
    └── vitreous-cli/src/main.rs
```

### Key Dependency Wiring

Foundation crates (no internal deps):
- `vitreous_reactive/Cargo.toml` — no `[dependencies]` from workspace
- `vitreous_style/Cargo.toml` — no `[dependencies]` from workspace
- `vitreous_events/Cargo.toml` — no `[dependencies]` from workspace

Mid-tier crates:
- `vitreous_layout` depends on `taffy = "0.9"`
- `vitreous_a11y` depends on `vitreous_events = { path = "../vitreous_events" }`, `accesskit = "0.24"`

Widget crate:
- `vitreous_widgets` depends on `vitreous_reactive`, `vitreous_style`, `vitreous_events`, `vitreous_a11y`

Backend crates:
- `vitreous_render` depends on `vitreous_layout`, `wgpu = "29"`, `cosmic-text = "0.18"`
- `vitreous_web` depends on `vitreous_reactive`, `vitreous_widgets`, `vitreous_layout`, `vitreous_a11y`, `wasm-bindgen = "0.2"`, `web-sys = "0.3"`
- `vitreous_platform` depends on `vitreous_render`, `vitreous_a11y`, `winit = "0.30"`, `rfd = "0.17"`, `arboard = "3.6"`, `cosmic-text = "0.18"`, `accesskit_winit = "0.32"`

Each `lib.rs` contains only module declarations and `pub use` re-exports — no logic. Example main.rs files contain `fn main() {}` stubs.

### Out of Scope

- Any implementation logic — only empty module files with correct structure
- Tests beyond `cargo check`
- CI/CD configuration
- `rust-toolchain.toml` (defer to Phase 6 when MSRV is validated)

