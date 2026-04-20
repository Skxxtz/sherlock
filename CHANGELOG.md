# Changelog

All notable changes to the **Sherlock Launcher** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),

1. MAJOR version when you make incompatible API changes
2. MINOR version when you add functionality in a backward compatible manner
3. PATCH version when you make backward compatible bug fixes

Additional labels for pre-release and build metadata are available as extensions to the MAJOR.MINOR.PATCH format.

---

## [0.2.0-dev] - 20.04.26

### Added

- **Translator:** Added new translation functionality.
- **Launchers:**
  - Implemented **Script Launcher** with "wait for return" support.
  - Implemented **Events Launcher** with `look_ahead` and `look_back` parameter parsing.
  - Added **Web Launcher** application action type.
- **Emoji Picker:** Added foundational picker, including context menus and default skin tone support.
- **File Search:** Implemented basic file search supporting `ripgrep` and `walkdir` backends.
- **Clipboard:** Added clipboard listener and functionality.
- **Integration:** Added support for Zoom meetings and XDG-settings mimetype handling.

### Improvements

- **UI/UX:**
  - Added visual shortcuts.
  - Improved design for script, event, and file tiles.
  - Added animations and improved transition handling.
  - Improved error views (made scrollable, cleaner design).
  - Improved icon theme loading and SVG rendering.
- **Performance & Async:**
  - Decoupled async updates; tiles update as soon as they are ready.
  - Refactored search scoring and improved fuzzy matching.
- **Configuration:**
  - Added configuration hot-reloading (reloads when files change).
  - Improved error handling for currency exchange rate updates and empty configs.
- **Documentation:** General code documentation improvements.

### Refactoring

- **Architecture:** - Major refactor of launcher-specific widgets; moved away from `mod.rs` into module-specific files.
  - Refactored `main.rs` into a dedicated `app` module.
  - Refactored `SherlockError` into `SherlockMessage` for better maintainability.
- **File Search:** Moved file search helpers to `utils.rs` and added model variants.
- **ContextMenu:** Moved `ContextMenuAction` to `context_menu.rs`.

### Fixed

- **Core:** Resolved `tokio-gpui` incompatibility by migrating to `smol`.
- **Deserialization:** Fixed incorrect deserialization/serialization of `ContextMenuActions` which caused application loading failures.
- **Emoji:** Fixed skin tone application issues and incorrect loading state.
- **Files:** Removed hardcoded home directory references.
- **Launchers:** Fixed launcher type migration and arg bar completion.
- **UI:** Fixed double borders, backgrounds, and incorrect icon/context menu rendering.
