# Launchers

Launchers are the backbone of Sherlock. Each item — applications, custom commands, widgets — is backed by a launcher. The `fallback.json` file controls which launchers are active and how they behave.

The default location is `~/.config/sherlock/fallback.json`.

---

## Shared Attributes

### Required

| Attribute | Description |
|---|---|
| `type` | The launcher type string (e.g. `app_launcher`, `command`) |
| `args` | Arguments specific to the launcher type. Can be empty (`{}`) |
| `priority` | Display order at startup. `0` means the launcher only appears when its `alias` is active |

### Optional

| Attribute | Description |
|---|---|
| `name` | Category label shown under the tile's name |
| `alias` | Prefix used to search within this launcher specifically |
| `home` | When to show this launcher. One of `Home`, `OnlyHome`, `Persist`, `Search` (default) |
| `async` | Run the launcher asynchronously |
| `on_return` | What happens when return is pressed |
| `spawn_focus` | Whether the tile auto-focuses when it appears first in the list |
| `shortcut` | Whether to show the shortcut indicator |
| `actions` | Custom context menu entries — see [Actions](#actions) |
| `variables` | Runtime input fields in the search bar — see [Variable Inputs](variable-inputs.md) |

---

## App Launcher

Searches and launches installed applications from `.desktop` files.

```json
{
    "name": "App Launcher",
    "alias": "app",
    "type": "app_launcher",
    "args": {
        "use_keywords": true
    },
    "priority": 2,
    "home": "Home"
}
```

### args

| Field | Required | Description |
|---|---|---|
| `use_keywords` | no | If `true`, also searches the `Keywords` field from the `.desktop` file |

---

## Bookmark Launcher

Finds and launches browser bookmarks.

```json
{
    "name": "Bookmarks",
    "type": "bookmarks",
    "args": {},
    "priority": 3,
    "home": "Search"
}
```

### Supported browsers

| Browser | Config value in `default_apps` |
|---|---|
| Zen | `zen`, `zen-browser`, `/opt/zen-browser-bin/zen-bin %u` |
| Firefox | `firefox`, `/usr/lib/firefox/firefox %u` |
| Brave | `brave`, `brave %u` |
| Chrome | `chrome`, `google-chrome`, `/usr/bin/google-chrome-stable %u` |
| Thorium | `thorium`, `/usr/bin/thorium-browser %u` |

The browser is matched against the `exec` string in your config, so both the short name and the full path are accepted.

---

## Calculator

Evaluates math expressions and unit conversions. On return, copies the result to the clipboard.

```json
{
    "name": "Calculator",
    "type": "calculation",
    "args": {
        "capabilities": [
            "calc.math",
            "calc.units"
        ]
    },
    "priority": 1
}
```

### args

| Field | Required | Description |
|---|---|---|
| `capabilities` | no | List of enabled features. Defaults to `calc.math` and `calc.units` |

### Capabilities

| Value | Enables |
|---|---|
| `calc.math` | Mathematical expression evaluation |
| `calc.units` | All unit conversions |
| `calc.length` | Length only |
| `calc.weight` | Weight only |
| `calc.volume` | Volume only |
| `calc.temperature` | Temperature only |
| `calc.pressure` | Pressure only |
| `calc.digital` | Digital storage only |
| `calc.time` | Time only |
| `calc.area` | Area only |
| `calc.speed` | Speed only |
| `calc.currencies` | Currency conversion (fetched from TradingView, cached) |
| `colors` | Color space conversion (`rgb`, `hex`, `hsl`, `hsv`, `lab`) |

---

## Category Launcher

Groups launchers or commands under a single tile. Activating the tile switches into that launcher's mode.

```json
{
    "name": "Categories",
    "alias": "cat",
    "type": "categories",
    "args": {
        "categories": {
            "Kill Processes": {
                "icon": "sherlock-process",
                "exec": "kill",
                "search_string": "terminate;kill;process"
            },
            "Power Menu": {
                "icon": "battery-full-symbolic",
                "exec": "pm",
                "search_string": "powermenu;"
            }
        }
    },
    "priority": 3,
    "home": "Home"
}
```

### args

**`categories`** (required) — a map of named entries:

| Field | Required | Description |
|---|---|---|
| `icon` | no | Icon name to display |
| `icon_class` | no | CSS class applied to the icon |
| `exec` | no | Alias of the launcher to activate on return |
| `search_string` | no | String used for fuzzy matching |

---

## Command Launcher

Runs custom shell commands. Supports variable inputs and replacement variables.

```json
{
    "name": "Utilities",
    "alias": "ex",
    "type": "command",
    "args": {
        "commands": {
            "NordVPN": {
                "icon": "nordvpn",
                "exec": "nordvpn c {variable:location}",
                "search_string": "nordvpn",
                "variables": [
                    { "string_input": "location" }
                ]
            }
        }
    },
    "priority": 5
}
```

### args

**`commands`** (required) — a map of named entries:

| Field | Required | Description |
|---|---|---|
| `exec` | yes | The command to run |
| `icon` | no | Icon name to display |
| `icon_class` | no | CSS class applied to the icon |
| `search_string` | no | String used for fuzzy matching |
| `variables` | no | Variable input fields — see [Variable Inputs](variable-inputs.md) |
| `tag_start` | no | Content shown in the left tag |
| `tag_end` | no | Content shown in the right tag |

---

## Music Player

Shows the currently playing track and controls playback via MPRIS over D-Bus.

```json
{
    "name": "Spotify",
    "type": "audio_sink",
    "args": {},
    "async": true,
    "priority": 1,
    "home": "Home",
    "spawn_focus": false,
    "actions": [
        {
            "name": "Skip",
            "icon": "media-seek-forward",
            "exec": "playerctl next",
            "method": "command"
        }
    ],
    "binds": [
        { "bind": "control+p", "callback": "playpause", "exit": false },
        { "bind": "control+l", "callback": "next", "exit": false },
        { "bind": "control+h", "callback": "previous", "exit": false }
    ]
}
```

### Inner functions

| Function | Description |
|---|---|
| `playpause` | Toggle playback |
| `next` | Skip to next track |
| `previous` | Go to previous track |
| `unbind` | Unbind a key (useful to unbind return) |

---

## Weather Launcher

Shows current weather conditions for a configured location.

```json
{
    "name": "Weather",
    "type": "weather",
    "args": {
        "location": "berlin",
        "update_interval": 60,
        "icon_theme": "Sherlock",
        "show_datetime": false
    },
    "priority": 1,
    "home": "OnlyHome",
    "async": true,
    "shortcut": false,
    "spawn_focus": false
}
```

### args

| Field | Required | Description |
|---|---|---|
| `location` | yes | City or region name |
| `update_interval` | no | Cache TTL in minutes |
| `icon_theme` | no | `Sherlock` to use bundled icons, omit for system theme |
| `show_datetime` | no | Show current date and time alongside weather |

---

## Web Launcher

Opens a search query in the browser using a configured search engine.

```json
{
    "name": "Web Search",
    "display_name": "Google Search",
    "alias": "gg",
    "type": "web_launcher",
    "args": {
        "search_engine": "google",
        "icon": "google"
    },
    "priority": 100
}
```

### args

| Field | Required | Description |
|---|---|---|
| `search_engine` | yes | Engine name or a custom URL containing `{keyword}` |
| `icon` | yes | Icon name to display |

### Built-in search engines

| Name | URL |
|---|---|
| `google` | `https://www.google.com/search?q={keyword}` |
| `bing` | `https://www.bing.com/search?q={keyword}` |
| `duckduckgo` | `https://duckduckgo.com/?q={keyword}` |
| `yahoo` | `https://search.yahoo.com/search?p={keyword}` |
| `ecosia` | `https://www.ecosia.org/search?q={keyword}` |
| `startpage` | `https://www.startpage.com/sp/search?q={keyword}` |
| `qwant` | `https://www.qwant.com/?q={keyword}` |
| `yandex` | `https://yandex.com/search/?text={keyword}` |
| Custom | Any URL with `{keyword}` as the query placeholder |

---

## Clipboard Launcher

> **Not yet implemented in this version**

Reads the clipboard and acts on its content — opening URLs, displaying colors, or evaluating expressions.

---

## Debug Launcher

> **Not yet implemented in this version**

Runs internal debug commands such as clearing the cache or resetting launch counts.

---

## Emoji Picker

> **Not yet implemented in this version**

Searches and inserts emoji characters.

---

## Bulk Text

> **Not yet implemented in this version**

Runs an external script asynchronously and displays its output as a text widget.

---

## Teams Event

> **Not yet implemented in this version**

Shows upcoming Microsoft Teams meetings and joins them on return.

---

## Theme Picker

> **Not yet implemented in this version**

Lists available themes and applies them on selection.

---

## Process Terminator

> **Not yet implemented in this version**

Lists running user processes and terminates the selected one on return.

---

## Pomodoro Timer

> **Not yet implemented in this version**

Displays a Pomodoro focus timer. Requires the external [sherlock-pomodoro](https://github.com/Skxxtz/sherlock-pomodoro) client.
