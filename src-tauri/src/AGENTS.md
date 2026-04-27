<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-04-12 | Updated: 2026-04-12 -->

# src (Rust)

## Purpose

Rust source code for the Tauri backend. Contains the application entry point and Tauri app builder configuration.

## Key Files

| File | Description |
|------|-------------|
| `main.rs` | Application entry point — calls `glimmerx_lib::run()` and sets `windows_subsystem` for release builds |
| `lib.rs` | Tauri app setup — `Builder::default()` with `tauri_plugin_opener` plugin |

## For AI Agents

### Working In This Directory

- `main.rs` is minimal — do not add logic here, use `lib.rs` or new modules
- `lib.rs` is where `tauri::Builder` chains plugins and commands
- New modules should be organized by domain: `db/`, `models/`, `commands/`, `utils/`
- Library name is `glimmerx_lib` — referenced from `main.rs`
- Crate types: `staticlib`, `cdylib`, `rlib` — required for Tauri

<!-- MANUAL: Any manually added notes below this line are preserved on regeneration -->
