# Changelog

All notable changes to the **Sherlock Launcher** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),

1. MAJOR version when you make incompatible API changes
2. MINOR version when you add functionality in a backward compatible manner
3. PATCH version when you make backward compatible bug fixes

Additional labels for pre-release and build metadata are available as extensions to the MAJOR.MINOR.PATCH format.

---

## [0.2.0-dev] - 2026-03-26

### 🚀 Added

- **Emoji Picker System**: A complete emoji subsystem featuring skin tone support (`default_skin_tone`), Title Case formatting, and a dedicated navigation stack for category switching.
- **Clipboard Lanucher**: Integrated a new `ClipboardLauncher` with intent-based color coding and background system polling.
- **Inner Functions & Keybinds**: Added the ability for launcher variants (like Audio/MPRIS) to define internal functions that can be triggered via direct keybindings.
- **Config Watcher**: Implemented a hot-reload system that monitors configuration files and updates the application state in real-time without a restart.
- **Deployment Tooling**: Added `PKGBUILD` for Arch Linux support, a `packager.sh` script, and GitHub Workflows for automated releases and issue labeling.

### 🔧 Changed

- **Modular Architecture**: Executed a major refactor of `src/launcher/mod.rs`, splitting monolithic logic into specialized files: `app_launcher`, `calc_launcher`, `bookmark_launcher`, and others.
- **App Module Migration**: Refactored `main.rs` into a structured `app` module, isolating `bindings`, `updates`, and core state management for better testability.
- **Icon Rendering Pipeline**: Moved icon loading into a dedicated module with improved SVG rendering and a theme-aware caching layer.
- **Enhanced Error Views**: Overhauled the error reporting UI to be scrollable and interactive, allowing users to dismiss individual errors.
- **Async Data Flow**: Decoupled async updates for launcher tiles; metadata (like Weather and MPRIS) now populates the UI reactively as soon as it is available.

### 🐞 Fixed

- **Emoji Loading**: Optimized the initial state and deserialization of the emoji picker to prevent UI stutters during large data loads.
- **Context Menu Alignment**: Resolved several rendering issues regarding context menu widths and icon positioning on the command launcher.
- **App Deserialization**: Fixed a crash caused by incorrect `ContextMenuAction` serialization in `app_data.rs`.

### 🗑️ Removed

- **Debug Noise**: Stripped out numerous `println!` statements and "debug-only" migration paths across the codebase.
- **Dead Code**: Removed the deprecated `roadmap-tasks.md` and pruned unused imports using `cargo fix`.
- **Monolithic Main**: Removed 400+ lines from `main.rs` in favor of the new modular `app` and `loader` structures.

---
