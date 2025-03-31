# Getting Started

## 1. Dependencies

To run the Sherlock Launcher, ensure the following dependencies are installed:

- `gtk4` - [Gtk4 Documentation](https://docs.gtk.org/gtk4/)
- `gtk-4-layer-shell` - [Gtk4 Layer Shell](https://github.com/wmww/gtk4-layer-shell)

Additionally, if you're building from source, you will need:

- `rust` - [How to install rust](https://www.rust-lang.org/tools/install)
- `git` - [How to install git](https://github.com/git-guides/install-git)
<br><br>
## 2. Installation

### <ins>Arch Linux</ins>

If you're using Arch Linux, you can install the pre-built binary package with the following command:

```bash
yay -S sherlock-launcher-bin
```

### <ins>From Source</ins>

To build Sherlock Launcher from source, follow these steps.<br>
Make sure you have the necessary dependencies installed:

- `rust` - [How to install rust](https://www.rust-lang.org/tools/install)
- `git` - [How to install git](https://github.com/git-guides/install-git)
- `gtk4` - [Gtk4 Documentation](https://docs.gtk.org/gtk4/)
- `gtk-4-layer-shell` - [Gtk4 Layer Shell](https://github.com/wmww/gtk4-layer-shell)

1. **Clone the repository**:

    ```bash
    git clone https://github.com/skxxtz/sherlock.git
    cd sherlock
    ```

2. **Install necessary Rust dependencies**:

    Build the project using the following command:

    ```bash
    cargo build --release
    ```

3. **Install the binary**:

    After the build completes, install the binary to your system:

    ```bash
    sudo cp target/release/sherlock /usr/bin/
    ```


### <ins>Build Debian Package</ins>

To build a `.deb` package directly from the source, follow these steps:<br>
Make sure you have the following dependencies installed:
- `rust` - [How to install rust](https://www.rust-lang.org/tools/install)
- `git` - [How to install git](https://github.com/git-guides/install-git)
- `gtk4` - [Gtk4 Documentation](https://docs.gtk.org/gtk4/)
- `gtk-4-layer-shell` - [Gtk4 Layer Shell](https://github.com/wmww/gtk4-layer-shell)

1. **Install the `cargo-deb` tool**:

    First, you need to install the `cargo-deb` tool, which simplifies packaging Rust projects as Debian packages:

    ```bash
    cargo install cargo-deb
    ```

2. **Build the Debian package**:

    After installing `cargo-deb`, run the following command to build the `.deb` package:

    ```bash
    cargo deb
    ```

    This will create a `.deb` package in the `target/debian` directory.

3. **Install the generated `.deb` package**:

    Once the package is built, you can install it using:

    ```bash
    sudo dpkg -i target/debian/sherlock-launcher_0.1.5_amd64.deb
    ```

    (Make sure to replace the filename if the version number is different.)
<br><br>
# 3. Post Installation

### **Config Setup**
After the installation is completed, you can set up your configuration files. The location for them is `~/.config/sherlock/`. Depending on your needs, you should add the following files:

1. [**config.toml**](https://github.com/Skxxtz/sherlock/blob/main/docs/examples/config.toml): This file specifies the behavior and defaults of your launcher. Documentation [here](https://github.com/Skxxtz/sherlock/blob/main/docs/config.md).
2. [**fallback.json**](https://github.com/Skxxtz/sherlock/blob/main/docs/examples/fallback.json): This file specifies the features your launcher should have. Documentation [here](https://github.com/Skxxtz/sherlock/blob/main/docs/launchers.md).
3. [**sherlock_alias.json**](https://github.com/Skxxtz/sherlock/blob/main/docs/examples/sherlock_alias.json): This file specifies aliases for applications. Documentation [here](https://github.com/Skxxtz/sherlock/blob/main/docs/aliases.md).
4. [**sherlockignore**](https://github.com/Skxxtz/sherlock/blob/main/docs/examples/sherlockignore): This file specifies which applications to exclude from your search. Documentation [here](https://github.com/Skxxtz/sherlock/blob/main/docs/sherlockignore.md).

```bash
mkdir -p ~/.config/sherlock/
touch ~/.config/sherlock/config.toml ~/.config/sherlock/sherlockignore
echo [] > ~/.config/sherlock/fallback.json
echo {} > ~/.config/sherlock/sherlock_alias.json
```
<br><br>
**Alternatively**, you can use `curl -O` in the `~/.config/sherlock/` directory to load the [example configs](https://github.com/Skxxtz/sherlock/tree/main/docs/examples). This is **not good practice** and **not recommended**, especially if you don't know the source, as you could end up downloading **malware**! Nevertheless, here's how you can do that, if you prefer to be a risk-taker:

> **🚨 Warning:** Only use `curl` with trusted sources to avoid downloading malicious content!
```bash
mkdir -p ~/.config/sherlock/
cd ~/.config/sherlock/
curl -O https://raw.githubusercontent.com/skxxtz/sherlock/main/docs/examples/config.toml
curl -O https://raw.githubusercontent.com/skxxtz/sherlock/main/docs/examples/sherlockignore
curl -O https://raw.githubusercontent.com/skxxtz/sherlock/main/docs/examples/fallback.json
curl -O https://raw.githubusercontent.com/skxxtz/sherlock/main/docs/examples/sherlock_alias.json
```
### Warnings after startup
If you're getting warnings after startup, you can press `return` to access the main application. Alternatively you can set the `try_suppress_warnings` key in the config file to true. This will prevent any warnings to be shown. The same thing can be done for errors. However, if you suppress errors, the application might not work as expected.

### **Keybind Setup**
To launch Sherlock, you can either type `sherlock` into the command line or bind it to a key. The latter is recommended.
The setup steps vary by display manager.<br>

The setup for **Hyprland** is outlined here:

1. (Recommended) Bind the `$menu` variable to Sherlock:
```conf
$menu = sherlock
```
2. Bind a key to execute `$menu`
```conf
bind = $mainMod, space, exec, $menu
```
