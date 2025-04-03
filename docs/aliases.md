# Aliases
Sherlock aliases provide a way for you to customize the:
1. Name
2. Icon
3. Keywords
4. Exec
of an app.
<br>
> **Example File:** [sherlock_alias.json](https://github.com/Skxxtz/sherlock/blob/main/docs/examples/sherlock_alias.json)

## Setup:
1. Create the `sherlock_alias.json` file:
```echo {} > ~/.config/sherlock/sherlock_alias.json
```
2. Find the application you want to alias
3. Write a simple alias entry into the alias file
```json
{
    "the current app name":{
        "name": "your desired name",
        "icon": "your icon",
        "exec": "/path/to/applicatoin --your-flags %U",
        "keywords": "sample alias"
    }
}
```
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
