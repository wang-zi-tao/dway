# AGENTS.md — dway

A Wayland compositor built on the Bevy 0.17 ECS engine.

## Knowledge base hierarchy

```
./AGENTS.md                          (this file — project-wide conventions)
├── dway-ui-framework/AGENTS.md      (custom UI framework + shader pipeline)
├── dway-server/AGENTS.md            (Wayland protocol implementation)
├── dway-client-core/AGENTS.md       (desktop logic, workspaces, DBus)
├── dway-tty/AGENTS.md               (DRM/KMS output, libseat, EGL)
├── dway-ui/AGENTS.md                (compositor shell widgets)
├── dway-ui-derive/AGENTS.md         (proc-macros for widget DSL)
├── bevy-relationship/AGENTS.md      (graph-style ECS queries)
└── dway-util/AGENTS.md              (shared utilities)
```

Crates without their own AGENTS.md:
- `bevy-relationship-derive/` — covered by `bevy-relationship/AGENTS.md`
- `dway/` — thin top-level binary; see this file

## Build & run

```bash
# Enter nix dev shell (required for all system deps: mesa, wayland, vulkan, etc.)
direnv allow   # or `nix develop`

# Standard build
cargo build -p dway

# Run
cargo run -p dway

# Build a single crate (faster than workspace build)
cargo build -p dway-server
cargo build -p dway-ui-framework

# Lint / format
cargo fmt --all
cargo clippy --workspace --all-targets
```

## Architecture

```
dway (top-level binary + plugin wiring)
 ├── dway-tty          — GPU/screen output: raw DRM/KMS, libseat, EGL
 ├── dway-server       — Wayland protocol implementation (wayland-rs)
 ├── dway-client-core  — Desktop logic: workspaces, layouts, app management, DBus
 ├── dway-ui           — Widgets for the compositor shell UI
 ├── dway-ui-framework — Custom reactive GUI framework (shape rendering, composable shaders)
 ├── dway-ui-derive    — Proc-macros for `dway_widget!` DSL
 ├── dway-util         — Shared utilities: logging, diagnostics, event loop
 ├── bevy-relationship — Graph-style ECS entity queries (`relationship!`, `graph_query!`)
 └── bevy-relationship-derive — Proc-macros for bevy-relationship
```

**Dependency order (build from leaf up):**
`dway-util` → `bevy-relationship-derive` → `bevy-relationship` → `dway-server` / `dway-tty` → `dway-client-core` → `dway-ui-derive` → `dway-ui-framework` → `dway-ui` → `dway`

## Critical environment details

- **Nix mandatory**: All system deps (mesa, vulkan-loader, wayland, libinput, etc.) come from the flake. Do NOT install system packages manually.
- **Nightly Rust**: `lib.rs` uses `#![feature(stmt_expr_attributes)]` — the flake's fenix toolchain includes nightly.
- **Vulkan-only**: `WgpuSettings { backends: Some(Backends::VULKAN), ... }` — OpenGL/WebGPU backends are not used.
- **TTY vs windowed**: At startup, `DISPLAY` and `WAYLAND_DISPLAY` env vars are checked. If both are unset → TTY mode (raw DRM output). Otherwise → winit window.
- **Default asset path**: Custom `AssetPlugin { file_path: opts.assets, ... }` — provide assets via CLI flag `--assets`.

## Feature flags (on `dway` crate)

| Feature | Effect |
|---|---|
| `develop` | Enables egui inspector, reflect auto-register |
| `debug` | `develop` + `dump_system_graph` + bevy `track_location` |
| `debug_render` | RenderDoc capture support |
| `single_thread` | All schedules single-threaded |
| `trace_tracy` / `trace_chrome` | CPU profiling (disables file-based logging) |
| `pprof` / `dhat-heap` | pprof flamegraph / DHAT heap profiling |
| `hot_reload` | Dexterous developer hot-reload (currently commented out) |

## Test & verify

```bash
# Run tests for one crate (some require GPU/Wayland)
cargo test -p dway-ui-framework
cargo test -p bevy-relationship

# Integration test: shader rendering (dway-ui-framework)
cargo test -p dway-ui-framework --test shader

# Benchmarks (bevy-relationship)
cargo bench -p bevy-relationship
```

**Test caveats**: Many crates have no tests. The `dway-ui-framework` shader test renders actual GPU output and compares screenshots — needs a display/headless GPU.

## Code conventions

- **rustfmt**: `group_imports = "StdExternalCrate"`, `merge_imports = true`, `reorder_impl_items = true`
- **clippy**: `too-many-arguments-threshold = 16`, `type-complexity-threshold = 512`
- **Profiles**: `dev.opt-level = 1`, all dependency crates `opt-level = 3` in dev (compiles deps with full optimizations)
- **Release**: `lto = "thin"`, `strip = true`
- **Hot reload** (Dexterous): Only `dway-ui/` is watched. Hot reload features are commented out — to re-enable, uncomment `hot_reload` in Cargo.toml and `Dexterous.toml`.

## Important design notes

- **Custom UI framework**: `dway-ui-framework` is NOT bevy_ui. It has its own layout engine (`dway_widget!` macro DSL), shape rendering pipeline, and state management (`@use_state`). Do not confuse with bevy's built-in UI.
- **Graph queries**: `bevy-relationship` provides `relationship!` for typed edges between entities, and `graph_query!` / `graph_query2!` for multi-hop graph traversals within ECS queries. This is the compositor's primary data model for window/surface/client relationships.
- **Server plugins are monolithic**: `dway_server`, `dway_client_core`, `dway_ui` are added as single plugins (`DWayServerPlugin`, etc.) in `init_app()`. Changes that affect plugin ordering must edit `lib.rs` directly.
- **No CI workflows**: No `.github/workflows` present. All verification is manual/local.
- **Documentation**: mdBook project in `docs/` (build with `mdbook build`). Uses PlantUML for diagrams.
