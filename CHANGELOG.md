# Changelog

All notable changes to the **Sherlock Launcher** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),

## Changelog Categories

### Changelog Category Definitions

* `Added`: Use for new features or capabilities that did not exist previously.
* `Changed`: Use for modifications to existing functionality or API updates.
* `Deprecated`: Use for features that are still present but scheduled for
  removal in future versions.
* `Removed`: Use for features that have been deleted or disabled.
* `Fixed`: Use for any bug fixes, unintended behavior repairs, or logic
  corrections.
* `Security`: Use specifically for patches addressing vulnerabilities or
  security exploits.

---

### Industry Common Additions

* `Improved`: Use for performance optimizations or refinements to existing features.
* `Performance`: Use for significant speed or memory usage improvements.

## Versioning Scheme

1. MAJOR version when you make incompatible API changes
2. MINOR version when you add functionality in a backward compatible manner
3. PATCH version when you make backward compatible bug fixes

Additional labels for pre-release and build metadata are available as
extensions to the MAJOR.MINOR.PATCH format.

run `git log main..dev` for all changes

---

## [0.2.1-dev] - 26.04.26

### Added

* **Config:** Implemented default `fallback.json` file if none is provided by
  the user
* **User Actions:** Added functionality to the clipboard launcher to open URLs
  based on intent
* **Piped Input:** Added basic support for piped input using *dmenu-style*
  newline splitting
* **Sub-menu Flag:** Implemented `-sm` / `--sub-menu` flag functionality and
  added smoother transitions
* **Alias Execution** Added automatic alias execution for file search and emoji
  picker

### Improvements

* **Animations:** Added ease animation for the weather launcher
* **NL Intents:** Improved intent parsing to use iterators instead of
  `smallvecs`
* **Currencies:** Improved currency factor fetching (extensibility,
  cleanliness)
* **Chore:** Refactored multiple code sections according to `clippy`
  suggestions

### Fixed

* **Weather:** Fixed `wttr.in` repeating format change
* **Networking:** Resolved a `tokio::net::UnixSocket` issue that limited the
  application to 64 concurrent openings
* **Launcher Parsing:** Fixed general launcher parsing issues
* **Tests:** Removed redundant clipboard watcher test causing issues in
  automated builds

## [0.2.0-dev] - 20.04.26

### Added

* **Translator:** Added new translation functionality.
* **Launchers:**
  * Implemented **Script Launcher** with "wait for return" support.
  * Implemented **Events Launcher** with `look_ahead` and `look_back` parameter parsing.
  * Added **Web Launcher** application action type.
* **Emoji Picker:** Added foundational picker, including context menus and default skin tone support.
* **File Search:** Implemented basic file search supporting `ripgrep` and `walkdir` backends.
* **Clipboard:** Added clipboard listener and functionality.
* **Integration:** Added support for Zoom meetings and XDG-settings mimetype handling.

### Improvements

* **UI/UX:**
  * Added visual shortcuts.
  * Improved design for script, event, and file tiles.
  * Added animations and improved transition handling.
  * Improved error views (made scrollable, cleaner design).
  * Improved icon theme loading and SVG rendering.
* **Performance & Async:**
  * Decoupled async updates; tiles update as soon as they are ready.
  * Refactored search scoring and improved fuzzy matching.
* **Configuration:**
  * Added configuration hot-reloading (reloads when files change).
  * Improved error handling for currency exchange rate updates and empty configs.
* **Documentation:** General code documentation improvements.

### Refactoring

* **Architecture:** - Major refactor of launcher-specific widgets; moved away from `mod.rs` into module-specific files.
  * Refactored `main.rs` into a dedicated `app` module.
  * Refactored `SherlockError` into `SherlockMessage` for better maintainability.
* **File Search:** Moved file search helpers to `utils.rs` and added model variants.
* **ContextMenu:** Moved `ContextMenuAction` to `context_menu.rs`.

### Fixed

* **Core:** Resolved `tokio-gpui` incompatibility by migrating to `smol`.
* **Deserialization:** Fixed incorrect deserialization/serialization of `ContextMenuActions` which caused application loading failures.
* **Emoji:** Fixed skin tone application issues and incorrect loading state.
* **Files:** Removed hardcoded home directory references.
* **Launchers:** Fixed launcher type migration and arg bar completion.
* **UI:** Fixed double borders, backgrounds, and incorrect icon/context menu rendering.
