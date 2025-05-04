# Sherlock Actions

## Description and Use Case
Sherlock actions can be used to run specific actions on every n runs. For example, this can be used to clear Sherlock's cache every 100 runs. 

## Usage
To use Sherlock actions, the `sherlock_actions.json` file is required. The file must contain a valid JSON array containing `SherlockAction` objects. These objects are defined like so:
```json
{
    "on": 100,
    "action": "clear_cache"
}
```
**Arguments**
- `on`: an integer, related to the nth run on which the action should be executed..
- `action`: ""

## Available Actions
This feature was implemented in version `0.1.11`. As of now, there's only one available action. If you have any ideas for others, feel free to open a Github issue or one on the Discord server.

### `clear_cache`
Runs the following steps:
- Clears the `~/.cache/sherlock/` directory (including all subdirectories)
- Clears the .desktop file cache in the specified location

