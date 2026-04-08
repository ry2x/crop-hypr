# crop-hypr

A fast, Hyprland-native screenshot tool written in Rust.

日本語版README: [README.ja.md](./README.ja.md)

## Features

- **Immediate capture** — crop region, active window, focused monitor, or all monitors
- **Portal capture** — select any window or monitor via xdg-desktop-portal's WM source-picker
- **Freeze mode** — freeze the screen and interactively select what to capture via an overlay UI (similar to Windows Win+Shift+S Clipping Tool)
- Automatic clipboard copy via `wl-copy`
- Desktop notification on success/failure
- Configurable save path, filename pattern, freeze toolbar glyphs, toolbar position, and window border inclusion

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
git clone https://github.com/ry2x/crop-hypr.git
cd crop-hypr
cargo build --release
cp target/release/crop-hypr ~/.local/bin/
```

### For Arch Linux users

A PKGBUILD is included for building an Arch package.

```sh
git clone https://github.com/ry2x/crop-hypr.git
cd crop-hypr
makepkg -si
```

## Usage

```sh
crop-hypr [--config <FILE>] <SUBCOMMAND>
```

| Subcommand | Description |
| ---------- | ----------- |
| `crop` | Select a region with slurp and capture it |
| `window` | Capture the active window (geometry via Hyprland IPC) |
| `portal` | Capture a selected window or monitor via xdg-desktop-portal (shows WM source-picker) |
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
bindd = SUPER, S, ScreenshotMonitor,    exec, crop-hypr monitor
bindd = SUPER SHIFT, S, FreezeMode,     exec, crop-hypr freeze
bindd = , Print, ScreenshotFull,        exec, crop-hypr all
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

# Edge of the screen where the freeze mode toolbar is docked.
# Options: "top" | "bottom" | "left" | "right"  (default: "top")
toolbar_position = "top"

# When true, window captures (both immediate `window` and freeze-mode Window
# selection) include the Hyprland border, expanding the crop area by
# `general:border_size` on each side. The freeze-mode overlay also draws
# rounded highlight frames matching `decoration:rounding`.
# Default: false
capture_window_border = true

# Glyphs shown in the freeze mode toolbar.
# Requires a Nerd Font. Override individual icons as needed.
[freeze_glyphs]
crop    = "󰆟"
window  = ""
monitor = "󰍹"
all     = "󰁌"
cancel  = "󰖭"
```

### Config reference

| Key | Type | Default | Description |
| --- | ---- | ------- | ----------- |
| `save_path` | path | XDG Pictures dir + `/Screenshots` (fallback: `$HOME/Screenshots`) | Destination directory for saved screenshots |
| `filename_pattern` | string | `hyprsnap_%Y%m%d_%H%M%S` | strftime pattern for filenames (no extension) |
| `toolbar_position` | string | `top` | Freeze toolbar edge: `top`, `bottom`, `left`, or `right` |
| `capture_window_border` | bool | `false` | Include Hyprland window border in window captures; also draws rounded highlights in freeze mode |
| `freeze_glyphs.crop` | string | `󰆟` (U+F019F) | Toolbar icon for crop mode |
| `freeze_glyphs.window` | string | `` (U+EB7F) | Toolbar icon for window mode |
| `freeze_glyphs.monitor` | string | `󰍹` (U+F0379) | Toolbar icon for monitor mode |
| `freeze_glyphs.all` | string | `󰁌` (U+F004C) | Toolbar icon for all-monitors mode |
| `freeze_glyphs.cancel` | string | `󰖭` (U+F05AD) | Toolbar icon for cancel button |

## License

MIT
