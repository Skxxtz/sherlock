# Variable Inputs

Variable inputs spawn a text field for each declared variable, collecting user input before the command is executed.

---

## Declaring variables

Variables are declared per-command in the config:

```json
"Vista": {
    "variables": [
        { "path_input": "Select file" }
    ],
    "exec": "/usr/bin/vista {variable:Select file}",
    "search_string": "vista;speed reader"
}
```

---

## Types

### `string_input`
A plain text input field.

```json
{ "string_input": "Enter username" }
```

### `password_input`
Masked text input. When the command contains `sudo`, the value is automatically piped to stdin via `sudo -S`.

```json
{ "password_input": "Sudo password" }
```

### `path_input`
A file system path input. The value is normalized before execution:
- `~` expanded to `$HOME`
- Relative paths resolved against `$HOME`
- Absolute paths passed through unchanged

```json
{ "path_input": "Select file" }
```

---

## Referencing variables in exec

Use `{variable:label}` where `label` matches the declared variable string exactly:

```json
"exec": "/usr/bin/vista {variable:Select file}"
```

Multiple variables:

```json
"variables": [
    { "string_input": "Host" },
    { "string_input": "Port" }
],
"exec": "ssh {variable:Host} -p {variable:Port}"
```

---

## Built-in substitutions

These work in any exec string without declaring a variable:

| Placeholder | Resolves to |
|---|---|
| `{keyword}` | The current search query typed by the user |
| `{terminal}` | The configured default terminal with `-e` appended |

```json
"exec": "{terminal} nvim {variable:File}"
```

---

## Optional variables

Variables are optional — if the user leaves a field empty, the `{variable:label}` placeholder is left as-is in the exec string. To avoid passing a bare placeholder to the command, wrap optional arguments with a conditional prefix instead of referencing the variable directly.

---

## Conditional prefix

Use `{prefix[label]:text}` to prepend text only when the variable has a non-empty value:

```json
"variables": [
    { "string_input": "Port" }
],
"exec": "ssh {prefix[Port]:-p }{variable:Port} {variable:Host}"
```

If `Port` is empty: `ssh myhost`
If `Port` is `22`: `ssh -p 22 myhost`

> The space is inside the prefix string — there is no space between `{prefix[Port]:-p }` and `{variable:Port}`.

