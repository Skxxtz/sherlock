# Configuration File

The configuration file for Sherlock is located at `~/.config/sherlock/config.toml`, unless specified otherwise. This file allows you to customize various parameters to tailor Sherlock to your needs. Below, we will explore the available options and their purposes.
<br>
> **Example File:** [config.toml](https://github.com/Skxxtz/sherlock/blob/main/docs/examples/config.toml)
---

## Default App Section `[default_apps]`

| **Keyword**       | **Default**          | **Explanation**                                                                                                                  |
|-------------------|----------------------|-------------------------------------------------------------------------------------------------------------------------------|
| `terminal`        | Automatically detected | May be required if the `TERMINAL` environment variable is not set. Specify the executable name of your terminal (e.g., `"gnome-terminal"`, `"konsole"`). |
| `teams`        | `teams-for-linux --enable-features=UseOzonePlatform --ozone-platform=wayland --url {meeting_url}` | Only required for the teams-event tile to automatically enter a teams meeting. The `{meeting_url}` will be replaced by the actual teams meeting URL. |
| `calendar_client`        | `thunderbird` | Sets your calendar client used in event tiles. Currently only thunderbird is supported. |

---
## Units Section `[units]`
| **Keyword**       | **Default**          | **Explanation**                                                                                                                  |
|-------------------|----------------------|-------------------------------------------------------------------------------------------------------------------------------|
| `lengths`        | `meter`| Sets the default unit for any length calculations. |
| `weights`        | `kg`| Sets the default unit for any weight calculations. |
| `volumes`        | `l`| Sets the default unit for any volume calculations. |

---
## Debug Section `[debug]`

| **Keyword**           | **Default** | **Explanation**                                                                 |
|-----------------------|-------------|---------------------------------------------------------------------------------|
| `try_suppress_errors` | `false`     | If set to `true`, errors and warnings will not be displayed when starting the app. |
| `try_suppress_warnings` | `false`   | If set to `true`, only errors will trigger the error screen at startup, while warnings will be ignored. |
| `app_paths` | `[]`   | Adds custom paths to search for `.desktop` files. Should be a list of strings. |

---

## Appearance Section `[appearance]`

| **Keyword**     | **Default** | **Explanation**                                                                                                                 |
|-----------------|-------------|-------------------------------------------------------------------------------------------------------------------------------|
| `width`    | `900`        | Sets the width of the main window.|
| `height`    | `593`        | Sets the height of the main window. | 
| `gsk_renderer`  | `"cairo"`   | Specifies the renderer used to display the Sherlock application. During testing, `cairo` showed the fastest startup times. You can also use `ngl`, `gl`, or `vulkan` based on your system's performance. |
| `recolor_icons` | `false`     | REMOVED |
| `icon_paths`    | `[]`        | Defines custom paths for the application to search for icons. This is useful for adding custom icons for commands or aliases through the `sherlockalias` file. |
| `icon_size`    | `22`        | Sets the default icon size for the icons in each tile. |
| `search_icon`    | `true`        | Enables or disables the use of the search icon |
| `use_base_css`    | `true`        | Enables or disables the extension of Sherlock's default style sheet. |

---
## Behavior Section `[behavior]`

| **Keyword**           | **Default** | **Explanation**| **Documentation** |
|-----------------------|-------------|---------------------------------------------------------------------------------|-------------------|
| `caching` | `false`     | If set to `true`, Desktop file caching will be activated to either the specified or the default location `~/.cache/sherlock_desktop_cache.json`. |[Caching](https://github.com/Skxxtz/sherlock/blob/documentation/docs/features/daemonizing.md)|
| `cache` | `~/.cache/sherlock_desktop_cache.json`   | Overrides the default caching location. ||
| `daemonize` | `false`     | If set to `true`, Sherlock will run in daemon mode. This will consume more memory because the rendered application will be kept in memory. Daemonizing will allow faster startup times. Send the `open` message to socket `/tmp/sherlock_daemon.socket` to open the window. |[Daemonizing](https://github.com/Skxxtz/sherlock/blob/documentation/docs/features/daemonizing.md)|
| `animate` | `true`   | Sets if startup animation should play. (Only works on daemonize=false) ||
| `launch_prefix` | `None` | Command prefix used for launching applications, e.g., `"uwsm app --"`. ||
---
## Binds Section `[binds]`

The `[binds]` section allows you to configure additional keybindings for navigation. The values of the binds are specified in the format `<modifier>-<key>`. For example, `control-tab` binds the Control key and the Tab key. If you only want to bind a single Key, you only provide `<key>`. For the modifier key you can only provide `<modifier>.

| **Keyword**           | **Default** | **Explanation**                                                                 |
|-----------------------|-------------|---------------------------------------------------------------------------------|
| `prev` | `None`     | Defines an additional keybind to switch to the previous item in the list. |
| `next` | `None`     | Defines an additional keybind to switch to the next item in the list. |
| `modifier` | `control`     | Defines the keybind used for shortcuts (`<modifier>+<1-5>`) and the clearing of the search bar using (`<modifier>+<backspace>`)  |


### Available Keys
| Key Input   | Config Name  |
|------------|-------------|
| `<Tab>`    | `tab`       |
| `<Up>`     | `up`        |
| `<Down>`   | `down`      |
| `<Left>`   | `left`      |
| `<Right>`  | `right`     |
| `<PageUp>` | `pgup`      |
| `<PageDown>` | `pgdown`  |
| `<End>`    | `end`       |
| `<Home>`   | `home`      |

### Available Modifiers
| Key Input   | Config Name  |
|------------|-------------|
| `<Shift>`  | `shift`     |
| `<Control>`| `control`   |
| `<Alt>`    | `alt`       |
| `<Super>`  | `super`     |
| `<Lock>`   | `lock`      |
| `<Hyper>`  | `hypr`      |
| `<Meta>`   | `meta`      |

---
## Files Section `[files]`
This section holds the location for the config files.

| **Keyword**           | **Default** | **Explanation**|
|-----------------------|-------------|-----------------------------------|
| `fallback` | `~/.config/sherlock/fallback.json`     | Sets the location for the `fallback.json` file | 
| `css` | `~/.config/sherlock/main.css`     | Sets the location for the `main.css` file | 
| `alias` | `~/.config/sherlock/sherlock_alias.json`     | Sets the location for the `sherlock_alias.json` file | 
| `ignore` | `~/.config/sherlock/sherlockignore`     | Sets the location for the `sherlockignore` file | 
---
