# crop-hypr

A fast, Hyprland-native screenshot tool written in Rust.

## Features

- **Immediate capture** — crop region, active window, focused monitor, or all monitors
- **Freeze mode** — freeze the screen and interactively select what to capture via an overlay UI (similar to Windows Win+Shift+S)
- Automatic clipboard copy via `wl-copy`
- Desktop notification on success/failure
- Configurable save path and filename pattern

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

A [Nerd Font](https://www.nerdfonts.com/) is recommended for the freeze mode toolbar icons.

## Installation

```sh
cargo build --release
cp target/release/crop-hypr ~/.local/bin/
```

## Usage

```sh
crop-hypr <SUBCOMMAND>
```

| Subcommand | Description |
| ---------- | ----------- |
| `crop` | Select a region with slurp and capture it |
| `window` | Capture the active window |
| `monitor` | Capture the focused monitor |
| `all` | Capture all monitors |
| `freeze` | Freeze screen and select interactively |

### Freeze mode

Freeze mode overlays the screen and lets you switch capture type via a toolbar:

![](./bar.png)

| Icon | Mode | Behaviour |
| ---- | ---- | --------- |
| 󰆟 | Crop | Drag to draw a custom rectangle |
|  | Window | Hover and click a window |
| 󰍹 | Monitor | Hover and click a monitor |
| 󰁌 | All | Capture everything instantly |
| 󰖭 | Close | Cancel (same as Escape) |

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

The file is created with defaults on first run if absent.

### Sample config

```toml
# Directory where screenshots are saved.
# Default: ~/Pictures/Screenshots
save_path = "~/Pictures/Screenshots"

# strftime-style filename template (no extension — .png is appended automatically).
# Default: "hyprsnap_%Y%m%d_%H%M%S"
filename_pattern = "screenshot_%Y-%m-%d_%H-%M-%S"

# How individual windows are captured.
# "geometry" — use Hyprland IPC to get window coordinates, then capture via Wayland screencopy (default)
# "portal"   — xdg-desktop-portal based capture (TODO: not yet implemented)
window_capture_method = "geometry"
```

### Config reference

| Key | Type | Default | Description |
| --- | ---- | ------- | ----------- |
| `save_path` | path | `~/Pictures/Screenshots` | Destination directory for saved screenshots |
| `filename_pattern` | string | `hyprsnap_%Y%m%d_%H%M%S` | strftime pattern for filenames (no extension) |
| `window_capture_method` | `"geometry"` \| `"portal"` | `"geometry"` | Window capture backend |

## License

MIT
