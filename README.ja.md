# crop-hypr

Hyprland向けに作られた、高速なRust製スクリーンショットツール。

## 特徴

- **即時キャプチャ**: 範囲、アクティブウィンドウ、フォーカス中モニター、全モニターを撮影
- **Portalキャプチャ**: xdg-desktop-portal のソースピッカー経由で任意のウィンドウ/モニターを選択
- **フリーズモード**: 画面を凍結し、オーバーレイUIで対話的に撮影対象を選択（Windowsの Win+Shift+S に近い操作感）
- `wl-copy` によるクリップボード自動コピー
- 成功/失敗のデスクトップ通知
- 保存先、ファイル名パターン、フリーズツールバーのグリフ・表示位置を設定可能

## 必要要件

以下のツールが `$PATH` から実行できる状態が必要です。

| ツール | 用途 |
| ---- | ---- |
| `slurp` | 範囲選択（cropモード） |
| `wl-copy` | Waylandクリップボードへ画像をコピー |
| `notify-send` | デスクトップ通知（任意） |

画面キャプチャは **`zwlr_screencopy_manager_v1`** Waylandプロトコルでネイティブに実行されます（Hyprland / sway など wlroots 系コンポジター対応）。

ウィンドウ・モニター情報は **Hyprland IPCソケット**
（`$XDG_RUNTIME_DIR/hypr/<sig>/.socket.sock`）から直接取得します。

> [!CAUTION]
> フリーズモードのデフォルトグリフ表示には [Nerd Font](https://www.nerdfonts.com/) が必要です。アイコンは設定ファイルで変更できます。[設定](#設定)を参照してください。

## インストール

```sh
cargo build --release
cp target/release/crop-hypr ~/.local/bin/
```

## 使い方

```sh
crop-hypr [--config <FILE>] <SUBCOMMAND>
```

| サブコマンド | 説明 |
| ---------- | ---- |
| `crop` | `slurp` で範囲選択して撮影 |
| `window` | アクティブウィンドウを撮影（Hyprland IPCのジオメトリを使用） |
| `portal` | xdg-desktop-portal のソースピッカーで選択して撮影 |
| `monitor` | フォーカス中モニターを撮影 |
| `all` | 全モニターを撮影 |
| `freeze` | 画面を凍結して対話的に選択 |
| `generate-config` | デフォルト設定ファイルを出力 |

### グローバルオプション

`--config <FILE>` / `-c <FILE>`: 既定パスではなく任意の設定ファイルを読み込みます。
すべてのサブコマンド（`generate-config` 含む）で利用できます。

```sh
crop-hypr --config ~/.config/crop-hypr/work.toml freeze
```

### フリーズモード

フリーズモードでは画面全体にオーバーレイを表示し、ツールバーから撮影方式を切り替えられます。

![bar-image](./bar.png)

| モード | 動作 |
| ---- | ---- |
| Crop | ドラッグで任意矩形を作成 |
| Window | ウィンドウにホバーしてクリック |
| Monitor | モニターにホバーしてクリック |
| All | 画面全体を即時撮影 |
| Close | キャンセル（Escapeと同じ） |

アイコングリフは設定ファイルで変更可能です。[設定](#設定)を参照してください。

**キーボード:** `Escape` でキャンセル終了。

### Hyprland キーバインド例

```ini
# ~/.config/hypr/hyprland.conf
bind = , Print,       exec, crop-hypr freeze
bind = SHIFT, Print,  exec, crop-hypr crop
bind = CTRL, Print,   exec, crop-hypr window
```

## 設定

設定ファイルの既定場所: `~/.config/crop-hypr/config.toml`

デフォルト設定は以下で生成できます。

```sh
crop-hypr generate-config
# 既存ファイルを上書きする場合:
crop-hypr generate-config --force
# 任意パスへ出力する場合:
crop-hypr --config ~/my-config.toml generate-config
```

### 設定サンプル

```toml
# スクリーンショットを保存するディレクトリ
# 既定値: ~/Screenshots
save_path = "~/Pictures/Screenshots"

# strftimeパターンで指定するファイル名（拡張子なし .pngは自動付与されます）
# 既定値: "hyprsnap_%Y%m%d_%H%M%S"
filename_pattern = "screenshot_%Y-%m-%d_%H-%M-%S"

# フリーズモードのツールバーを表示する画面の端。
# 選択肢: "top" | "bottom" | "left" | "right"  (既定値: "top")
toolbar_position = "top"

# フリーズモードのツールバーに表示されるグリフ。
# 既定値はNerd Fontが必要です。必要に応じて個別のアイコンを上書きしてください。
[freeze_glyphs]
crop    = "󰆟"
window  = ""
monitor = "󰍹"
all     = "󰁌"
cancel  = "󰖭"
```

### 設定項目リファレンス

| キー | 型 | 既定値 | 説明 |
| --- | --- | --- | --- |
| `save_path` | path | `~/Pictures/Screenshots` | 保存先ディレクトリ |
| `filename_pattern` | string | `hyprsnap_%Y%m%d_%H%M%S` | ファイル名のstrftimeパターン（拡張子なし） |
| `toolbar_position` | string | `top` | フリーズツールバーの表示位置: `top`, `bottom`, `left`, `right` |
| `freeze_glyphs.crop` | string | `󰆟` (U+F019F) | cropモードのアイコン |
| `freeze_glyphs.window` | string | `` (U+EB7F) | windowモードのアイコン |
| `freeze_glyphs.monitor` | string | `󰍹` (U+F0379) | monitorモードのアイコン |
| `freeze_glyphs.all` | string | `󰁌` (U+F004C) | allモードのアイコン |
| `freeze_glyphs.cancel` | string | `󰖭` (U+F05AD) | cancelボタンのアイコン |

## ライセンス

[MIT](./LICENSE)
