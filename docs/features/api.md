# Sherlock Socket API

Commands are sent as JSON-serialized enum variants of `ApiCall`.

| Command Variant               | Description                           | Example JSON                         |
|------------------------------|-------------------------------------|------------------------------------|
| `InputOnly`                  | Enable input-only mode               | `{ "InputOnly": null }`             |
| `Obfuscate` (bool)           | Enable or disable obfuscation       | `{ "Obfuscate": true }`             |
| `Socket` (`Option<String>`)  | Use socket with optional name       | `{ "Socket": "my-socket" }` or `{ "Socket": null }` |
| `Show`                      | Show current state/info              | `{ "Show": null }`                  |
| `Clear`                     | Clear internal buffers or UI         | `{ "Clear": null }`                 |
| `SherlockError`            | Send an error message               | `{ "SherlockWarning": {error: SherlockErrorType, traceback: String} }` |
| `SherlockWarning`            | Send a warning message               | `{ "SherlockWarning": {error: SherlockErrorType, traceback: String} }` |
| `ClearAwaiting`              | Clear commands waiting execution     | `{ "ClearAwaiting": null }`         |
| `Pipe` (String)              | Pipe string input through processor | `{ "Pipe": "some input" }`          |
| `DisplayRaw` (String)        | Display raw output string            | `{ "DisplayRaw": "raw output" }`   |
| `SwitchMode` (`SherlockModes`) | Change operating mode             | `{ "SwitchMode": "mode_name" }`    |
| `SetConfigKey` (String, String) | Set config key-value pair        | `{ "SetConfigKey": ["key", "value"] }` |

For the above mentioned `SherlockError` and `SherlockWarning` the following `SherlockErrorTypes` exist:
```rust
enum SherlockErrorType {
    // Debug
    DebugError(String),
    // Environment
    EnvVarNotFoundError(String),

    // Filesystem - Files
    FileExistError(PathBuf),
    FileReadError(PathBuf),
    FileWriteError(PathBuf),
    FileParseError(PathBuf),
    FileRemoveError(PathBuf),

    // Filesystem - Directories
    DirReadError(String),
    DirCreateError(String),
    DirRemoveError(String),

    // Config & Flags
    ConfigError(Option<String>),
    FlagLoadError,

    // Resources
    ResourceParseError,
    ResourceLookupError(String),

    // Display / UI
    DisplayError,
    ClipboardError,

    // Regex / Parsing
    RegexError(String),

    // Commands
    CommandExecutionError(String),

    // DBus
    DBusConnectionError,
    DBusMessageConstructError(String),
    DBusMessageSendError(String),

    // Networking
    HttpRequestError(String),

    // Sockets
    SocketRemoveError(String),
    SocketConnectError(String),
    SocketWriteError(String),

    // Sqlite
    SqlConnectionError(),

    // (De-) Serialization
    SerializationError,
    DeserializationError,

    // Apps
    UnsupportedBrowser(String),

    // Icons
    MissingIconParser(String),
}
```

---

## Example Usage

```json
{ "SetConfigKey": ["animate", false] }
```
