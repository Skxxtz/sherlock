# Flags

## Basic Flags
| Flag         | Description | 
|--------------|----------------------|
| --version    | Print the version of the application.|
| --help       | Show this help message with allowed flags.                                        | 
| init | Creates default configuration files into the `~/.config/sherlock/` directory.|

## File Flags
| Flag         | Description | Note |
|--------------|----------------------|------------------------------------------------|
| --config     | Specify the configuration file to load. | Recommended location: `~/.config/sherlock/config.toml` |
| --fallback   | Specify the fallback file to load. | Recommended location: `~/.config/sherlock/fallback.json`             |
| --style      | Set the style configuration file. | Recommended location: `~/.config/sherlock/style.css`                 |
| --ignore     | Specify the Sherlock ignore file. | Recommended location: `~/.config/sherlock/sherlockignore`            |
| --alias      | Specify the Sherlock alias file. |Recommended location: `~/.config/sherlock/sherlock_alias.json`       |
| --cache      | Specify the location for the caching file. Sets caching active |Recommended location: `~/.cache/sherlock/sherlock_desktop_cache.json`       |

## Behavioral Flags
| Flag         | Description | Note |
|--------------|----------------------|----------------------------------|
| --daemonize      | Overrides the daemonizing value in `config.toml` | |
| --time-inspect      | Prints startup time from 0 to content. Also prints the time it took to load the launchers.| (Removed) use `TIMING=true sherlock` instead.|
| --sub-menu      | Launch Sherlock with a custom alias from the beginning. For example `sherlock --sub-menu pm`, where `pm` is an alias you defined in your fallbacks.| |

## Pipe Mode Flags
| Flag         | Description | Note |
|--------------|----------------------|------------------------------------------------|
| --display-raw      | Displays the piped input as a text field. Useful for term graphics. | |
| --center | Centers the content.  | Only works with `--display-raw`|
| --method | Specifies the method Sherlock will use to handle return presses. | Can either be `print` or `copy`|
| --field | Selects a field as the output data.  | Only works with json formatting. |

## Environment Variables
| Flag         | Description | Note |
|--------------|----------------------|------------------------------------------------|
| `TIMING=true` |Prints timing information for several functions. | |
