<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-04-12 | Updated: 2026-04-12 -->

# icons

## Purpose

Application icons in multiple sizes and formats for cross-platform bundling (Windows, macOS, Linux).

## Key Files

| File | Description |
|------|-------------|
| `icon.ico` | Windows icon (multi-resolution) |
| `icon.icns` | macOS icon |
| `icon.png` | Generic PNG icon |
| `32x32.png`, `128x128.png`, `128x128@2x.png` | Various sized PNG icons for desktop environments |
| `Square*.png` | Windows Store logo sizes |
| `StoreLogo.png` | Microsoft Store listing icon |

## For AI Agents

### Working In This Directory

- These files are used by Tauri's bundler — do not modify unless replacing the app icon
- Icon regeneration: use `tauri icon` command to generate all sizes from a single source image
- Tauri expects all these sizes to exist — removing files will break the build

<!-- MANUAL: Any manually added notes below this line are preserved on regeneration -->
