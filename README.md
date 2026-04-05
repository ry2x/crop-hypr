# crop-hypr

A fast, Hyprland-native screenshot tool written in Rust.

## Features

- **Immediate capture** — crop region, active window, focused monitor, or all monitors
- **Portal capture** — capture the active window via xdg-desktop-portal, for transparent windows *(Not yet implemented)*
- **Freeze mode** — freeze the screen and interactively select what to capture via an overlay UI (similar to Windows Win+Shift+S Clipping Tool)
- Automatic clipboard copy via `wl-copy`
- Desktop notification on success/failure
- Configurable save path, filename pattern, and freeze toolbar glyphs
- `generate-config` command to scaffold a default config file

## Requirements

The following tools must be available on `$PATH`:

| Tool | Purpose |
| ---- | ------- |
| `slurp` | Interactive region selection (crop mode) |
| `wl-copy` | Copy image to Wayland clipboard |
| `notify-send` | Desktop notifications (optional) |

Screen capture is performed natively via the **`zwlr_screencopy_manager_v1`** Wayland protocol
(wlroots-based compositors: Hyprland, sway, etc.) — no external capture tool is required.

Window and monitor metadata is fetched directly via the **Hyprland IPC socket**
(`$XDG_RUNTIME_DIR/hypr/<sig>/.socket.sock`).

> [!CAUTION]
> A [Nerd Font](https://www.nerdfonts.com/) is required to display default glyphs in the freeze mode toolbar. Check the [configuration section](#configuration) for details on customizing icons.

## Installation

```sh
cargo build --release
cp target/release/crop-hypr ~/.local/bin/
```

## Usage

```sh
crop-hypr [--config <FILE>] <SUBCOMMAND>
```

| Subcommand | Description |
| ---------- | ----------- |
| `crop` | Select a region with slurp and capture it |
| `window` | Capture the active window (geometry via Hyprland IPC) |
| `portal` | Capture the active window via xdg-desktop-portal *(not yet implemented)* |
| `monitor` | Capture the focused monitor |
| `all` | Capture all monitors |
| `freeze` | Freeze screen and select interactively |
| `generate-config` | Write a default config file |

### Global flag

`--config <FILE>` / `-c <FILE>` — Load config from a custom path instead of the default.
Works with every subcommand, including `generate-config`.

```sh
crop-hypr --config ~/.config/crop-hypr/work.toml freeze
```

### Freeze mode

Freeze mode overlays the screen and lets you switch capture type via a toolbar:

![bar-image](./bar.png)

| Mode | Behaviour |
| ---- | --------- |
| Crop | Drag to draw a custom rectangle |
| Window | Hover and click a window |
| Monitor | Hover and click a monitor |
| All | Capture everything instantly |
| Close | Cancel (same as Escape) |

Icon glyphs can be customized in the config file. Check the [configuration section](#configuration) for details.

**Keyboard:** `Escape` cancels and exits.

### Hyprland keybind example

```ini
# ~/.config/hypr/hyprland.conf
bind = , Print,       exec, crop-hypr freeze
bind = SHIFT, Print,  exec, crop-hypr crop
bind = CTRL, Print,   exec, crop-hypr window
```

## Configuration

Config file location: `~/.config/crop-hypr/config.toml`

Generate a default config with:

```sh
crop-hypr generate-config
# Already exists? Use --force to overwrite:
crop-hypr generate-config --force
# Write to a custom path:
crop-hypr --config ~/my-config.toml generate-config
```

### Sample config

```toml
# Directory where screenshots are saved.
# Default: ~/Screenshots
save_path = "~/Pictures/Screenshots"

# strftime-style filename template (no extension — .png is appended automatically).
# Default: "hyprsnap_%Y%m%d_%H%M%S"
filename_pattern = "screenshot_%Y-%m-%d_%H-%M-%S"

# Glyphs shown in the freeze mode toolbar.
# Requires a Nerd Font. Override individual icons as needed.
[freeze_glyphs]
crop    = "󰆟"
window  = ""
monitor = "󰍹"
all     = "󰁌"
cancel  = "󰖭"
```

### Config reference

| Key | Type | Default | Description |
| --- | ---- | ------- | ----------- |
| `save_path` | path | `~/Pictures/Screenshots` | Destination directory for saved screenshots |
| `filename_pattern` | string | `hyprsnap_%Y%m%d_%H%M%S` | strftime pattern for filenames (no extension) |
| `freeze_glyphs.crop` | string | `󰆟` (U+F019F) | Toolbar icon for crop mode |
| `freeze_glyphs.window` | string | `` (U+EB7F) | Toolbar icon for window mode |
| `freeze_glyphs.monitor` | string | `󰍹` (U+F0379) | Toolbar icon for monitor mode |
| `freeze_glyphs.all` | string | `󰁌` (U+F004C) | Toolbar icon for all-monitors mode |
| `freeze_glyphs.cancel` | string | `󰖭` (U+F05AD) | Toolbar icon for cancel button |

## License

MIT
