# Hyprsnap (Rust) — Hyprland向け高機能スクリーンショットツール

このドキュメントは、Hyprland環境で動作するCLI型スクリーンショットツール（Rust版）の仕様・依存関係・実装ガイドラインをまとめたものです。

---

## 1. 動作環境・依存関係

- OS: Arch Linux（Hyprland環境を想定）
- 言語: Rust

必須ツール・ライブラリ（Backend）:

- `slurp` — 画面上の領域選択
- `wl-clipboard` — クリップボード連携（`wl-copy` 等）
- `hyprctl` — ウィンドウ・モニター情報の取得
- `libnotify`（`notify-send`）— 通知の送信
- `ashpd`（Rust crate）— xdg-desktop-portal 経由の撮影（必要時）
- Wayland screencopy: `zwlr_screencopy_manager_v1` プロトコル — 外部ツール不要でネイティブキャプチャ（wlroots系コンポジター対応）
- UI ライブラリ例: `iced_layershell` 等（Wayland の layer-shell 対応、フリーズモードの選択UI用）

## 2. 核心機能（Features）

### A. 呼び出しモード

#### 即時撮影モード（Immediate）

- `window` — 現在アクティブなウィンドウを撮影
    - 設定（TOML）により `xdg-desktop-portal`（portal）を使うか、`hyprctl` で座標を取得して Wayland screencopy で撮影するかを切り替え可能
- `monitor` — 現在のモニターを即撮影
- `all` — 全モニターを連結して撮影
- `crop` — `slurp` で選択した範囲を撮影

#### フリーズ撮影モード（Freeze）

Windows の Win+Shift+S に相当する高度な対話型モード。

- ロジック概要:
    1. 実行時に全画面を一時保存する
    2. 保存した画像を全画面にオーバーレイ表示（フリーズ状態）する
    3. 画面上部にモダンなツールバー（選択ボタン）を表示する
    4. オプション: `crop`（デフォルト）, `window`, `monitor`, `all`
    5. `window` 等を選択した場合は `hyprctl` の情報を元にウィンドウ位置を割り出し、クリックまたは領域確定でその部分を切り出す（Portalは使用しない）
    6. 選択座標に基づき、一時画像から最終的な切り出し（Crop）を行う

### B. 設定（Configuration）

- 形式: TOML（例: `~/.config/hyprsnap/config.toml`）
- 主な設定項目:
    - `save_path`: 保存先ディレクトリ
    - `window_capture_method`: `"portal"` または `"geometry"`（`hyprctl` 経由）

### C. 後処理

- 保存: 設定されたパスへ保存
- クリップボード: 撮影成功時に自動で画像をクリップボードへコピー（`wl-copy`）
- 通知: `notify-send` 等で成功/失敗を通知

## 3. 実装ガイドライン（Copilot への指示）

### CLI 設計

- `clap` を用いてサブコマンド設計を行う（`immediate`, `freeze`, など）

### フリーズモードの UI 実装

- `iced_layershell` 等を用いて Hyprland の layer-shell 上に透過／オーバーレイ UI を構築
- 選択ボタンは角丸、ホバーエフェクト等を含むモダンな外観を目指す

### 透過ウィンドウ対策（Portal vs Geometry）

- 設定ファイルの `window_capture_method` を読み取り、ロジックを分岐
- `geometry` モード時の例:
    - `hyprctl -j activewindow` から `at`（座標）と `size` を取得
    - 取得した座標を元に `zwlr_screencopy_manager_v1` 経由でモニターをキャプチャし、ウィンドウ領域を切り出す

## 4. コードスタイル

- `Result` 型による適切なエラーハンドリングを行う
- 外部プロセス呼び出しと D-Bus 通信を明確に分離したモジュール構成にする

---

（補足）このドキュメントは開発者向けの実装方針を示すもので、必要に応じて詳細な設計図やコード例を追加してください。
