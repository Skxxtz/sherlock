# Changelog

All notable changes to the **Sherlock Launcher** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),

1. MAJOR version when you make incompatible API changes
2. MINOR version when you add functionality in a backward compatible manner
3. PATCH version when you make backward compatible bug fixes

Additional labels for pre-release and build metadata are available as extensions to the MAJOR.MINOR.PATCH format.

run `git log main..dev` for all changes

---

## [0.2.1-dev] - 22.04.26

### Added

- Implemented default `fallback.json` file if none is provided by the user
- Added functionality to the clipboard launcher to open URLs based on intent


### Improvements

- Added automatical alias execution for file search and emoji picker
- Added ease animation for the weather launcher
- Improved app loader
- Improved intent parsing to use iterators instead of `smallvecs`

### Refactoring

- Improved extensibility and cleanliness of currency fetching
- Refactored multiple code sections according to `clippy` suggestions

### Fixed

- Fixed `wttr.in` repeating format change
