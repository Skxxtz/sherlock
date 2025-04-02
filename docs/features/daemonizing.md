# Daemonizing (Experimental)  
Sherlock's daemonizing feature is still experimental. Therefore, it doesn't support all features yet.  

### Known Unsupported Features:  
- Startup animation won't work  
- Piping content into Sherlock won't work  

### Known Issues:  
- High memory usage  
- GPU will be defaulted to `on` state (on laptops)  

## How to Enable  
To enable `daemonizing`, you'll have to set its key in the `behavior` section of the `config.toml` file to `true`. Alternatively, you can run sherlock with the `--daemonize` flag. This will override the value set in the `config.toml` file.

## How to Use  
1. In your system's configuration, set Sherlock to run at startup.  
2. To open a window, send the `open` message to the socket located at `/tmp/sherlock_daemon.socket`.  

> **💡 Note:** In your system configuration, you can set a keybind to execute:  
> `echo "show" | nc -U /tmp/sherlock_daemon.socket`

