# Configuration File

The configuration file for Sherlock is located at `~/.config/sherlock/config.toml`, unless specified otherwise. This file allows you to customize various parameters to tailor Sherlock to your needs. Below, we will explore the available options and their purposes.
<br>
> **Example File:** [config.toml](https://github.com/Skxxtz/sherlock/blob/main/docs/examples/config.toml)
---

## Default App Section `[default_apps]`

| **Keyword**       | **Default**          | **Explanation**                                                                                                                  |
|-------------------|----------------------|-------------------------------------------------------------------------------------------------------------------------------|
| `terminal`        | Automatically detected | May be required if the `TERMINAL` environment variable is not set. Specify the executable name of your terminal (e.g., `"gnome-terminal"`, `"konsole"`). |
| `teams`        | `teams-for-linux --enable-features=UseOzonePlatform --ozone-platform=wayland --url {meeting_url}` | Only required for the teams-event tile to automatically enter a teams meeting. The `{meeting_url}` will be replaced by the actual teams meeting url. |
| `calendar_client`        | `thunderbird` | Sets your calendar client used in event tiles. Currently only thunderbird is supported. |

---
## Debug Section `[debug]`

| **Keyword**           | **Default** | **Explanation**                                                                 |
|-----------------------|-------------|---------------------------------------------------------------------------------|
| `try_suppress_errors` | `false`     | If set to `true`, errors and warnings will not be displayed when starting the app. |
| `try_suppress_warnings` | `false`   | If set to `true`, only errors will trigger the error screen at startup, while warnings will be ignored. |

---

## Appearance Section `[appearance]`

| **Keyword**     | **Default** | **Explanation**                                                                                                                 |
|-----------------|-------------|-------------------------------------------------------------------------------------------------------------------------------|
| `width`    | `900`        | Sets the width of the main window.|
| `height`    | `593`        | Sets the height of the main window. | 
| `gsk_renderer`  | `"cairo"`   | Specifies the renderer used to display the Sherlock application. During testing, `cairo` showed the fastest startup times. You can also use `ngl`, `gl`, or `vulkan` based on your system's performance. |
| `recolor_icons` | `false`     | Appends the `-symbolic` postfix to all icons, allowing them to be colorized. Note: not all icons have a symbolic version.       |
| `icon_paths`    | `[]`        | Defines custom paths for the application to search for icons. This is useful for adding custom icons for commands or aliases through the `sherlockalias` file. |
| `icon_size`    | `22`        | Sets the default icon size for the icons in each tile. |

---
## Behavior Section `[behavior]`

| **Keyword**           | **Default** | **Explanation**                                                                 |
|-----------------------|-------------|---------------------------------------------------------------------------------|
| `caching` | `false`     | If set to `true`, Desktop file caching will be activated to either the specified or the default location `~/.cache/sherlock_desktop_cache.json`. |
| `cache` | `~/.cache/sherlock_desktop_cache.json`   | Overrides the default caching location. |
| `daemonize` | `false`     | If set to `true`, Sherlock will run in daemon mode. This will consume more memory because the rendered application will be kept in memory. Damonizing will allow faster startup times. Send the `show` message to socket `/tmp/sherlock_daemon.socket` to open the window. For example with `echo "show" \| socat - UNIX-CLIENT:/tmp/sherlock_daemon.socket`|
| `animate` | `true`   | Sets if startup animation should play. (Only works on deamonize=false) |

---
