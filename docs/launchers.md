# Launchers

In the Sherlock Application Launcher, each tile is associated with a specific "launcher." You can think of a launcher as a category to which tiles belong. For example, if a launcher is set to invisible, all tiles under that launcher will also be invisible.<br>

Launchers are defined in the `fallback.json` file located in your config directory (`/home/user/.config/sherlock/`). If the application cannot find your configuration, it will fallback to the default configuration, which is stored in [fallback.json](resources/fallback.json).<br>

> **Example File:** [fallback.json](https://github.com/Skxxtz/sherlock/blob/main/docs/examples/fallback.json)
<br>

The launcher can be of the following types:<br>

- **[App Launcher](#app-launcher):** Launches your apps. 
- **[Web Launcher](#web-launcher):** Opens the ``{keyword}`` in your default webbrowser. The used search engine is configureable and the most common search engines are included. 
- **[Calculator](#calculator):** Converts your input into a math equation and displays its result. On Return, it also copies the result into the clipboard.
- **[Clipboard Launcher](#clipboard-launcher):** Checks if your clipboard currently holds a URL. On Return, it opens the url in the default webbrowser.
- **[Command](#command-launcher):** This field can execute commands that do not rely on the ``{keyword}`` attribute (such as connecting to a specific wifi).
- **[Bulk Text](#bulk-text):** The Bulk Text is a way to launch a custom script/application in an async form and to display its result in a widget.
- **[Teams Event](#teams-event):** This launcher is capable of joining Microsoft Teams meetings that are scheduled to begin between 5mins ago and in 15mins. 


## Common Launcher Attributes
`[UI]` - used for UI <br>
`[FC]` - used to specify behaviour <br>
| Attribute   | Type | Description |
|-------------|------|-------------|
| `name`      | `[UI]` (required) | The name of the category the tiles belong to. This name will appear under the app’s name. It is required but can be left empty. |
| `alias`     | `[FC]` (optional) | The command used to search within this category. |
| `home`      | `[FC]` (optional) | Determines if the elements of this launcher are displayed at startup. |
| `type`      | `[FC]` (required) | Specifies the tile and functionality to be used for this Launcher. |
| `args`      | `[FC]` (required) | Arguments specific to the `type`. Can be left empty. |
| `priority`  | `[FC]` (required) | Defines the display order of launcher elements at startup. A value of 0 means the launcher will only be shown if the `alias` is active. |
| `async`     | `[FC]` (optional) | Indicates whether the launcher should run asynchronously. This is used in `Bulk Text`. |
| `on_return`     | `[FC]` (optional) | Specifies what to do if return is pressed on the tile. |

---

## App Launcher
```json
{
    "name": "App Launcher",
    "alias": "app",
    "type": "app_launcher",
    "args": {},
    "priority": 2,
    "home": true
}
```
---
## Web Launcher
```json
{
    "name": "Web Search",
    "display_name": "Google Search"
    "tag_start": "{keyword}",
    "tag_end": "{keyword}",
    "alias": "gg",
    "type": "web_launcher",
    "args": {
        "search_engine": "google",
        "icon": "google"
    },
    "priority": 100
}
```
### Arguments (args):
**`search_engine`** (required):
Can be either of the following:
| Search Engine   | URL                                      |
|-----------------|------------------------------------------|
| **Google**      | `https://www.google.com/search?q={keyword}` |
| **Bing**        | `https://www.bing.com/search?q={keyword}` |
| **DuckDuckGo**  | `https://duckduckgo.com/?q={keyword}`    |
| **Yahoo**       | `https://search.yahoo.com/search?p={keyword}` |
| **Baidu**       | `https://www.baidu.com/s?wd={keyword}`   |
| **Yandex**      | `https://yandex.com/search/?text={keyword}` |
| **Ask**         | `https://www.ask.com/web?q={keyword}`    |
| **Ecosia**      | `https://www.ecosia.org/search?q={keyword}` |
| **Qwant**       | `https://www.qwant.com/?q={keyword}`     |
| **Startpage**   | `https://www.startpage.com/sp/search?q={keyword}` |
| **Custom**      | `https://www.example.com/search={keyword}` |

**`icon`** (required):<br>
Sets the icon-name the launcher should show. For a guide on how to add your own icons see [!WARNING]

---

## Calculator
```json
{
    "name": "Calculator",
    "type": "calculation",
    "args": {},
    "priority": 1,
}
```

---
## Clipboard Launcher
```json
    {
        "name": "Clipboard",
        "type": "clipboard-execution",
        "args": {},
        "priority": 1,
        "home": true
    }
```
---
## Command Launcher
```json
{
    "name": "Example Command",
    "alias": "ex",
    "type": "command",
    "args": {
        "commands": {
            "command name": {
                "icon": "icon-name",
                "exec": "command to execute", 
                "search_string": "examplecommand"
                "tag_start": "{keyword}"
                "tag_end": "{keyword}"
            },
            "command2": {
                "icon": "icon-name",
                "exec": "command to execute", 
                "search_string": "examplecommand"
                "tag_start": "{keyword}"
                "tag_end": "{keyword}"
            }
        }
    },
    "priority": 5
}
```
### Arguments (args):
**commands** (required):<br>
Has following fields of its own:
1. `name field` / the name of the applicaiton
2. `icon` / the icon-name for the icon to display 
3. `exec` / the command to execute
4. `search_string` / the string to match to on search
5. `tag_start` / specifies what will be displayed in the start tag
6. `tag_end` / specifies what will be displayed in the end tag

---

## Bulk Text
```json
{
    "name": "Wikipedia Search",
    "alias": "wiki",
    "type": "bulk_text",
    "async": true,
    "on_return": "copy",
    "args": {
        "icon": "wikipedia",
        "exec": "~/.config/sherlock/scripts/sherlock-wiki"
        "exec-args": "{keyword}"
    },
    "priority": 0
}
```
### Arguments (args):
**`icon`** (required):<br>
Specifies the icon shown for the command.<br>

**`exec`** (required):<br>
Specifies the program that should be run. **Note:** that its probably suitable to run it asynchronously. To do that, set the `async` attribute to `true`. <br>

**`exec-args`** (optional):<br>
Specifies the arguments to pass along to the `exec` program.<br>

> The provided snippet works with the project [sherlock-wiki](https://github.com/Skxxtz/sherlock-wiki) 
--- 

## Teams Event
> **🚨 Warning:** Currently only supports Thunderbird calendar events
```json
{
    "name": "Teams Event",
    "type": "teams_event",
    "args": {
        "event_date": "now",
        "event_start": "-5 minutes",
        "event_end": "+15 minutes"
    },
    "priority": 1,
    "home": true
},
```

### Arguments (args):
**`icon`** (optional):<br>
Specifies the icon shown for the event.<br>

**`event_date`** (optional):<br>
Specifies the date for the event lookup<br>

**`event_start`** (optional):<br>
Specifies the offset from the `date` parameter.<br>

**`event_end`** (optional):<br>
Specifies the second offset from the `date` parameter.<br>
--- 
