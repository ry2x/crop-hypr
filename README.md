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
# All colors are CSS-style hex strings: "#RRGGBBAA" (or "#RRGGBB", "#RGBA", "#RGB").
# Every key is optional; omitted keys fall back to the built-in defaults shown below.

# [freeze_colors.overlay]
# background = "#00000059"     # dim over frozen screen

# [freeze_colors.toolbar]
# background = "#141414D9"  # toolbar pill background

# [freeze_colors.button]
# idle_background   = "#797A7DFF"
# idle_text         = "#E6E6E6FF"
# active_background = "#5865F2FF"
# active_text       = "#FFFFFFFF"
# hover_background  = "#6B79F5FF"
# hover_text        = "#FFFFFFFF"

# [freeze_colors.cancel_button]
# idle_background  = "#C3423FFF"
# idle_text        = "#FFFFFFFF"
# hover_background = "#D44A47FF"
# hover_text       = "#FFFFFFFF"

# [freeze_colors.window_frame]
# fill_idle      = "#4585FF33"
# fill_hovered   = "#4585FF8C"
# stroke_idle    = "#4D99FFB3"
# stroke_hovered = "#4D99FFFF"
# label_text     = "#FFFFFFFF"
# hint_text      = "#CCE6FFE6"  # "Click to capture"

# [freeze_colors.monitor_frame]
# fill_idle      = "#4585FF14"
# fill_hovered   = "#4585FF66"
# stroke_idle    = "#4D99FF59"
# stroke_hovered = "#4D99FFFF"
# label_text     = "#FFFFFFFF"
# hint_text      = "#CCE6FFE6"  # "Click to capture"
# name_text_idle = "#FFFFFF80"  # monitor name when not hovered  # monitor name when not hovered

# [freeze_colors.crop_frame]
# stroke     = "#FFFFFFFF"
# label_text = "#FFFFFFFF"      # "W × H" size label
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
| `freeze_colors.overlay.background` | string (hex) | `"#00000059"` | Dim fill over frozen screen |
| `freeze_colors.toolbar.background` | string (hex) | `"#141414D9"` | Toolbar pill background |
| `freeze_colors.button.idle_background` | string (hex) | `"#797A7DFF"` | Mode button — unselected background |
| `freeze_colors.button.idle_text` | string (hex) | `"#E6E6E6FF"` | Mode button — unselected text |
| `freeze_colors.button.active_background` | string (hex) | `"#5865F2FF"` | Mode button — selected background |
| `freeze_colors.button.active_text` | string (hex) | `"#FFFFFFFF"` | Mode button — selected text |
| `freeze_colors.button.hover_background` | string (hex) | `"#6B79F5FF"` | Mode button — hover background |
| `freeze_colors.button.hover_text` | string (hex) | `"#FFFFFFFF"` | Mode button — hover text |
| `freeze_colors.cancel_button.idle_background` | string (hex) | `"#C3423FFF"` | Cancel button — normal background |
| `freeze_colors.cancel_button.idle_text` | string (hex) | `"#FFFFFFFF"` | Cancel button — normal text |
| `freeze_colors.cancel_button.hover_background` | string (hex) | `"#D44A47FF"` | Cancel button — hover background |
| `freeze_colors.cancel_button.hover_text` | string (hex) | `"#FFFFFFFF"` | Cancel button — hover text |
| `freeze_colors.window_frame.fill_idle` | string (hex) | `"#4585FF33"` | Window highlight fill (not hovered) |
| `freeze_colors.window_frame.fill_hovered` | string (hex) | `"#4585FF8C"` | Window highlight fill (hovered) |
| `freeze_colors.window_frame.stroke_idle` | string (hex) | `"#4D99FFB3"` | Window highlight outline (not hovered) |
| `freeze_colors.window_frame.stroke_hovered` | string (hex) | `"#4D99FFFF"` | Window highlight outline (hovered) |
| `freeze_colors.window_frame.label_text` | string (hex) | `"#FFFFFFFF"` | Window title text (hovered) |
| `freeze_colors.window_frame.hint_text` | string (hex) | `"#CCE6FFE6"` | "Click to capture" hint (hovered) |
| `freeze_colors.monitor_frame.fill_idle` | string (hex) | `"#4585FF14"` | Monitor highlight fill (not hovered) |
| `freeze_colors.monitor_frame.fill_hovered` | string (hex) | `"#4585FF66"` | Monitor highlight fill (hovered) |
| `freeze_colors.monitor_frame.stroke_idle` | string (hex) | `"#4D99FF59"` | Monitor highlight outline (not hovered) |
| `freeze_colors.monitor_frame.stroke_hovered` | string (hex) | `"#4D99FFFF"` | Monitor highlight outline (hovered) |
| `freeze_colors.monitor_frame.label_text` | string (hex) | `"#FFFFFFFF"` | Monitor name text (hovered) |
| `freeze_colors.monitor_frame.hint_text` | string (hex) | `"#CCE6FFE6"` | "Click to capture" hint (hovered) |
| `freeze_colors.monitor_frame.name_text_idle` | string (hex) | `"#FFFFFF80"` | Monitor name text (not hovered) |
| `freeze_colors.crop_frame.stroke` | string (hex) | `"#FFFFFFFF"` | Crop rubber-band outline |
| `freeze_colors.crop_frame.label_text` | string (hex) | `"#FFFFFFFF"` | "W × H" size label in crop mode |

## License

MIT
