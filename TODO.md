## UI
- [x] Improve startup time further by reducing the number of widgets shown initially. Allow the user to configure which launchers are displayed at startup.
- [x] Add error tile.

## Scripts
- [x] On startup, check if the clipboard contains a URL and enable URL search from it.
    - [x] Improve the functionality further for example display colors
- [ ] Create a custom Spotify (ncspot) script to control it.


## Functionalities
- [x] Implement a locking mechanism to ensure only one instance is running at a time.
- [x] Implement tags on command widgets 
- [x] Add an "return" command type for a tile.
- [x] Implement a `next()` function that adds a layout to the launcher stack to navigate within Sherlock, similar to a follow-up screen.
- [x] Startup animation
- [ ] Create a widget that uses `gtk4::Builder::from_string(ui_string)`.
- [ ] Modular Widget: depending on an external condition (e.g. Spotify is playing -> Spotify tile)
- [ ] Make more widgets asynchronous.
- [ ] Add an `ArgCommand` launcher type or convert the existing one to be more versatile.
- [ ] Add a callback type for a command to execute another command.
- [ ] Implement command execution count and sort commands based on that count.
- [ ] Finish setting up the loading animation for asynchronous widgets
- [ ] Property/Detail tab on the right side of the screen to display application information
- [ ] Markdown parser: Markdown > GTK ui
- [ ] Currency calculator function / clipboard function

- X Consider changing the alias for each command in `commandlauncher` (if possible). -> Not viable

## Configuration
- [x] Allow the user to set flags such as `--config` to configure fallback, style, and config files.
- [x] Implement a custom file for aliases and custom icons for apps.
- [x] Improve Sherlock ignore file to support `*` macro and case sensitivity.
- [x] Make it possible to customize the height and width of the window.
- [ ] Provide better user integration from custom scripts, e.g., give control over tiles to spawn.
    - [x] Implement piping into sherlock
    - [ ] Use flag to specify what type of piping input - e.g. json for more control over what to display (or what not)
- [ ] Implement user-defined CSS classes for tile tags.
- [ ] Implement the possibility to customize categories and their UI files. Allow specifying the UI files used for the categories. Required: category config file. (What ui file should be used for a specific cateogory)

## Refactoring
- [ ] Take a look at the search/user-display functions and extract common functionalities

## Documentation
- [x] Add flag section in docs.


