# Sherlock Actions

## Description and Use Case

Sherlock actions can be used to run specific actions on every n runs. For example, this can be used to clear Sherlock's cache every 100 runs.

## Usage

To use Sherlock actions, the `sherlock_actions.json` file is required. The file must contain a valid JSON array containing `SherlockAction` objects. These objects are defined like so:

```json
{
    "on": 100,
    "action": "clear_cache"
    "exec": ""
}
```

**Arguments**

- `on`: an integer, related to the nth run on which the action should be executed..
- `action`: the action to execute
- `exec`: the exec argument for the action

## Available Actions

| Action | Exec | Use Case |
| --------------- | --------------- | --------------- |
| debug | restart | for daemon mode only. It restarts Sherlock, returning memory back to the OS |
| debug | clear_cache | clears the content of the `~/.cache/sherlock` directory and clears the .desktop file cache |
| debug | reset_log | resets the log, located in `~/.sherlock/sherlock.log` |
