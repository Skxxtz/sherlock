# Aliases
Sherlock aliases allow you to customize how applications are displayed and launcher within Sherlock. 
## Example Problem
The application `Brave Browser` should:
- Appear with the display name **Webbrowser**
- Use a custom **Webbrowser icon**
- Alwaays be launched with **Electron Wayland flags** to enhance rendering quality
- Should have context menu entries to start a new private window
This is a problem Sherlock aliases can fix.<br>

Sherlock aliases provide a way for you to customize the application's:
1. Name
2. Icon
3. Keywords
4. Exec
5. Sherlock Context Menu Entries
<br>

> **Example File**: [sherlock_alias.json](https://github.com/Skxxtz/sherlock/blob/main/docs/examples/sherlock_alias.json)

## Usage:
1. Create the `sherlock_alias.json` file:
`echo {} > ~/.config/sherlock/sherlock_alias.json` or `Sherlock init`
2. Find the application you want to alias
3. Write a simple alias entry into the alias file
```json
{
    "the current app name":{
        "name": "your desired name",
            "icon": "your icon",
            "exec": "/path/to/applicatoin --your-flags %U",
            "keywords": "sample alias",
            "add_actions": [
            {
                "Example Action",
                "exec": "/path/to/application --your-flags",
                "icon": "your icon",
                "method": "method",
            }
            ],
            "actions": [
            {
                "Example Action",
                "exec": "/path/to/application --your-flags",
                "icon": "your icon",
                "method": "method",
            }
            ]
    }
}
```
### Available Methods for Actions/Add_actions

- `category`: Uses the `exec` to open a new mode
- `app_launcher`: Opens the `exec` as an app
- `web_launcher`: Opens the `exec` as a link in your default web browser
- `command`: Opens the `exec` as a command
- `debug`: Matches the `exec` against
    - `clear_cache`: To clear the application's cache
    - `show_errors`: To switch to the error/warning screen
    - `reset_counts`: To reset the execution counter

**DONE!**<br>

## Examples
### Start `vesktop` using Wayland flags
```json
{
    "Vesktop": {
        "name": "Discord",
            "icon": "discord",
            "exec": "/usr/bin/vesktop --enable-features=UseOzonePlatform --ozone-platform=wayland %U",
            "keywords": "discord"
    },
}
```
