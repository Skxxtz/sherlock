## Functionalities
- [ ] Make the home screen function as a dashboard with event tiles, spotify widget, most used apps widget, time, etc
    - [x] spotify 
    - [x] weather
    - [ ] most used apps widget
    - [ ] tile
- [ ] Create a widget that uses `gtk4::Builder::from_string(ui_string)`.
- [x] Make more widgets asynchronous.
- [ ] Add a callback type for a command to execute another command.
- [ ] Implement command execution count and sort commands based on that count.
    - [x] Implement basic execution counter
    - [x] Make the execution count aware to changes in to the alias file and check for validity
    - [ ] If maximum_decimal exceeds a specific number, reset it
    - [x] think about storing f32 directly in the file to reduce calculation of 10^-n every time
- [ ] Finish setting up the loading animation for asynchronous widgets
- [ ] Property/Detail tab on the right side of the screen to display application information
- [ ] Markdown parser: Markdown > GTK ui
- [ ] Currency calculator function / clipboard function
- [x] Sherlock flag to clear cache like mpris cache

- X Consider changing the alias for each command in `commandlauncher` (if possible). -> Not viable

## Configuration
- [ ] Give scripts control over what tile to spawn.
- [ ] Implement user-defined CSS classes for tile tags.
- [ ] Implement the possibility to customize categories and their UI files. Allow specifying the UI files used for the categories. Required: category config file. (What ui file should be used for a specific cateogory)

## Refactoring
- [x] Take a look at the search/user-display functions and extract common functionalities

## Documentation
- [ ] Add documentation for custom scripts

## Porting
- [ ] X11 Support? Seems to work apart from centering the window and hiding the title bar
- [ ] Windows? 

