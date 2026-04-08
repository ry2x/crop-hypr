# crop-hypr

A fast, Hyprland-native screenshot tool written in Rust.

日本語版README: [README.ja.md](./README.ja.md)

## Features

- **Immediate capture** — crop region, active window, focused monitor, or all monitors
- **Portal capture** — select any window or monitor via xdg-desktop-portal's WM source-picker
- **Freeze mode** — freeze the screen and interactively select what to capture via an overlay UI (similar to Windows Win+Shift+S Clipping Tool)
- Automatic clipboard copy via `wl-copy`
- Desktop notification on success/failure
- Configurable save path, filename pattern, freeze toolbar glyphs, toolbar position, window border inclusion, and full UI color theming

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
# capture_window_border = false

# Glyphs shown in the freeze mode toolbar.
# Requires a Nerd Font. Override individual icons as needed.
[freeze_glyphs]
crop    = "󰆟"
window  = ""
monitor = "󰍹"
all     = "󰁌"
cancel  = "󰖭"

# ── Freeze mode UI colors ─────────────────────────────────────────────────────
# All colors are [red, green, blue, alpha] float arrays with values in [0.0, 1.0].
# Every key is optional; omitted keys fall back to the built-in defaults shown below.

# [freeze_colors.overlay]
# background = [0.0, 0.0, 0.0, 0.35]     # dim over frozen screen

# [freeze_colors.toolbar]
# background = [0.08, 0.08, 0.08, 0.85]  # toolbar pill background

# [freeze_colors.button]
# idle_background   = [0.20, 0.20, 0.20, 1.0]
# idle_text         = [0.90, 0.90, 0.90, 1.0]
# active_background = [0.345, 0.396, 0.949, 1.0]
# active_text       = [1.0, 1.0, 1.0, 1.0]
# hover_background  = [0.42, 0.475, 0.961, 1.0]
# hover_text        = [1.0, 1.0, 1.0, 1.0]

# [freeze_colors.cancel_button]
# idle_background  = [0.765, 0.259, 0.247, 1.0]
# idle_text        = [1.0, 1.0, 1.0, 1.0]
# hover_background = [0.831, 0.290, 0.278, 1.0]
# hover_text       = [1.0, 1.0, 1.0, 1.0]

# [freeze_colors.window_frame]
# fill_idle      = [0.27, 0.52, 1.0, 0.20]
# fill_hovered   = [0.27, 0.52, 1.0, 0.55]
# stroke_idle    = [0.3, 0.6, 1.0, 0.70]
# stroke_hovered = [0.3, 0.6, 1.0, 1.0]
# label_text     = [1.0, 1.0, 1.0, 1.0]
# hint_text      = [0.8, 0.9, 1.0, 0.9]  # "Click to capture"

# [freeze_colors.monitor_frame]
# fill_idle      = [0.27, 0.52, 1.0, 0.08]
# fill_hovered   = [0.27, 0.52, 1.0, 0.40]
# stroke_idle    = [0.3, 0.6, 1.0, 0.35]
# stroke_hovered = [0.3, 0.6, 1.0, 1.0]
# label_text     = [1.0, 1.0, 1.0, 1.0]
# hint_text      = [0.8, 0.9, 1.0, 0.9]
# name_text_idle = [1.0, 1.0, 1.0, 0.5]  # monitor name when not hovered

# [freeze_colors.crop_frame]
# stroke     = [1.0, 1.0, 1.0, 1.0]
# label_text = [1.0, 1.0, 1.0, 1.0]      # "W × H" size label
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
| `freeze_colors.overlay.background` | [f32;4] | `[0.0, 0.0, 0.0, 0.35]` | Dim fill over frozen screen |
| `freeze_colors.toolbar.background` | [f32;4] | `[0.08, 0.08, 0.08, 0.85]` | Toolbar pill background |
| `freeze_colors.button.idle_background` | [f32;4] | `[0.20, 0.20, 0.20, 1.0]` | Mode button — unselected background |
| `freeze_colors.button.idle_text` | [f32;4] | `[0.90, 0.90, 0.90, 1.0]` | Mode button — unselected text |
| `freeze_colors.button.active_background` | [f32;4] | `[0.345, 0.396, 0.949, 1.0]` | Mode button — selected background |
| `freeze_colors.button.active_text` | [f32;4] | `[1.0, 1.0, 1.0, 1.0]` | Mode button — selected text |
| `freeze_colors.button.hover_background` | [f32;4] | `[0.42, 0.475, 0.961, 1.0]` | Mode button — hover background |
| `freeze_colors.button.hover_text` | [f32;4] | `[1.0, 1.0, 1.0, 1.0]` | Mode button — hover text |
| `freeze_colors.cancel_button.idle_background` | [f32;4] | `[0.765, 0.259, 0.247, 1.0]` | Cancel button — normal background |
| `freeze_colors.cancel_button.idle_text` | [f32;4] | `[1.0, 1.0, 1.0, 1.0]` | Cancel button — normal text |
| `freeze_colors.cancel_button.hover_background` | [f32;4] | `[0.831, 0.290, 0.278, 1.0]` | Cancel button — hover background |
| `freeze_colors.cancel_button.hover_text` | [f32;4] | `[1.0, 1.0, 1.0, 1.0]` | Cancel button — hover text |
| `freeze_colors.window_frame.fill_idle` | [f32;4] | `[0.27, 0.52, 1.0, 0.20]` | Window highlight fill (not hovered) |
| `freeze_colors.window_frame.fill_hovered` | [f32;4] | `[0.27, 0.52, 1.0, 0.55]` | Window highlight fill (hovered) |
| `freeze_colors.window_frame.stroke_idle` | [f32;4] | `[0.3, 0.6, 1.0, 0.70]` | Window highlight outline (not hovered) |
| `freeze_colors.window_frame.stroke_hovered` | [f32;4] | `[0.3, 0.6, 1.0, 1.0]` | Window highlight outline (hovered) |
| `freeze_colors.window_frame.label_text` | [f32;4] | `[1.0, 1.0, 1.0, 1.0]` | Window title text (hovered) |
| `freeze_colors.window_frame.hint_text` | [f32;4] | `[0.8, 0.9, 1.0, 0.9]` | "Click to capture" hint (hovered) |
| `freeze_colors.monitor_frame.fill_idle` | [f32;4] | `[0.27, 0.52, 1.0, 0.08]` | Monitor highlight fill (not hovered) |
| `freeze_colors.monitor_frame.fill_hovered` | [f32;4] | `[0.27, 0.52, 1.0, 0.40]` | Monitor highlight fill (hovered) |
| `freeze_colors.monitor_frame.stroke_idle` | [f32;4] | `[0.3, 0.6, 1.0, 0.35]` | Monitor highlight outline (not hovered) |
| `freeze_colors.monitor_frame.stroke_hovered` | [f32;4] | `[0.3, 0.6, 1.0, 1.0]` | Monitor highlight outline (hovered) |
| `freeze_colors.monitor_frame.label_text` | [f32;4] | `[1.0, 1.0, 1.0, 1.0]` | Monitor name text (hovered) |
| `freeze_colors.monitor_frame.hint_text` | [f32;4] | `[0.8, 0.9, 1.0, 0.9]` | "Click to capture" hint (hovered) |
| `freeze_colors.monitor_frame.name_text_idle` | [f32;4] | `[1.0, 1.0, 1.0, 0.5]` | Monitor name text (not hovered) |
| `freeze_colors.crop_frame.stroke` | [f32;4] | `[1.0, 1.0, 1.0, 1.0]` | Crop rubber-band outline |
| `freeze_colors.crop_frame.label_text` | [f32;4] | `[1.0, 1.0, 1.0, 1.0]` | "W × H" size label in crop mode |

## License

MIT
