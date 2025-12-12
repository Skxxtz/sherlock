# Sherlock Alias

Sherlock aliases allow you to customize how applications appear and behave
inside Sherlock. They enable you to override an applications presentation,
launch behavior, and context menu actions without modifying the original
`.desktop` file.

## Motivation

Suppose the application **Brave Browser** should:

- Appear under the display name **Brave**
- Use a custom icon
- Always launch with Electron Wayland flags for improved rendering
- Provide context-menu options, e.g. Start private window
- Support variable input fields
- Appear when searching for custom keywords

These modifications are difficult to achieve using default desktop entries. Sherlock aliases solve this problem by giving you full user-side control over:

1. `name`: Application name
2. `icon`: Icon
3. `keywords`: Keywords
4. `exec`: Execution command
5. `add_actions`: Additional Sherlock context-menu actions
6. `actions`: Overwriting context-menu actions
7. `variables`: Variable input fields

## Creating and Using Sherlock Aliases

### Step 1 - Create the alias file

Create an empty alias configuration file (if it does not exist):

```bash
    echo {} > ~/.config/sherlock/sherlock_alias.json
```

or run:

```bash
sherlock init
```

### Step 2 - Identify the application

Find the application name as Sherlock detects it. Thhis is the key used to override it.

### Step 3 - Add an alias entry

Insert a JSON object into `sherlock_alias.json`:

```json
{
    "Current App Name": {
        "name": "Desired Name",
        "icon": "your-icon",
        "keywords": "sample alias",
        "variables": [
            {"string_input": "variable name"}
        ],
        "exec": "/path/to/application --your-flags %U {prefix[variable name]:some prefix}{variable:variable name}",
        "add_actions": [
            {
                "name": "Example Action",
                "exec": "/path/to/application --your-flags",
                "icon": "your-icon",
                "method": "method"
            }
        ],
        "actions": [
            {
                "name": "Example Action",
                "exec": "/path/to/application --your-flags",
                "icon": "your-icon",
                "method": "method"
            }
        ]
    }
}
```

> [!TIP]
> For full documentation on actions, see [Actions](https://github.com/Skxxtz/sherlock/blob/main/docs/features/actions.md)
