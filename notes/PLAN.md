# xmux 実装計画

cmux (manaflow-ai/cmux) の Linux 向け再現実装. Iced + alacritty_terminal + wgpu アーキテクチャ.

---

## 1. Cargo Workspace 構成

```
xmux/
├── Cargo.toml                    # workspace root
├── crates/
│   ├── xmux-core/                # 共有型・設定・エラー型
│   ├── xmux-terminal/            # alacritty_terminal ラッパー + PTY 管理
│   ├── xmux-renderer/            # wgpu テキストレンダリング (グリフキャッシュ, セル描画)
│   ├── xmux-notification/        # OSC パーサー + デスクトップ通知
│   ├── xmux-rpc/                 # Unix ソケット JSON-RPC v2 サーバー
│   ├── xmux-session/             # セッション保存/復元
│   ├── xmux-agent/               # エージェントフック (Claude Code, Codex 等)
│   ├── xmux-browser/             # wry WebView 統合
│   ├── xmux-math/                # RaTeX 数式レンダリング統合
│   └── xmux-platform/            # プラットフォーム抽象レイヤー
├── src/
│   └── main.rs                   # エントリポイント (Iced App)
├── tests/                        # 統合テスト
├── config/                       # デフォルト設定ファイル
└── notes/                        # プロジェクトドキュメント
```

### 各クレートの責務と依存関係

```
xmux-core          ← (依存なし, 他の全クレートが依存)
xmux-platform      ← xmux-core, portable-pty (macOS/Windows のみ, P7)
xmux-terminal      ← xmux-core, alacritty_terminal (tty + EventLoop)
xmux-renderer      ← xmux-core, xmux-terminal, wgpu, glyphon
xmux-notification   ← xmux-core, xmux-terminal, notify-rust
xmux-rpc           ← xmux-core, xmux-terminal, tokio, tokio-util, serde_json
xmux-session       ← xmux-core, xmux-terminal, serde, tokio
xmux-agent         ← xmux-core, xmux-rpc, xmux-notification
xmux-browser       ← xmux-core, wry
xmux-math          ← xmux-core, ratex-parser, ratex-layout, ratex-render
xmux (bin)         ← 全クレート, iced
```

### 各クレートの pub API 概要

**xmux-core**
```rust
// 共有 ID 型
pub struct WorkspaceId(pub Uuid);
pub struct PaneId(pub Uuid);
pub struct SurfaceId(pub Uuid);
pub struct WindowId(pub Uuid);

// 設定
pub struct Config {
    pub font: FontConfig,
    pub colors: ColorScheme,
    pub shell: ShellConfig,
    pub keybindings: Vec<Keybinding>,
    pub scrollback_lines: usize,   // default: 100_000
    pub socket_path: PathBuf,
}

pub struct FontConfig {
    pub family: String,
    pub size: f32,
    pub weight: u16,
    pub bold_is_bright: bool,
}

pub struct ColorScheme {
    pub primary: PrimaryColors,
    pub normal: AnsiColors,
    pub bright: AnsiColors,
    pub cursor: CursorColors,
    pub selection: SelectionColors,
}

// エラー型
pub enum XmuxError {
    Pty(String),
    Terminal(String),
    Io(std::io::Error),
    Config(String),
    Rpc(String),
    Session(String),
}
```

**xmux-platform**
```rust
pub trait PlatformPty: Send + Sync {
    fn spawn(&self, config: &PtyConfig) -> Result<PtyHandle, XmuxError>;
    fn resize(&self, handle: &PtyHandle, size: PtySize) -> Result<(), XmuxError>;
}

pub trait PlatformNotifier: Send + Sync {
    fn send_notification(&self, title: &str, body: &str) -> Result<(), XmuxError>;
}

pub trait PlatformClipboard: Send + Sync {
    fn get_text(&self) -> Result<String, XmuxError>;
    fn set_text(&self, text: &str) -> Result<(), XmuxError>;
}

pub trait PlatformShell: Send + Sync {
    fn default_shell(&self) -> PathBuf;
    fn shell_env(&self) -> HashMap<String, String>;
}
```

**xmux-terminal**
```rust
pub struct Terminal {
    // alacritty_terminal::Term<EventProxy> を内包
    // Arc<FairMutex<Term<EventProxy>>> でスレッドセーフ
}

impl Terminal {
    pub fn new(config: &Config, size: TerminalSize) -> Result<Self, XmuxError>;
    pub fn input(&self, data: &[u8]);
    pub fn resize(&self, size: TerminalSize);
    pub fn renderable_content(&self) -> RenderableContent;
    pub fn scroll(&self, scroll: Scroll);
    pub fn selection_text(&self) -> Option<String>;
    pub fn search(&self, pattern: &str, direction: Direction) -> Option<Match>;
    pub fn damage(&self) -> TermDamage;
    pub fn reset_damage(&self);
    pub fn title(&self) -> String;
    pub fn exit_status(&self) -> Option<ExitStatus>;
}

pub struct Pane {
    pub id: PaneId,
    pub terminal: Terminal,
    pub pty_writer: Box<dyn Write + Send>,
    pub working_dir: PathBuf,
    pub title: String,
}

pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub panes: PaneGrid<Pane>,
    pub active_pane: PaneId,
    pub metadata: WorkspaceMetadata,
}

pub struct WorkspaceMetadata {
    pub git_branch: Option<String>,
    pub pr_status: Option<PrStatus>,
    pub listening_ports: Vec<u16>,
    pub last_notification: Option<String>,
}
```

**xmux-renderer**
```rust
pub struct TerminalRenderer {
    // wgpu デバイス, グリフキャッシュ, パイプライン
}

impl TerminalRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, font: &FontConfig) -> Self;
    pub fn render(&mut self, content: &RenderableContent, viewport: &Viewport) -> Vec<DrawCommand>;
    pub fn update_font(&mut self, font: &FontConfig);
    pub fn resize_viewport(&mut self, size: PhysicalSize);
}
```

**xmux-notification**
```rust
pub struct NotificationManager { /* ... */ }

impl NotificationManager {
    pub fn new() -> Self;
    pub fn parse_osc(&mut self, sequence: &[u8]) -> Option<Notification>;
    pub fn add(&mut self, notification: Notification) -> NotificationId;
    pub fn list(&self) -> &[Notification];
    pub fn clear(&mut self);
    pub fn clear_one(&mut self, id: NotificationId);
}

pub struct Notification {
    pub id: NotificationId,
    pub title: String,
    pub body: String,
    pub source: NotificationSource,
    pub timestamp: SystemTime,
    pub read: bool,
    pub pane_id: PaneId,
}

pub enum NotificationSource {
    Osc9,
    Osc99,
    Osc777,
    Cli,
}
```

**xmux-rpc**
```rust
pub struct RpcServer { /* ... */ }

impl RpcServer {
    pub async fn bind(socket_path: &Path) -> Result<Self, XmuxError>;
    pub async fn run(&self, handler: impl RpcHandler) -> Result<(), XmuxError>;
    pub async fn shutdown(&self);
}

pub trait RpcHandler: Send + Sync + 'static {
    async fn handle(&self, method: &str, params: serde_json::Value)
        -> Result<serde_json::Value, RpcError>;
}

pub struct RpcError {
    pub code: i32,
    pub message: String,
}
```

**xmux-session**
```rust
pub struct SessionManager { /* ... */ }

impl SessionManager {
    pub fn new(data_dir: &Path) -> Self;
    pub async fn save(&self, state: &AppState) -> Result<PathBuf, XmuxError>;
    pub async fn load(&self, path: &Path) -> Result<AppState, XmuxError>;
    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>, XmuxError>;
    pub fn enable_autosave(&self, interval: Duration);
}

pub struct SessionSnapshot {
    pub version: u32,
    pub windows: Vec<WindowSnapshot>,
    pub active_window: WindowId,
    pub timestamp: SystemTime,
}
```

**xmux-agent**
```rust
pub struct AgentRegistry { /* ... */ }

impl AgentRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, agent: AgentConfig);
    pub fn detect_agent(&self, env: &HashMap<String, String>) -> Option<&AgentConfig>;
    pub fn setup_hooks(&self, agent: &str, pane_id: PaneId) -> Result<(), XmuxError>;
}

pub struct AgentConfig {
    pub name: String,              // "claude-code", "codex", etc.
    pub detect_env: String,        // 環境変数名
    pub hook_command: String,      // 通知フック
    pub resume_command: Option<String>,
}
```

---

## 2. プラットフォーム抽象レイヤー

### trait 定義 (再掲・詳細)

```rust
// crates/xmux-platform/src/lib.rs

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;

/// PTY サイズ (行・列・ピクセル)
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

/// PTY 生成設定
pub struct PtyConfig {
    pub shell: PathBuf,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: Option<PathBuf>,
    pub size: PtySize,
}

/// PTY ハンドル: reader/writer + プロセス制御
pub struct PtyHandle {
    pub reader: Box<dyn Read + Send>,
    pub writer: Box<dyn Write + Send>,
    pub child: Box<dyn PtyChild + Send>,
    pub fd: Option<i32>,  // Unix のみ, Windows は None
}

pub trait PtyChild: Send {
    fn try_wait(&mut self) -> Result<Option<ExitStatus>, XmuxError>;
    fn kill(&mut self) -> Result<(), XmuxError>;
    fn pid(&self) -> u32;
}

/// PTY 生成・管理
pub trait PlatformPty: Send + Sync {
    fn spawn(&self, config: &PtyConfig) -> Result<PtyHandle, XmuxError>;
    fn resize(&self, handle: &PtyHandle, size: PtySize) -> Result<(), XmuxError>;
}

/// デスクトップ通知
pub trait PlatformNotifier: Send + Sync {
    fn send_notification(&self, title: &str, body: &str) -> Result<(), XmuxError>;
    fn supports_actions(&self) -> bool;
}

/// クリップボード
pub trait PlatformClipboard: Send + Sync {
    fn get_text(&self) -> Result<String, XmuxError>;
    fn set_text(&self, text: &str) -> Result<(), XmuxError>;
}

/// シェル・環境
pub trait PlatformShell: Send + Sync {
    fn default_shell(&self) -> PathBuf;
    fn shell_env(&self) -> HashMap<String, String>;
    fn config_dir(&self) -> PathBuf;
    fn data_dir(&self) -> PathBuf;
    fn socket_path(&self) -> PathBuf;
}

/// ファクトリ: プラットフォーム固有の実装をまとめて返す
pub fn create_platform() -> Platform {
    #[cfg(target_os = "linux")]
    { Platform::new(LinuxPty, LinuxNotifier, LinuxClipboard, LinuxShell) }
    #[cfg(target_os = "macos")]
    { Platform::new(MacPty, MacNotifier, MacClipboard, MacShell) }
    #[cfg(target_os = "windows")]
    { Platform::new(WindowsPty, WindowsNotifier, WindowsClipboard, WindowsShell) }
}

pub struct Platform {
    pub pty: Box<dyn PlatformPty>,
    pub notifier: Box<dyn PlatformNotifier>,
    pub clipboard: Box<dyn PlatformClipboard>,
    pub shell: Box<dyn PlatformShell>,
}
```

### 各プラットフォームの実装方針

| trait | Linux | macOS | Windows |
|---|---|---|---|
| `PlatformPty` | `alacritty_terminal::tty` 直接使用 (P0–P3) | `portable-pty` の `NativePtySystem` (macOS) (P7) | `portable-pty` の `NativePtySystem` (ConPTY) (P7) |
| `PlatformNotifier` | `notify-rust` (D-Bus / XDG) | `notify-rust` (macOS UNNotification) | `notify-rust` (Win Toast) |
| `PlatformClipboard` | `arboard` crate (X11/Wayland, `wayland-data-control` feature 必須) | `arboard` crate (NSPasteboard) | `arboard` crate (Win32 clipboard) |
| `PlatformShell` | `$SHELL` → fallback `/bin/bash` | `$SHELL` → fallback `/bin/zsh` | `cmd.exe` / `powershell.exe` |
| `config_dir` | `$XDG_CONFIG_HOME/xmux` | `~/Library/Application Support/xmux` | `%APPDATA%/xmux` |
| `data_dir` | `$XDG_DATA_HOME/xmux` | `~/Library/Application Support/xmux` | `%LOCALAPPDATA%/xmux` |
| `socket_path` | `/tmp/xmux.sock` (検出: `$XMUX_SOCKET_PATH`) | `/tmp/xmux.sock` | 名前付きパイプ `\\.\pipe\xmux` |

P0–P3 は Linux 実装のみ. macOS/Windows は P7 以降.

#### PTY 管理方式の設計判断

**[アーキテクチャ決定] Linux では alacritty_terminal::tty + EventLoop を直接使用し, portable-pty は P0–P6 では使わない.**

Sonnet B は portable-pty の完全廃止を提案したが, 以下の理由で **Linux 限定の廃止 + macOS/Windows では portable-pty を維持** する方針を採る:

1. **Linux での portable-pty 廃止の根拠**: alacritty_terminal の EventLoop は内部で `tty::Pty` を直接操作する. portable-pty を経由すると, PTY バイトストリームの受け渡しが二重になり, EventLoop の設計と競合する. iced_term, COSMIC Terminal いずれも portable-pty を使わず alacritty_terminal::tty を直接使用している.
2. **macOS/Windows では portable-pty を維持する根拠**: `alacritty_terminal::tty` は `#[cfg(unix)]` / `#[cfg(windows)]` で分岐しているが, macOS 固有の挙動 (シグナルハンドリング差異) や Windows の ConPTY 対応は portable-pty が吸収するメリットがある. P7 でクロスプラットフォーム移植する際に `PlatformPty` trait の macOS/Windows 実装で portable-pty を使い, Linux 実装は alacritty_terminal::tty のラッパーとする.
3. **`xmux-platform` の `PlatformPty` trait は P0 では使わない**. P7 の macOS/Windows 移植時に, Linux 実装を `PlatformPty` trait に適合させるリファクタリングを行う.

---

## 3. フェーズ分割

### P0: プロジェクト基盤 + 最小ターミナル (5 日)

**目標**: Iced ウィンドウ内で単一ペインのターミナルが動作する. キー入力を送信し, シェルの出力が描画される.

**完了条件**: `cargo run` で Iced ウィンドウが開き, bash/zsh プロンプトが表示され, `ls`, `vim`, `htop` 等のコマンドが正常に動作する. テキスト選択・コピーが可能.

**推定工数**: 5 日

#### タスク

**P0-T1: Cargo workspace 初期化**
- 入力: なし
- 出力: `Cargo.toml` (workspace), `crates/xmux-core/`, `crates/xmux-terminal/`, `crates/xmux-platform/`, `src/main.rs`
- 実装内容:
  - workspace root `Cargo.toml` に全メンバー定義
  - `xmux-core`: `Config`, `XmuxError`, ID 型 (`WorkspaceId`, `PaneId` 等), `FontConfig`, `ColorScheme`
  - `xmux-platform`: trait 定義 (`PlatformPty`, `PlatformClipboard`, `PlatformShell`), Linux 実装スタブ
  - `src/main.rs`: `iced::Application` の空実装 (ウィンドウ表示のみ)
- 検証方法: `cargo build --workspace` が成功. `cargo run` で空の Iced ウィンドウが開く
- 推定規模: **L (~700 行)** (Config の全サブ型定義 — `FontConfig`, `ColorScheme`, `PrimaryColors`, `AnsiColors`, `CursorColors`, `SelectionColors`, `ShellConfig`, `Keybinding` 等 — が予想より大きい)
- 使用 crate: `iced` 0.14, `uuid`, `serde`, `thiserror`, `dirs` **6.0** (最新)
- 注意点:
  - **Iced 0.14 の `application()` シグネチャ**: 第1引数は文字列ではなく boot 関数. 正しくは `iced::application(App::new, App::update, App::view).title(|_| "xmux".into()).run()` 形式.
  - `portable_pty::ExitStatus` と `std::process::ExitStatus` は別型. 変換が必要.
  - `portable_pty::MasterPty::take_writer()` は1回しか呼べない. 呼び出しタイミングに注意.

**P0-T2: PTY 生成と I/O ループ**
- 入力: P0-T1
- 出力: `crates/xmux-terminal/src/pty.rs`
- 実装内容:
  - **設計変更 (API 検証結果)**: `portable-pty` は使わない. `alacritty_terminal::tty` + `EventLoop` を直接使う (iced_term と同パターン. portable-pty は alacritty のデータ変換が二重になる).
  - PTY 生成:
    ```rust
    // tty::new シグネチャ: pub fn new(config: &tty::Options, window_size: WindowSize, window_id: u64) -> Result<Pty>
    let pty = alacritty_terminal::tty::new(&tty_options, window_size, window_id)?;
    ```
  - `tty::Options` フィールド: `shell: Option<Shell>`, `working_directory: Option<PathBuf>`, `drain_on_exit: bool`, `env: HashMap<String, String>`
  - I/O ループ:
    ```rust
    // EventLoop::new: pub fn new(terminal, event_proxy, pty, drain_on_exit, ref_test) -> Result<EventLoop<T, U>>
    // EventLoop::spawn() は std::thread ベース. JoinHandle<(EventLoop<Pty, EventProxy>, State)> を返す
    let event_loop = EventLoop::new(term.clone(), event_proxy.clone(), pty, false, false)?;
    let notifier = event_loop.channel(); // Notifier 取得
    let _handle = event_loop.spawn();
    ```
  - PTY 書き込み: `Notifier::notify<B: Into<Cow<'static, [u8]>>>(bytes)` で書き込む.
  - `PtyManager` struct:
    ```rust
    pub struct PtyManager {
        notifier: Notifier,
        _handle: std::thread::JoinHandle<(EventLoop<Pty, EventProxy>, State)>,
    }
    impl PtyManager {
        pub fn write(&self, data: impl Into<Cow<'static, [u8]>>) {
            self.notifier.notify(data);
        }
    }
    ```
  - `LinuxShell::default_shell()`: `$SHELL` 環境変数 → fallback `/bin/bash`
- 検証方法: `#[test]` (tokio 不要: EventLoop は std::thread ベース) で PTY 生成 → `notifier.notify(b"echo hello\r".as_ref())` → 200ms 待機後 `term.lock().grid()` を検査し "hello" を含む行を assert
- 推定規模: M (~200 行, EventLoop が I/O を担うため削減)
- 使用 crate: `alacritty_terminal` 0.26 (tty, event_loop モジュール)
- 注意点:
  - `EventLoop::spawn()` は `std::thread::spawn` ベース. tokio ランタイムとは独立.
  - `tty::Options::shell: None` は `$SHELL` 環境変数を自動使用. 明示する場合は `alacritty_terminal::tty::Shell { program, args }` を渡す.
  - `window_id: u64` は PTY 識別子. ペインの UUID を u64 にハッシュして使用.
  - `crates/xmux-platform` の `PlatformPty` trait は P0 では使わない. P7 の macOS/Windows 移植時にリファクタリングする.

**P0-T3: alacritty_terminal 統合**
- 入力: P0-T2
- 出力: `crates/xmux-terminal/src/lib.rs`, `crates/xmux-terminal/src/event.rs`
- 実装内容:
  - `EventProxy` struct: `EventListener` trait を実装. 検証済みシグネチャ:
    ```rust
    // EventListener trait: fn send_event(&self, _event: Event) {} (デフォルト空実装あり)
    #[derive(Clone)]
    pub struct EventProxy(mpsc::Sender<Event>); // tokio::sync::mpsc::Sender (bounded)

    impl EventListener for EventProxy {
        fn send_event(&self, event: Event) {
            let _ = self.0.try_send(event); // 非同期送信, backpressure は無視
        }
    }
    ```
  - `Terminal` struct:
    ```rust
    pub struct Terminal {
        term: Arc<FairMutex<Term<EventProxy>>>,  // FairMutex は alacritty_terminal::sync::FairMutex (parking_lot ではない)
        pty: PtyManager,
        event_rx: mpsc::Receiver<Event>,
        pub needs_update: bool,
    }
    ```
  - `Terminal::new()`:
    ```rust
    // Term::new シグネチャ: pub fn new<D: Dimensions>(config: Config, dimensions: &D, event_proxy: T) -> Term<T>
    // Config フィールド: scrolling_history: usize, default_cursor_style: CursorStyle,
    //   vi_mode_cursor_style: Option<CursorStyle>, semantic_escape_chars: String,
    //   kitty_keyboard: bool, osc52: Osc52
    let config = term::Config {
        scrolling_history: 100_000,
        ..Default::default()
    };
    let term = Term::new(config, &size, event_proxy.clone());
    let term = Arc::new(FairMutex::new(term));
    ```
  - PTY I/O は P0-T2 の `EventLoop` が担う. `Terminal::new()` で EventLoop を起動して `PtyManager` を保持.
  - `Terminal::write(data)`: `self.pty.write(data)` → Notifier 経由で PTY に書き込み
  - `Terminal::renderable_content()`: ロック取得 → `RenderableContent` を返す
    ```rust
    // RenderableContent フィールド:
    //   display_iter: GridIterator<'a, Cell>  ← セルイテレータ
    //   selection: Option<SelectionRange>
    //   cursor: RenderableCursor
    //   display_offset: usize
    //   colors: &'a Colors
    //   mode: TermMode
    pub fn with_renderable_content<F, R>(&self, f: F) -> R
    where F: FnOnce(&RenderableContent<'_>) -> R {
        let term = self.term.lock();
        f(&term.renderable_content())
    }
    ```
  - `Terminal::resize(size)`:
    ```rust
    // Term::resize シグネチャ: pub fn resize<S: Dimensions>(&mut self, size: S)
    self.term.lock().resize(size);
    ```
  - `Terminal::scroll_display(scroll)`:
    ```rust
    // Term::scroll_display シグネチャ: pub fn scroll_display(&mut self, scroll: Scroll)
    //   (Terminal::scroll() は PLAN 記載だが実際は scroll_display())
    // Scroll enum: Delta(i32), PageUp, PageDown, Top, Bottom
    self.term.lock().scroll_display(scroll);
    ```
  - `Terminal::damage()` / `Terminal::reset_damage()`:
    ```rust
    // Term::damage() -> TermDamage<'_>
    // TermDamage enum: Full | Partial(TermDamageIterator<'a>)
    // TermDamageIterator は LineDamageBounds をイテレート
    ```
- 検証方法: `Terminal::new()` → `write(b"echo test\r")` → 200ms 後に `with_renderable_content()` で "test" を含む Cell を assert
- 推定規模: L (~380 行)
- 使用 crate: `alacritty_terminal` 0.26 (FairMutex は alacritty_terminal::sync 内蔵), `tokio` (mpsc channel のみ)
- 注意点:
  - **FairMutex は alacritty_terminal::sync::FairMutex** (parking_lot ではない, PLAN 記載を修正). parking_lot の `FairMutex` とは API が異なる: `lock()`, `lock_unfair()`, `lease()` がある.
  - **Term::input() は char を受け取る** (`fn input(&mut self, c: char)`) — バイト列ではない. PTY からのバイト列処理は EventLoop が内部で行うため, 外部から `term.input()` を呼ぶ必要はない. Notifier 経由で PTY に書き込む.
  - **scroll は `scroll_display()` が正しいメソッド名** (PLAN 記載の `scroll()` は存在しない).
  - COSMIC Term の EventProxy パターン踏襲: `mpsc::Sender<Event>` を `try_send` で送信 (blocking しない).

**P0-T4: Iced Canvas ターミナル描画**
- 入力: P0-T3
- 出力: `src/terminal_widget.rs`, `src/app.rs` 更新
- 実装内容:
  - `TerminalWidget`: Iced `canvas::Program` を実装するカスタムウィジェット
  - `canvas::Program` trait の検証済みシグネチャ:
    ```rust
    pub trait Program<Message, Theme = Theme, Renderer = Renderer> {
        type State: Default + 'static;
        fn draw(&self, state: &Self::State, renderer: &Renderer, theme: &Theme,
                bounds: Rectangle, cursor: Cursor) -> Vec<<Renderer as Renderer>::Geometry>;
        fn update(&self, _state: &mut Self::State, _event: &Event,
                  _bounds: Rectangle, _cursor: Cursor) -> Option<Action<Message>> { None }
        fn mouse_interaction(&self, ...) -> Interaction { Interaction::default() }
    }
    ```
  - `draw()` メソッド実装 (iced_term の view.rs パターン):
    ```rust
    let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
        // 1. デフォルト背景を fill_rectangle で描画
        // 2. セルをイテレート: terminal.with_renderable_content(|c| { for cell in c.display_iter { ... } })
        // 3. 背景バッチ最適化: BackgroundBatch で連続同色セルをまとめて描画 (draw call 削減)
        // 4. テキスト: frame.fill_text(canvas::Text { content, position, color, size, font, ... })
        // 5. カーソル: fill_rectangle でカーソルブロック描画
        // 6. 下線・取り消し線: stroke_line
    });
    vec![geometry]
    ```
  - `canvas::Cache` 統合:
    ```rust
    // Cache::draw(renderer, size, |frame: &mut Frame| { ... }) -> Geometry
    // Cache::clear() でキャッシュ無効化 → 次フレームで再描画
    pub struct TerminalWidget {
        terminal: Arc<Terminal>,
        cache: canvas::Cache,        // フレーム間でジオメトリをキャッシュ
        cell_width: f32,
        cell_height: f32,
    }
    ```
  - セルサイズ計算: P0 では `cell_width = font_size * 0.6`, `cell_height = font_size * 1.2` の近似. P1 で fontdue による正確な計測に置換.
  - `App::view()`: `Canvas::new(terminal_widget).width(Length::Fill).height(Length::Fill)`
  - `App::subscription()` (50ms ポーリング):
    ```rust
    // iced 0.14 の時間ベース Subscription: iced::time::every(Duration::from_millis(50))
    // → Message::Tick を発火 → update() で terminal.needs_update を確認 → cache.clear() で再描画
    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(50)).map(|_| Message::Tick)
    }
    ```
- 検証方法: `cargo run` でウィンドウにシェルプロンプトが描画される. カーソルが点滅する.
- 推定規模: L (~450 行)
- 使用 crate: `iced` (features: canvas, wgpu)
- 注意点:
  - **cosmic-term は canvas::Program を使わず Widget trait を直接実装し cosmic-text の Buffer を描画している**. iced_term は canvas::Program を使っており, xmux は iced_term パターンを採用する.
  - `canvas::Frame::fill_text()` でテキストを描画. `fill_rectangle()` で背景を描画. これらは確認済み.
  - **フォント**: `canvas::Text` の `font` フィールドに `iced::Font::with_name("JetBrains Mono")` 等を設定. P0 では等幅フォントの cell_width 近似で開始.
  - **Subscription**: `iced::time::every()` が 50ms ポーリング用. イベント駆動の代替は `alacritty_terminal::event::Event` (PtyWrite 等) を mpsc 経由で受け取る方法があるが P0 では不要.
  - **50ms ポーリング vs イベント駆動**: 50ms (20fps 相当) は P0 では許容. `terminal.needs_update` フラグ (`EventProxy` が `Event::Wakeup` 受信時に set) で不要な `cache.clear()` を省く.

**P0-T5: キーボード入力処理**
- 入力: P0-T4
- 出力: `src/input.rs`, `src/app.rs` 更新
- 実装内容:
  - `InputHandler` モジュール: Iced の `keyboard::Event` を VT100/VT220 エスケープシーケンスに変換
  - **イベント受信**: `TerminalWidget` は `canvas::Program` ではなく `iced::widget::Widget` を直接実装する. `on_event()` の `event: &Event` 引数で `Event::Keyboard(KeyEvent::KeyPressed { .. })` をパターンマッチ. `keyboard::listen()` による global subscription は不要 — widget 内でイベントを完結させる
  - **通常文字**: `KeyPressed { text, .. }` の `text: Option<SmolStr>` フィールドから UTF-8 バイト列を取得して PTY に送信
  - **修飾数値エンコーディング** (`mod_no`): xterm 互換の修飾子番号をビットフラグで計算
    ```rust
    fn modifier_number(m: &Modifiers) -> u8 {
        let mut n = 0u8;
        if m.shift()   { n |= 1; }
        if m.alt()     { n |= 2; }
        if m.control() { n |= 4; }
        if m.logo()    { n |= 8; }
        n + 1  // 修飾なし = 1, Shift = 2, Alt = 3, Ctrl = 5, ...
    }
    ```
  - **ヘルパー関数**:
    ```rust
    fn csi(code: &str, suffix: &str, mod_no: u8) -> Option<Vec<u8>> {
        if mod_no == 1 { Some(format!("\x1B[{code}{suffix}").into_bytes()) }
        else            { Some(format!("\x1B[{code};{mod_no}{suffix}").into_bytes()) }
    }
    fn csi2(code: &str, mod_no: u8) -> Option<Vec<u8>> {
        if mod_no == 1 { Some(format!("\x1B[{code}").into_bytes()) }
        else            { Some(format!("\x1B[1;{mod_no}{code}").into_bytes()) }
    }
    fn ss3(code: &str, mod_no: u8) -> Option<Vec<u8>> {
        if mod_no == 1 { Some(format!("\x1B\x4F{code}").into_bytes()) }
        else            { Some(format!("\x1B[1;{mod_no}{code}").into_bytes()) }
    }
    ```
  - **特殊キー変換表** (`Key::Named` に対してマッチ):
    | Named variant | 通常モード | APP_CURSOR モード |
    |---|---|---|
    | `Enter` | `\r` | `\r` |
    | `Tab` | `\t` | `\t` |
    | `Escape` | `\x1b` | `\x1b` |
    | `Backspace` | `\x7f` | `\x7f` |
    | `ArrowUp` | `csi2("A", mod_no)` | `ss3("A", mod_no)` |
    | `ArrowDown` | `csi2("B", mod_no)` | `ss3("B", mod_no)` |
    | `ArrowRight` | `csi2("C", mod_no)` | `ss3("C", mod_no)` |
    | `ArrowLeft` | `csi2("D", mod_no)` | `ss3("D", mod_no)` |
    | `Home` | `csi2("H", mod_no)` | `ss3("H", mod_no)` |
    | `End` | `csi2("F", mod_no)` | `ss3("F", mod_no)` |
    | `Insert` | `csi("2", "~", mod_no)` | 同左 |
    | `Delete` | `csi("3", "~", mod_no)` | 同左 |
    | `PageUp` | `csi("5", "~", mod_no)` (Shift → viewport scroll) | 同左 |
    | `PageDown` | `csi("6", "~", mod_no)` (Shift → viewport scroll) | 同左 |
    | `F1`–`F4` | `ss3("P"/"Q"/"R"/"S", mod_no)` | 同左 |
    | `F5` | `csi("15", "~", mod_no)` | 同左 |
    | `F6`–`F8` | `csi("17"/"18"/"19", "~", mod_no)` | 同左 |
    | `F9`–`F12` | `csi("20"/"21"/"23"/"24", "~", mod_no)` | 同左 |
  - **Ctrl 修飾 + 文字キー**: `modifiers.control()` が true の場合, ASCII 文字 `a`–`z` に対して `char as u8 & 0x1F` でコントロール文字を生成 (`Ctrl+C` → `\x03`, `Ctrl+D` → `\x04`, `Ctrl+Z` → `\x1a` 等)
  - **Alt 修飾**: `modifiers.alt()` が true の場合, 生成したバイト列の前に `\x1b` を prepend
  - **APP_CURSOR モード判定**: `term.lock().mode().contains(TermMode::APP_CURSOR)` で分岐. `is_app_cursor` 変数に格納して矢印キー・Home・End のシーケンス切替に使用
  - **APP_KEYPAD モード**: 本フェーズでは未実装 (COSMIC Terminal も未対応). P7 仕上げで対応
  - **IME 対応**: `Event::InputMethod(input_method::Event::Commit(text))` で確定文字列を PTY に送信. Preedit 中は `preedit: Option<Preedit>` を Widget State に保持し Canvas 上に下線付き表示
  - **フォーカス管理**: Widget の `State` に `is_focused: bool` を持ち, クリックで focus(), ウィンドウ blur で unfocus(). キーイベントは `is_focused` 時のみ処理
- 検証方法: `cargo run` → キー入力でコマンド入力可能. Ctrl+C でプロセス中断. 矢印キーで bash ヒストリ移動. `vim` が起動し, `hjkl` 移動, `:q` で終了可能. `htop` で F キーが動作
- 推定規模: M (~300 行) — ヘルパー関数 + 変換テーブル + IME 状態管理で妥当
- 使用 crate: `iced` (keyboard module, input_method module)
- テストケース:
  ```rust
  #[test]
  fn test_arrow_normal_mode() {
      assert_eq!(key_to_bytes(Named::ArrowUp, &Modifiers::NONE, false), Some(b"\x1B[A".to_vec()));
  }
  #[test]
  fn test_arrow_app_cursor() {
      assert_eq!(key_to_bytes(Named::ArrowUp, &Modifiers::NONE, true), Some(b"\x1BOA".to_vec()));
  }
  #[test]
  fn test_ctrl_c() {
      assert_eq!('c' as u8 & 0x1F, 0x03u8);
  }
  #[test]
  fn test_f5() {
      assert_eq!(key_to_bytes(Named::F5, &Modifiers::NONE, false), Some(b"\x1B[15~".to_vec()));
  }
  #[test]
  fn test_shift_arrow_modifier_no() {
      // mod_no = Shift(1) + 1 = 2
      assert_eq!(key_to_bytes(Named::ArrowUp, &Modifiers::SHIFT, false), Some(b"\x1B[1;2A".to_vec()));
  }
  ```
- 注意点:
  - Iced 0.14 の `KeyPressed` フィールド (確認済み): `key: Key`, `modified_key: Key`, `physical_key: Physical`, `location: Location`, `modifiers: Modifiers`, `text: Option<SmolStr>`, `repeat: bool`. 通常文字入力には `text` を優先使用 (ロケール・デッドキー対応のため). `key` で分岐するのは特殊キーのみ
  - `key == modified_key` のガード条件を設けること — COSMIC Terminal のパターン踏襲. `AltGr` 等で文字が変わるキーを誤って特殊キーとして処理しない
  - `keyboard::listen()` は Subscription API だが, Widget 実装では `on_event()` 内で処理するため不要. アプリレベルの global hotkey (ペイン分割 `Ctrl+Shift+D` 等) のみ `keyboard::listen()` を使う
  - `canvas::Program::update()` の `event: &canvas::Event` も `canvas::Event::Keyboard(...)` と `canvas::Event::InputMethod(...)` を含む. Canvas approach でもキー受信は可能だが, Widget 直接実装の方がフォーカス管理が明示的
  - `TermMode::DISAMBIGUATE_ESC_CODES` / Kitty keyboard protocol はターミナルアプリが要求した場合のみ対応. P0 では未対応, P7 で検討

**P0-T6: テキスト選択とクリップボード**
- 入力: P0-T5
- 出力: `src/selection.rs`, `crates/xmux-platform/src/linux.rs` (clipboard 追加)
- 実装内容:
  - **マウスイベントで選択範囲を管理**: Widget の `on_event()` 内で `Event::Mouse(...)` をパターンマッチ
    ```rust
    // ボタン押下 → 選択開始
    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
        let location = pixel_to_grid(cursor_pos, bounds, padding, cell_size);
        let side = if col_frac < 0.5 { Side::Left } else { Side::Right };
        let ty = match click_count {
            1 => SelectionType::Simple,
            2 => SelectionType::Semantic,   // ダブルクリック: 単語選択
            _ => SelectionType::Lines,      // トリプルクリック: 行選択
        };
        term.selection = Some(Selection::new(ty, location, side));
        state.is_selecting = true;
    }
    // カーソル移動 → 選択更新
    Event::Mouse(mouse::Event::CursorMoved { position }) if state.is_selecting => {
        let location = pixel_to_grid(position, bounds, padding, cell_size);
        let side = ...;
        if let Some(sel) = &mut term.selection {
            sel.update(location, side);
        }
    }
    // ボタン解放 → 選択確定
    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
        state.is_selecting = false;
        // 選択テキストを X11 PRIMARY に自動コピー (Linux慣習)
    }
    ```
  - **グリッド座標変換** (`pixel_to_grid`):
    ```rust
    fn pixel_to_grid(pos: Point, bounds: Rectangle, padding: Padding, cell: Size) -> TermPoint {
        let x = pos.x - bounds.x - padding.left;
        let y = pos.y - bounds.y - padding.top;
        let col = (x / cell.width) as usize;
        let row = (y / cell.height) as usize;
        // viewport_to_point で display_offset を考慮した絶対座標に変換
        terminal.viewport_to_point(Point::new(row, Column(col)))
    }
    ```
  - **クリック回数検出** (`ClickKind`): 前回クリック時刻と位置から Single/Double/Triple を判定. `DOUBLE_CLICK_INTERVAL = Duration::from_millis(300)`
  - **テキスト取得**: `term.selection_to_string()` → `Option<String>`. `term.selection` フィールドは `pub Option<Selection>` なので直接アクセス
  - **`LinuxClipboard`**: `arboard::Clipboard::new()` → `set_text()` / `get_text()`. Wayland では `wayland-data-control` feature を有効化すること.
    - **`arboard::Clipboard` は `Send + Sync` ではない可能性**: スレッドセーフでない場合, `Mutex<Clipboard>` でラップするか, 専用スレッドに mpsc チャネルでクリップボード操作を委譲する
  - **Ctrl+Shift+C**: 選択テキストをシステムクリップボードにコピー (clipboard API feature: Copy)
  - **Ctrl+Shift+V**: `clipboard.get_text()` → PTY に書き込み. ブラケットペーストモード (`BRACKETED_PASTE`) が有効なら `\x1b[200~` ... `\x1b[201~` で囲む
  - **選択ハイライト描画**: `term.selection` の `SelectionRange` を `to_range(&term)` で取得 → 範囲内セルの背景色を `selection_bg_color` で上書き
- 検証方法: `cargo run` → マウスドラッグで文字選択 → 背景色反転 → Ctrl+Shift+C でコピー → 他アプリにペースト可能. ダブルクリックで単語選択. トリプルクリックで行選択
- 推定規模: M (~300 行) — 妥当. グリッド変換・クリック検出・ブラケットペースト等で膨らむ
- 使用 crate: `arboard` (features: `wayland-data-control`), `iced` (mouse module)
- テストケース:
  ```rust
  #[test]
  fn test_pixel_to_grid_basic() {
      // cell_width=8, cell_height=16, no padding, no scroll offset
      let pos = Point::new(20.0, 48.0);  // x=20 -> col=2, y=48 -> row=3
      let result = pixel_to_grid_viewport(pos, 8.0, 16.0);
      assert_eq!(result, (3, 2));  // (row, col)
  }
  #[test]
  fn test_pixel_side_left() {
      // x=20.0, cell_width=8.0 → col=2, frac=0.5/8=0.5 → Side::Right境界
      let frac = 20.0_f32 % 8.0 / 8.0;  // = 0.5 → 丁度境界, Side::Right
      assert!(frac >= 0.5);
  }
  ```
- 注意点:
  - `alacritty_terminal` 0.26 の選択 API (確認済み):
    - `Selection::new(ty: SelectionType, location: Point, side: Side) -> Selection`
    - `selection.update(point: Point, side: Side)` で選択終点を更新
    - `selection.to_range(&term)` → `Option<SelectionRange>` でグリッド範囲取得
    - `term.selection_to_string()` → `Option<String>` でテキスト取得
    - `term.selection: Option<Selection>` は `pub` フィールドなので直接代入可能
    - `SelectionType` variants: `Simple`, `Block`, `Semantic` (単語), `Lines` (行)
    - `semantic_search_left/right` という独立関数は存在しない — `SelectionType::Semantic` を使うと自動的に単語境界に拡張される
  - `Side` は `alacritty_terminal::index::Side`. `Left` / `Right` の 2 variants (確認は docs で直接型エイリアスと記載があり, 実体は enum)
  - `arboard` の Wayland 対応: デフォルトでは X11/XWayland. 純粋 Wayland で正しく動かすには `Cargo.toml` で `arboard = { version = "3", features = ["wayland-data-control"] }` を指定. 失敗時は X11 にフォールバックするので, 機能フラグを入れておけば両対応になる
  - ブラケットペーストモード確認: `term.mode().contains(TermMode::BRACKETED_PASTE)`

---

### P1: 分割ペイン + 縦タブサイドバー (6 日)

**目標**: Iced `pane_grid` で水平・垂直分割. 左側に縦タブサイドバーでワークスペース一覧を表示. ワークスペース切替が動作する.

**完了条件**: Ctrl+Shift+D で垂直分割, Ctrl+Shift+E で水平分割. 複数ペインにそれぞれ独立したシェルが起動. 左サイドバーにワークスペース名と作業ディレクトリが表示され, クリックでワークスペース切替.

**推定工数**: 6 日 (COSMIC Terminal にワークスペース概念がなく xmux 独自設計のため +1 日)

#### タスク

**P1-T1: pane_grid 統合**
- 入力: P0 完成
- 出力: `src/pane.rs`, `src/app.rs` 更新
- 実装内容:
  - `App` の state に `pane_grid::State<PaneState>` を追加
  - `PaneState`: `Terminal` + `PaneId` + メタデータを保持
    ```rust
    struct PaneState {
        id: PaneId,
        terminal: Terminal,
        focused: bool,
        title: String,
    }
    ```
  - `App::view()`: `pane_grid::PaneGrid::new(&self.panes, |id, pane, _| { ... })` でペイングリッド構築
  - 各ペイン内に `TerminalWidget` を配置
  - 分割: `Message::Split(Axis)` → `self.panes.split(axis, focused, new_pane_state)`
  - 閉じる: `Message::ClosePane` → `self.panes.close(pane)`, ペインの PTY を kill
  - リサイズ: `on_resize` で `self.panes.resize(split, ratio)`
  - フォーカス: ペインクリックで `Message::FocusPane(pane)` → アクティブペイン切替
- 検証方法: `cargo run` → Ctrl+Shift+D → 画面が左右に分割 → 各ペインで独立してコマンド実行可能 → Ctrl+Shift+W でペイン閉じ
- 推定規模: L (~400 行)
- 使用 crate: `iced` (pane_grid)
- 注意点:
  - **`pane_grid::State::split()` のシグネチャ**: `&mut self` メソッドで `Option<(Pane, Split)>` を返す (計画記載の `(State, Option<...>)` は誤り). `None` の場合は分割不可 (最小サイズ制約).
  - **`PaneGrid::new()` のクロージャ**: `Content` を返す必要がある — `|id, pane, _| { pane_grid::Content::new(view_fn(pane)) }`.
  - **`on_resize` に `leeway` 引数が必要**: `PaneGrid::on_resize(leeway, |resize_event| Message::Resize(resize_event))`.
  - ペイン閉じ時の PTY クリーンアップを確実に行う (`child.kill()`).

**P1-T2: ワークスペース管理**
- 入力: P1-T1
- 出力: `src/workspace.rs`
- 実装内容:
  - `WorkspaceManager`:
    ```rust
    pub struct WorkspaceManager {
        workspaces: Vec<Workspace>,
        active_index: usize,
    }
    ```
  - `Workspace`:
    ```rust
    pub struct Workspace {
        pub id: WorkspaceId,
        pub name: String,
        pub pane_state: pane_grid::State<PaneState>,
        pub focused_pane: pane_grid::Pane,
        pub metadata: WorkspaceMetadata,
    }
    ```
  - ワークスペース作成: 新規 `pane_grid::State` + 初期ペイン (Terminal 付き)
  - ワークスペース切替: `active_index` 変更 → `view()` で該当ワークスペースの `pane_grid` を描画
  - ワークスペース削除: 全ペインの PTY を kill → ワークスペースを Vec から除去
  - ショートカット: Ctrl+Shift+T (新規), Ctrl+Shift+N/P (前後切替), Ctrl+Shift+数字 (直接切替)
- 検証方法: Ctrl+Shift+T で新ワークスペース作成 → サイドバーにタブ追加 → Ctrl+Shift+N/P で切替 → 各ワークスペースのペイン状態が独立
- 推定規模: M (~300 行)
- 使用 crate: なし (内部ロジック)
- 注意点: ワークスペース切替時に, 非アクティブワークスペースの Terminal は PTY I/O を継続するが, 描画は停止する. `Subscription` で全ワークスペースのイベントを監視し続ける

**P1-T3: 縦タブサイドバー**
- 入力: P1-T2
- 出力: `src/sidebar.rs`
- 実装内容:
  - `Sidebar` ウィジェット: Iced の `Column` + `Button` で構築
  - 各タブ表示項目:
    - ワークスペース名 (編集可能: ダブルクリック)
    - 作業ディレクトリ (短縮パス: `~/workspace/project`)
    - git ブランチ名 (後で P3 で実装, スタブ表示)
    - 通知バッジ (後で P2 で実装, スタブ)
  - レイアウト: 左端に固定幅 (200px) のサイドバー, 右側に `pane_grid`
  - サイドバー折りたたみ: Ctrl+B でトグル
  - タブの並び替え: ドラッグ&ドロップ (Iced の DnD 対応)
  - アクティブタブのハイライト
- 検証方法: `cargo run` → 左側にサイドバー表示 → ワークスペースが縦に並ぶ → クリックで切替 → Ctrl+B でサイドバー非表示/表示
- 推定規模: M (~300 行)
- 使用 crate: `iced` (widget)
- 注意点: Iced 0.14 ではドラッグ&ドロップは `dnd` feature で対応. P1 ではクリック切替のみ実装し, DnD は P4 で対応

**P1-T4: リサイズ対応とビューポート計算**
- 入力: P1-T1
- 出力: `src/terminal_widget.rs` 更新
- 実装内容:
  - **グリッドサイズ計算**: `bounds.width / cell_width` (floor) で列数, `bounds.height / cell_height` で行数. `u16::max(MIN_COLUMNS, cols as u16)` で下限保証
  - **`Term::resize()`**: `alacritty_terminal::term::Term::resize<S: Dimensions>(&mut self, size: S)` を呼ぶ. `S` は行・列を返す trait — `PtySize` 相当の型を実装して渡す
  - **PTY サイズ同期**: `Notifier` 経由で EventLoop にリサイズメッセージを送信 → EventLoop が内部で `pty.set_winsize()` を呼び, SIGWINCH が PTY 側プロセスに自動送信される
  - **前回サイズとの比較**: `state.last_size: (u16, u16)` を保持し, 変化した場合のみリサイズ処理を実行 (無駄なロックを回避)
  - **デバウンス実装**: Iced の `Subscription` タイマーではなく, Widget の `layout()` / `draw()` のタイミングで前回サイズと比較する方式が最もシンプル. 代替として `tokio::time::sleep` + `AbortHandle` で 50ms デバウンスも可能だが複雑になる
    ```rust
    // Widget::layout() または draw() 内で
    let new_size = (cols, rows);
    if new_size != state.last_size {
        state.pending_resize = Some(new_size);
        // 実際のリサイズは次フレームの draw() 冒頭で実行
    }
    ```
  - ペイン分割リサイズ (`pane_grid::on_resize`) でも同様に各ペインの `TerminalWidget` に新しい bounds が渡され, 上記処理が働く
- 検証方法: ウィンドウをドラッグリサイズ → ターミナル内容が正しく再描画 → `tput cols; tput lines` でサイズが正しい値を返す → ペイン境界ドラッグでもサイズ追従
- 推定規模: S (~150 行) — 妥当
- 使用 crate: なし (内部ロジック), `alacritty_terminal` (Dimensions trait)
- テストケース:
  ```rust
  #[test]
  fn test_grid_size_from_bounds() {
      let bounds = Size::new(800.0, 600.0);
      let cell = Size::new(8.0, 16.0);
      let cols = (bounds.width / cell.width).floor() as u16;   // = 100
      let rows = (bounds.height / cell.height).floor() as u16; // = 37
      assert_eq!(cols, 100);
      assert_eq!(rows, 37);
  }
  ```
- 注意点:
  - `Term::resize()` は `<S: Dimensions>` を受け取る. `Dimensions` trait は `alacritty_terminal::grid` にあり, `screen_lines()` と `columns()` を返す. 既存の `TerminalSize` 型に実装を追加する, または `alacritty_terminal::term::TermSize` (もし存在すれば) を使う
  - `SIGWINCH` は EventLoop 内部の `pty.set_winsize()` 呼び出しで PTY 側プロセスに自動送信される. `alacritty_terminal` 側は `Term::resize()` を呼ぶことで内部グリッドを更新する. 両方 (Term::resize + Notifier 経由の PTY リサイズ) を行うこと
  - リサイズ中に `renderable_content()` を呼ぶと不整合が出る可能性 → `FairMutex` でロック (`term.lock()` を取得している間のみ `resize()` と `renderable_content()` を呼ぶ)
  - 50ms デバウンスは"連続リサイズで PTY を毎フレーム叩かない"ための最適化. P1 では前回サイズ比較の簡易実装で十分. 厳密なデバウンスは P5 最適化フェーズで検討

**P1-T5: スクロールバック**
- 入力: P1-T1
- 出力: `src/terminal_widget.rs` 更新, `src/scrollbar.rs`
- 実装内容:
  - **ホイールスクロール**: `Event::Mouse(mouse::Event::WheelScrolled { delta })` で受信
    ```rust
    ScrollDelta::Lines { y, .. } => {
        let lines = (-y * 3.0) as i32;  // 1 ノッチ = 3行 (調整可)
        if lines != 0 { term.scroll_display(Scroll::Delta(-lines)); }
    }
    ScrollDelta::Pixels { y, .. } => {
        // 累積してセル高さを超えたら1行スクロール
        state.scroll_pixels -= y;
        let overflow = (state.scroll_pixels / cell_height) as i32;
        if overflow != 0 {
            state.scroll_pixels -= overflow as f32 * cell_height;
            term.scroll_display(Scroll::Delta(overflow));
        }
    }
    ```
  - **Scroll enum** (確認済み): `Scroll::Delta(i32)`, `Scroll::PageUp`, `Scroll::PageDown`, `Scroll::Top`, `Scroll::Bottom`. `alacritty_terminal::grid::Scroll`
  - **Shift+PageUp/Down**: キーイベントハンドラ内で `Scroll::PageUp` / `Scroll::PageDown` を呼ぶ
  - **スクロール位置計算** (COSMIC Terminal 実装より確認):
    ```rust
    fn scrollbar_position(&self, term: &Term) -> Option<(f32, f32)> {
        let grid = term.grid();
        let history = grid.history_size();
        if history == 0 { return None; }
        let total = history + grid.screen_lines();
        let display_offset = grid.display_offset();
        let start = (total - display_offset - grid.screen_lines()) as f32 / total as f32;
        let end = (total - display_offset) as f32 / total as f32;
        Some((start, end))
    }
    ```
  - **スクロールバーウィジェット** (`src/scrollbar.rs`):
    - Canvas 上に手描き (幅 8px, 右端固定)
    - ホバーで不透明, 非ホバーで α=0.4 の半透明
    - サムをドラッグして任意位置へのジャンプ
    - `scrollbar_position()` が `None` (履歴なし) のときは非表示
  - **スクロールロック**: `display_offset > 0` のとき新出力で自動スクロールしない. 入力 (PTY への書き込み) または `Scroll::Bottom` でロック解除
  - **スクロールロック判定**: `term.grid().display_offset() == 0` のとき末尾にいる (ロックなし)
  - スクロールバッファ上限は `alacritty_terminal::term::Config { scrolling: Scrolling { history: 100_000 } }` で設定済み (P0-T3 で設定)
- 検証方法: `seq 1000` 実行 → マウスホイールで上下スクロール → 過去の出力が閲覧可能 → スクロールバーが位置を反映 → スクロール中は新出力でスクロール位置が動かない
- 推定規模: M (~250 行) — スクロールバー Canvas 描画 + ドラッグ処理で妥当
- 使用 crate: `iced` (canvas), `alacritty_terminal` (grid::Scroll)
- テストケース:
  ```rust
  #[test]
  fn test_scroll_delta_lines_to_i32() {
      let y = -1.0_f32;  // 上スクロール (ホイール上方向)
      let lines = (-y * 3.0) as i32;  // = 3 (下にスクロール → history を見る)
      assert_eq!(lines, 3);
  }
  #[test]
  fn test_scrollbar_position_at_bottom() {
      // display_offset=0 (末尾), history=100, screen_lines=40
      let total = 140;
      let start = (140 - 0 - 40) as f32 / 140.0;  // = 100/140 ≈ 0.714
      let end   = (140 - 0) as f32 / 140.0;         // = 1.0
      assert!((start - 0.714).abs() < 0.001);
      assert_eq!(end, 1.0);
  }
  #[test]
  fn test_scrollbar_position_at_top() {
      // display_offset=100 (先頭), history=100, screen_lines=40
      let total = 140;
      let start = (140 - 100 - 40) as f32 / 140.0; // = 0.0
      let end   = (140 - 100) as f32 / 140.0;        // = 40/140 ≈ 0.286
      assert_eq!(start, 0.0);
      assert!((end - 0.286).abs() < 0.001);
  }
  ```
- 注意点:
  - `Term::scroll_display()` の呼び出しシグネチャ (確認済み): `pub fn scroll_display(&mut self, scroll: Scroll) where T: EventListener`
  - `Scroll` は `alacritty_terminal::grid::Scroll`. `Copy` trait 実装済み
  - `renderable_content()` はスクロール位置 (`display_offset`) を既に反映した内容を返すので, 呼び出し前に `scroll_display()` を呼ぶだけで描画が更新される
  - `iced::widget::Scrollable` は使わない — alacritty_terminal の内部バッファがスクロール状態を管理するため, Iced の scrollable widget と競合する. 手描きスクロールバー + `scroll_display()` の組み合わせが正しいアプローチ
  - `ALTERNATE_SCROLL` モードが有効なとき, ホイールイベントを Arrow Up/Down として PTY に送信する必要がある (vim 等のアルタネートスクリーンアプリ用). `term.mode().contains(TermMode::ALTERNATE_SCROLL)` で分岐

---

### P2: 通知システム + OSC パース (5 日)

**目標**: ターミナル出力から OSC 9/99/777 シーケンスを検出し, アプリ内通知 + デスクトップ通知を送信. サイドバーのタブにバッジ表示.

**完了条件**: `printf '\e]9;Task done\a'` でデスクトップ通知が表示. サイドバーのタブに通知バッジ (数字) が表示. 通知パネルで一覧表示と既読管理.

**推定工数**: 5 日 (OSC インターセプト方式の実装複雑性 +1 日)

#### タスク

**P2-T1: OSC シーケンスパーサー**
- 入力: P1 完成
- 出力: `crates/xmux-notification/src/parser.rs`
- 実装内容:
  - `OscParser`:
    ```rust
    pub struct OscParser {
        state: ParserState,
        buffer: Vec<u8>,
    }

    enum ParserState {
        Normal,
        Escape,       // \x1b を受信
        OscStart,     // \x1b] を受信
        OscParam,     // パラメータ番号読み取り中
        OscData,      // データ読み取り中
    }

    pub enum OscEvent {
        Notification(OscNotification),
        Passthrough(Vec<u8>),  // 通知以外の出力
    }

    pub struct OscNotification {
        pub protocol: OscProtocol,
        pub title: Option<String>,
        pub body: String,
    }

    pub enum OscProtocol {
        Osc9,    // \x1b]9;<body>\x07
        Osc99,   // \x1b]99;i=<id>:<body>\x07
        Osc777,  // \x1b]777;notify;<title>;<body>\x07
    }
    ```
  - パース: PTY 出力バイトストリームをフィルタし, OSC 9/99/777 を抽出
  - BEL (`\x07`) または ST (`\x1b\\`) で OSC 終端
  - **レート制限**: 1 秒あたり最大 3 件. 超過分はアプリ内通知のみ (デスクトップ通知は抑制).
- 検証方法: ユニットテスト: `parse(b"\x1b]9;hello\x07")` → `OscEvent::Notification { protocol: Osc9, body: "hello" }`. `parse(b"\x1b]777;notify;title;body\x07")` → title="title", body="body"
- 推定規模: M (~300 行) (レート制限 + バイトストリームタップ処理で増量)
- 使用 crate: なし (手書きパーサー)
- 注意点:
  - **[アーキテクチャ決定] OSC インターセプト方式 A (バイトストリームタップ) に確定**: alacritty_terminal の `EventListener` 経由では OSC 9/99/777 を取得不可能 (方式 B は排除). PTY 読み取りで同じバイト列を `OscParser` と `Term` 両方に渡す. 具体的には EventLoop のバイトストリーム処理をフックし, `OscParser::feed(bytes)` で通知 OSC を検出してから同じ bytes を `Term` に渡す.
  - `notify-rust` の D-Bus 送信が失敗した場合 (D-Bus デーモン未起動等) はログ出力してアプリ内通知のみ継続する. パニックしない.

**P2-T2: 通知マネージャー**
- 入力: P2-T1
- 出力: `crates/xmux-notification/src/lib.rs`
- 実装内容:
  - `NotificationManager`:
    ```rust
    pub struct NotificationManager {
        notifications: Vec<Notification>,
        max_count: usize,  // 最大保持数 (default: 1000)
    }
    ```
  - `add()`: 通知追加 → ID 発行 → デスクトップ通知送信要求
  - `list()`: 全通知取得
  - `list_by_pane(pane_id)`: ペインごとの通知
  - `unread_count(workspace_id)`: ワークスペースごとの未読数
  - `mark_read(id)`: 既読にする
  - `clear()`, `clear_one(id)`
  - デスクトップ通知連携: `PlatformNotifier::send_notification()` を呼び出し
- 検証方法: ユニットテスト: `add()` → `list()` → 件数確認. `mark_read()` → `unread_count()` 減少
- 推定規模: S (~150 行)
- 使用 crate: `notify-rust`
- 注意点: 通知の最大保持数超過時は古いものから破棄 (FIFO)

**P2-T3: 通知 UI (サイドバーバッジ + 通知パネル)**
- 入力: P2-T2, P1-T3
- 出力: `src/sidebar.rs` 更新, `src/notification_panel.rs`
- 実装内容:
  - サイドバー: 各タブの右上に未読通知数バッジ (赤丸 + 数字)
  - ペイン枠: 通知発生ペインの枠を青色にフラッシュ (2 秒間)
  - 通知パネル: サイドバー下部にトグル式パネル
    - 通知一覧: タイムスタンプ, タイトル, 本文, ソースペイン
    - クリックで該当ペインにジャンプ (ワークスペース切替 + フォーカス)
    - 全既読, 全クリアボタン
  - Iced `Subscription` で通知イベントを監視し UI 更新
- 検証方法: `printf '\e]9;Build done\a'` → デスクトップ通知ポップアップ → サイドバーのタブにバッジ "1" → 通知パネルに "Build done" が表示
- 推定規模: M (~300 行)
- 使用 crate: `iced` (widget)
- 注意点: デスクトップ通知はウィンドウが非フォーカス時のみ送信する (フォーカス時はアプリ内通知のみ)

**P2-T4: xmux notify CLI コマンド**
- 入力: P2-T2
- 出力: `src/cli.rs` (後で P3 の RPC に統合)
- 実装内容:
  - 暫定的な CLI: `xmux notify --title "Title" --body "Body"` で Unix ソケットに通知送信
  - P3 で JSON-RPC サーバーを実装後に統合するため, ここでは直接 `NotificationManager::add()` をプロセス内で呼ぶスタブ
  - 環境変数 `XMUX_PANE_ID` をペイン起動時に設定し, CLI からペイン特定
- 検証方法: `xmux notify --title "Test" --body "Hello"` → 通知パネルに表示
- 推定規模: S (~100 行)
- 使用 crate: `clap`
- 注意点: P3 (RPC) 実装前は同一プロセス内でのみ動作. ソケット経由の通知は P3 で対応

---

### P3: CLI + Unix ソケット JSON-RPC (7 日)

**目標**: Unix ドメインソケットで JSON-RPC v2 サーバーを起動し, 外部プロセスからワークスペース/ペイン操作が可能. `xmux` CLI バイナリで全コマンドを提供.

**完了条件**: `xmux list-workspaces --json` でワークスペース一覧を JSON 出力. `xmux new-split right` で右に分割. `xmux send "ls\n"` でコマンド送信. 全て別プロセスからソケット経由で実行.

**推定工数**: 7 日 (RPC ↔ Iced メインループ通信の非同期チャネル設計が最大の難所. +2 日)

#### タスク

**P3-T1: JSON-RPC v2 サーバー**
- 入力: P2 完成
- 出力: `crates/xmux-rpc/src/server.rs`, `crates/xmux-rpc/src/protocol.rs`
- 実装内容:
  - `RpcServer`:
    ```rust
    pub struct RpcServer {
        listener: tokio::net::UnixListener,
        handler: Arc<dyn RpcHandler>,
    }
    ```
  - ソケットパス: `/tmp/xmux-{uid}.sock` (マルチユーザー対応)
  - パーミッション: `0600` (owner-only)
  - 接続ごとに `tokio::spawn` で非同期ハンドラ
  - JSON-RPC v2 プロトコル:
    ```rust
    #[derive(Deserialize)]
    struct RpcRequest {
        jsonrpc: String,  // 必須: "2.0"
        id: Option<serde_json::Value>,
        method: String,
        params: Option<serde_json::Value>,
    }

    #[derive(Serialize)]
    struct RpcResponse {
        jsonrpc: &'static str,  // 常に "2.0"
        id: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<RpcError>,
    }
    ```
  - **[アーキテクチャ決定] フレーミング方式: NDJSON (Newline Delimited JSON)** — `tokio_util::codec::LinesCodec` を使用. 1 行 = 1 JSON-RPC メッセージ. Length-prefix 方式より実装がシンプルで, `socat` 等での手動テストが容易.
  - エラーコード: `-32700` (Parse error), `-32600` (Invalid Request), `-32601` (Method not found), `-32602` (Invalid params), `-32603` (Internal error)
- 検証方法: `#[tokio::test]` でソケット接続 → `{"jsonrpc":"2.0","id":1,"method":"system.ping"}` 送信 → `{"jsonrpc":"2.0","id":1,"result":"pong"}` 受信
- 推定規模: M (~350 行) (NDJSON フレーミング + jsonrpc フィールド検証で増量)
- 使用 crate: `tokio` (net::UnixListener), `tokio-util` (codec::LinesCodec), `serde_json`
- 注意点:
  - ソケットファイルは起動時に既存を unlink してから bind. シャットダウン時もクリーンアップ. `tokio::signal::ctrl_c()` でグレースフルシャットダウン.
  - **RPC ↔ Iced メインループ通信**: RPC ハンドラは tokio worker thread で動作するため, Iced の状態に直接アクセスできない. `iced::subscription::channel()` で Iced 側に mpsc チャネルを開き, RPC コマンドを送信. レスポンスは `tokio::sync::oneshot` で受け取る. タイムアウト (5 秒) を設定すること.

**P3-T2: RPC メソッドハンドラ**
- 入力: P3-T1
- 出力: `crates/xmux-rpc/src/handlers/mod.rs`, 各サブモジュール
- 実装内容:
  - メソッドルーター:
    ```rust
    pub struct MethodRouter {
        app_state: Arc<Mutex<AppState>>,
    }

    impl RpcHandler for MethodRouter {
        async fn handle(&self, method: &str, params: Value) -> Result<Value, RpcError> {
            match method {
                "system.ping" => Ok(json!("pong")),
                "system.capabilities" => self.capabilities(),
                "system.identify" => self.identify(),
                "workspace.list" => self.workspace_list(),
                "workspace.create" => self.workspace_create(params),
                "workspace.current" => self.workspace_current(),
                "workspace.select" => self.workspace_select(params),
                "workspace.close" => self.workspace_close(params),
                "workspace.rename" => self.workspace_rename(params),
                "surface.list" => self.surface_list(params),
                "surface.split" => self.surface_split(params),
                "surface.close" => self.surface_close(params),
                "surface.focus" => self.surface_focus(params),
                "surface.send_text" => self.surface_send_text(params),
                "surface.send_key" => self.surface_send_key(params),
                "surface.read_text" => self.surface_read_text(params),
                "notification.create" => self.notification_create(params),
                "notification.list" => self.notification_list(),
                "notification.clear" => self.notification_clear(),
                _ => Err(RpcError::method_not_found(method)),
            }
        }
    }
    ```
  - 各ハンドラ: `AppState` を操作して結果を返す
  - `surface.read_text`: `renderable_content()` から現在のスクリーン内容をテキストとして返す (エージェントが画面を読む用途)
- 検証方法: `echo '{"jsonrpc":"2.0","id":1,"method":"workspace.list"}' | socat - UNIX-CONNECT:/tmp/xmux-1000.sock` → ワークスペース一覧 JSON
- 推定規模: L (~500 行)
- 使用 crate: `serde_json`, `serde`
- 注意点:
  - **RPC ↔ Iced 通信パターン**: RPC ハンドラは tokio worker thread で動作し, 直接 Iced state を触らない. `iced::subscription::channel()` で Iced メインループ側にコマンド受信チャネルを開く. RPC ハンドラ → `mpsc::Sender<(RpcCommand, oneshot::Sender<RpcResult>)>` → Iced `update()` が処理 → `oneshot::Sender` で結果返送 → RPC レスポンス. タイムアウト (5 秒) 付き `oneshot::Receiver::recv()` で応答を待つ.
  - `Arc<Mutex<AppState>>` による直接共有は避ける — Iced の `update()` 以外からの状態変更はレースコンディションの原因になる.

**P3-T3: CLI クライアント**
- 入力: P3-T1
- 出力: `src/cli.rs` (完全版)
- 実装内容:
  - `clap` でサブコマンド定義:
    ```
    xmux [OPTIONS] <COMMAND>
    xmux list-workspaces [--json]
    xmux new-workspace [--name NAME] [--dir PATH]
    xmux select-workspace --workspace <ID>
    xmux current-workspace
    xmux close-workspace --workspace <ID>
    xmux new-split {left|right|up|down}
    xmux list-surfaces
    xmux focus-surface --surface <ID>
    xmux send "text"
    xmux send-key {enter|tab|escape|...}
    xmux send-surface --surface <ID> "text"
    xmux notify --title TITLE --body BODY
    xmux list-notifications
    xmux clear-notifications
    xmux ping
    xmux identify
    ```
  - 各サブコマンド: Unix ソケットに JSON-RPC リクエスト送信 → レスポンスを整形出力
  - `--json` フラグ: 生 JSON 出力 (スクリプト連携用)
  - `--socket PATH`: カスタムソケットパス
  - ソケット検出: `$XMUX_SOCKET_PATH` → `/tmp/xmux-{uid}.sock`
- 検証方法: `xmux ping` → "pong". `xmux list-workspaces --json` → JSON 配列. `xmux new-split right` → 画面分割
- 推定規模: M (~350 行)
- 使用 crate: `clap`, `tokio`, `serde_json`
- 注意点: CLI バイナリは同一クレートの `src/main.rs` に統合. 引数なしで起動 → GUI モード, サブコマンドあり → CLI モード. または別バイナリ `xmux-cli` として分離 (推奨: 同一バイナリの方がインストールが簡単)

**P3-T4: 環境変数の自動設定**
- 入力: P3-T1
- 出力: `crates/xmux-terminal/src/lib.rs` 更新
- 実装内容:
  - PTY 生成時に環境変数を設定:
    ```
    XMUX=1
    XMUX_PANE_ID=<pane-uuid>
    XMUX_WORKSPACE_ID=<workspace-uuid>
    XMUX_SOCKET_PATH=/tmp/xmux-{uid}.sock
    TERM=xterm-256color
    COLORTERM=truecolor
    ```
  - エージェントやスクリプトが xmux 内で動作中か検出可能
- 検証方法: `cargo run` → ターミナル内で `echo $XMUX` → "1". `echo $XMUX_PANE_ID` → UUID が出力
- 推定規模: S (~80 行)
- 使用 crate: なし
- 注意点: 既存の環境変数を上書きしないよう注意 (`PATH` 等)

**P3-T5: セキュリティ (ソケット認証)**
- 入力: P3-T1
- 出力: `crates/xmux-rpc/src/auth.rs`
- 実装内容:
  - デフォルトモード: ソケットパーミッション (`0600`) による OS レベル認証
  - 拡張モード (optional): プロセス祖先チェック (cmux 互換) — 接続元 PID の祖先に xmux プロセスが含まれるか確認
    - Linux: `/proc/{pid}/status` の `PPid` を辿る
  - パスワードモード (optional): 起動時にランダムトークン生成 → 環境変数 `XMUX_AUTH_TOKEN` で子プロセスに渡す → 接続時に `auth <token>` を最初のメッセージとして要求
- 検証方法: 別ユーザーからソケット接続 → Permission denied. 同一ユーザーの xmux 子プロセスから → 接続成功
- 推定規模: S (~150 行)
- 使用 crate: `rand` (トークン生成)
- 注意点: P3 では OS レベル認証 (パーミッション) のみ実装. プロセス祖先チェックとパスワードモードは P4 で追加

---

### P4: エージェントフック + git 連携 (4 日)

**目標**: Claude Code 等のエージェントを xmux 内で起動したとき, 通知フック・セッション復元が自動設定される. サイドバーに git ブランチ・PR ステータスが表示される.

**完了条件**: `claude` コマンドを実行 → エージェント完了時にデスクトップ通知. サイドバーのタブに git ブランチ名 `main` が表示. tmux 互換シムで `claude --teammate` が動作.

**推定工数**: 4 日

#### タスク

**P4-T1: エージェントレジストリ**
- 入力: P3 完成
- 出力: `crates/xmux-agent/src/registry.rs`
- 実装内容:
  - `AgentRegistry`: エージェント定義のデータベース
    ```rust
    pub struct AgentConfig {
        pub name: String,
        pub display_name: String,
        pub detect_env: Vec<String>,       // 検出用環境変数
        pub detect_process: Vec<String>,   // 検出用プロセス名
        pub hook_script: String,           // 通知フックスクリプト
        pub resume_command: Option<String>, // セッション復元コマンド
    }
    ```
  - 組み込みエージェント定義 (15+):
    - Claude Code: `detect_env: ["CLAUDE_CODE"]`, `hook: "xmux notify ..."`, `resume: "claude --resume {session_id}"`
    - Codex: `detect_env: ["CODEX_SESSION"]`, ...
    - Gemini CLI, Copilot, Cursor CLI, OpenCode, Amp, Rovo Dev, CodeBuddy, Factory, Pi, Grok, Qoder 等
  - `detect_agent()`: 現在の環境変数からアクティブなエージェントを自動検出
  - `setup_hooks()`: 検出したエージェントの通知フックをインストール
- 検証方法: `AgentConfig` の Claude Code 設定で `detect_agent({"CLAUDE_CODE": "1"})` → Some("claude-code")
- 推定規模: M (~250 行)
- 使用 crate: `serde` (TOML/JSON でカスタム定義読み込み)
- 注意点: エージェントの検出ロジックは環境変数優先. プロセス名検出は `/proc/{pid}/cmdline` を読むが, コストが高いので起動時 1 回のみ

**P4-T2: フック自動インストール**
- 入力: P4-T1
- 出力: `crates/xmux-agent/src/hooks.rs`
- 実装内容:
  - `xmux hooks setup [--agent AGENT]`: エージェントの設定ファイルに通知フックを追加
    - Claude Code: `~/.claude/settings.json` の `hooks` セクションに `PostToolUse`, `Stop` フックを追加
    - 各エージェントの設定ファイルパスとフック形式をレジストリから取得
  - `xmux hooks remove [--agent AGENT]`: フック除去
  - `xmux hooks list`: インストール済みフック一覧
  - 通知フックのテンプレート:
    ```bash
    #!/bin/sh
    xmux notify --title "${AGENT_NAME}" --body "Task completed" --pane "$XMUX_PANE_ID"
    ```
- 検証方法: `xmux hooks setup --agent claude-code` → Claude Code の設定ファイルにフックが追加. `xmux hooks list` → 設定済みエージェント一覧
- 推定規模: M (~200 行)
- 使用 crate: `serde_json` (設定ファイル操作)
- 注意点: 既存の設定ファイルを壊さないように, 設定の読み込み → マージ → 書き込みを行う. バックアップ作成

**P4-T3: git ブランチ・PR ステータス表示**
- 入力: P1-T3 (サイドバー)
- 出力: `src/sidebar.rs` 更新, `src/git_info.rs`
- 実装内容:
  - `GitInfo`:
    ```rust
    pub struct GitInfo {
        pub branch: Option<String>,
        pub pr_number: Option<u32>,
        pub pr_status: Option<PrStatus>,
        pub is_dirty: bool,
    }

    pub enum PrStatus {
        Open,
        Draft,
        Merged,
        Closed,
    }
    ```
  - git ブランチ取得: ペインの作業ディレクトリで `git rev-parse --abbrev-ref HEAD` を非同期実行
  - PR ステータス取得: `gh pr view --json number,state` を非同期実行 (GitHub CLI がインストールされている場合のみ)
  - ポーリング: 10 秒間隔で各ワークスペースの git 情報を更新
  - 作業ディレクトリ検出: PTY の foreground process の cwd を `/proc/{pid}/cwd` から取得
  - サイドバー表示: ブランチ名アイコン + テキスト, PR 番号 (#123) + ステータスバッジ
- 検証方法: git リポジトリ内で `cargo run` → サイドバーに現在のブランチ名表示. `git checkout -b feature` → 10 秒以内にサイドバー更新
- 推定規模: M (~250 行)
- 使用 crate: `tokio::process::Command`
- 注意点: `gh` コマンドが未インストールの場合はエラーにならず PR 情報を非表示にする. git リポジトリ外のディレクトリではブランチ表示なし

**P4-T4: tmux 互換シム**
- 入力: P3-T2 (RPC)
- 出力: `src/tmux_shim.rs`
- 実装内容:
  - `xmux` を `tmux` として振る舞わせるシム:
    - `TMUX` 環境変数をペイン内に設定 (エージェントが tmux セッション検出に使用)
    - **Claude Code の検出ロジック**: Claude Code は自分で tmux コマンドを発行しない — `$TMUX` 環境変数が非空であれば既存 tmux セッション内と判断し, tmux コマンドを使ってペイン操作を行う. つまり `TMUX=/tmp/xmux.sock,...` を PTY 環境変数に設定するだけで Claude Code の teammate モードが xmux を tmux として認識する.
    - tmux の基本コマンドを xmux RPC にマッピング:
      - `tmux new-session` → `workspace.create`
      - `tmux split-window` → `surface.split`
      - `tmux send-keys` → `surface.send_text`
      - `tmux list-sessions` → `workspace.list`
      - `tmux capture-pane` → `surface.read_text`
      - `tmux select-pane` → `surface.focus`
    - Claude Code teammate モード対応: `claude --teammate` が tmux API を呼ぶ → xmux が受ける
  - シムバイナリ: `xmux-tmux` として PATH に置くか, エイリアス設定スクリプトを提供
- 検証方法: `TMUX=/tmp/xmux.sock xmux-tmux list-sessions` → ワークスペース一覧. `claude --teammate` → xmux 内で teammate モード動作 (tmux API 呼び出しが xmux に到達)
- 推定規模: M (~300 行)
- 使用 crate: `clap`
- 注意点: tmux のプロトコルは複雑. 全コマンドの互換は不要 — Claude Code teammate モードが使う最小セット (`new-session`, `split-window`, `send-keys`, `capture-pane`, `list-panes`, `display-message`) のみ実装. 未対応コマンドはエラーメッセージで通知

**P4-T5: ポート検出**
- 入力: P1-T3 (サイドバー)
- 出力: `src/port_monitor.rs`
- 実装内容:
  - ペイン内プロセスが LISTEN しているポートを検出
  - Linux: `/proc/net/tcp` + `/proc/net/tcp6` をパースし, ペインの子プロセス群の inode と照合
  - 定期ポーリング (5 秒間隔)
  - サイドバー表示: `:3000`, `:8080` 等のポート番号バッジ
- 検証方法: ペイン内で `python3 -m http.server 8080` → 5 秒以内にサイドバーに `:8080` 表示
- 推定規模: M (~200 行)
- 使用 crate: なし (procfs 直接パース)
- 注意点: `/proc/net/tcp` のフォーマットはカーネルバージョンで安定. inode → PID の逆引きは `/proc/{pid}/fd/` のシンボリックリンクを読む. パフォーマンス: 多数のプロセスがある場合は `/proc/{pid}/net/tcp` (ネットワーク名前空間ごと) を使う

---

### P5: セッション保存/復元 + 描画最適化 (4 日)

**目標**: アプリ終了時にレイアウト・作業ディレクトリ・スクロールバックを保存し, 再起動で復元. 描画のダメージトラッキングで 130+ ワークスペースでも滑らか.

**完了条件**: `xmux` を閉じて再起動 → 同じワークスペース構成・ペインレイアウト・作業ディレクトリで復元. 100 ワークスペース開いた状態で入力遅延 < 16ms.

**推定工数**: 4 日

#### タスク

**P5-T1: セッションスナップショット**
- 入力: P4 完成
- 出力: `crates/xmux-session/src/snapshot.rs`
- 実装内容:
  - `SessionSnapshot`:
    ```rust
    #[derive(Serialize, Deserialize)]
    pub struct SessionSnapshot {
        pub version: u32,  // スキーマバージョン
        pub timestamp: u64,
        pub windows: Vec<WindowSnapshot>,
        pub active_window: usize,
    }

    #[derive(Serialize, Deserialize)]
    pub struct WindowSnapshot {
        pub workspaces: Vec<WorkspaceSnapshot>,
        pub active_workspace: usize,
    }

    #[derive(Serialize, Deserialize)]
    pub struct WorkspaceSnapshot {
        pub id: String,
        pub name: String,
        pub layout: LayoutNode,
        pub metadata: WorkspaceMetadataSnapshot,
    }

    #[derive(Serialize, Deserialize)]
    pub enum LayoutNode {
        Pane(PaneSnapshot),
        Split {
            axis: Axis,
            ratio: f32,
            first: Box<LayoutNode>,
            second: Box<LayoutNode>,
        },
    }

    #[derive(Serialize, Deserialize)]
    pub struct PaneSnapshot {
        pub working_dir: PathBuf,
        pub title: String,
        pub scrollback: Option<String>,  // 最新 N 行のテキスト
        pub agent_session_id: Option<String>,
        pub env_overrides: HashMap<String, String>,
    }
    ```
  - 保存先: `$XDG_DATA_HOME/xmux/sessions/`
  - ファイル名: `session_{timestamp}.json`
  - 最新 5 スナップショットを保持, 古いものは削除
- 検証方法: `SessionSnapshot` → JSON シリアライズ → デシリアライズ → 元と同一
- 推定規模: M (~250 行)
- 使用 crate: `serde`, `serde_json`
- 注意点:
  - **[検証済み] `pane_grid::State` は `Serialize`/`Deserialize` を derive していない** (実装されているのは `Clone`, `Debug` のみ). `pane_grid::State` を直接シリアライズすることは不可能. 必ず `pane_grid::State` → `LayoutNode` の変換レイヤーを実装すること.
  - `pane_grid::State` の内部は `BTreeMap<Pane, T>` と `Internal` (レイアウト木) で構成される. `Internal` は非公開型のため, 復元時は `LayoutNode` から `pane_grid::State::with_configuration()` を使って再構築する (Iced の `pane_grid::Configuration` enum を利用).
  - `pane_grid::Configuration` の定義:
    ```rust
    pub enum Configuration<T> {
        Split { axis: Axis, ratio: f32, a: Box<Configuration<T>>, b: Box<Configuration<T>> },
        Pane(T),
    }
    ```
    これが `LayoutNode` と 1:1 対応するため, 変換コストは低い.
  - スクロールバック全文は巨大になりうる → 最新 1000 行に制限. バイナリスクロールバック (画像等) は保存しない

**P5-T2: 自動保存 (dirty flag)**
- 入力: P5-T1
- 出力: `crates/xmux-session/src/autosave.rs`
- 実装内容:
  - `AutoSaver`:
    ```rust
    pub struct AutoSaver {
        dirty: Arc<AtomicBool>,
        interval: Duration,  // default: 30 秒
        save_task: Option<JoinHandle<()>>,
    }
    ```
  - dirty flag: ワークスペース作成/削除, ペイン分割/閉じ, 作業ディレクトリ変更時に `dirty.store(true)`
  - バックグラウンドタスク: `interval` ごとに dirty flag チェック → true なら `SessionManager::save()` → flag リセット
  - 保存はバックグラウンドスレッドで実行 (メインスレッドブロック回避 — cmux の失敗を回避)
  - シャットダウン時: 即座に最終保存
- 検証方法: ワークスペース作成 → 30 秒待機 → `$XDG_DATA_HOME/xmux/sessions/` にスナップショットファイル生成
- 推定規模: S (~150 行)
- 使用 crate: `tokio`
- 注意点:
  - 保存中にアプリが強制終了されても整合性を保つ → 一時ファイル (`.tmp` サフィックス) に書き込み → `std::fs::rename` でアトミック置換. rename は同一ファイルシステム内なら POSIX 保証のアトミック操作.
  - `tokio::fs::rename` を使い async コンテキストで完結させる. `write_all` + `sync_data` の後に rename.
  - `AutoSaver` の `save_task` は `tokio::spawn` で起動するが, シャットダウン時は `JoinHandle::await` で完了を待つ (強制 kill でデータ破損しないため).

**P5-T3: セッション復元**
- 入力: P5-T1
- 出力: `crates/xmux-session/src/restore.rs`
- 実装内容:
  - 起動時: 最新スナップショットを読み込み
  - 復元手順:
    1. `LayoutNode` からペイングリッド再構築
    2. 各ペインの `working_dir` で新しい PTY 生成
    3. スクロールバックテキストを PTY に流し込み (表示再現)
    4. エージェントセッション ID があれば `resume_command` を実行
    5. ワークスペースメタデータ (名前等) を復元
  - `xmux restore-session [--file PATH]`: 手動復元コマンド
  - `xmux list-sessions`: 保存済みスナップショット一覧
- 検証方法: ワークスペース 3 つ (各 2 ペイン) → 終了 → 再起動 → 同じレイアウトで復元 → 各ペインの pwd が正しい
- 推定規模: M (~300 行)
- 使用 crate: `serde_json`, `tokio`
- 注意点:
  - スクロールバック復元は完全ではない (ターミナルアプリが画面を再描画する可能性). エージェント復元は `resume_command` が存在する場合のみ試行.
  - `LayoutNode` → `pane_grid::State` 逆変換は `pane_grid::Configuration` を経由する:
    ```rust
    fn layout_to_config(node: LayoutNode) -> pane_grid::Configuration<RestoredPane> {
        match node {
            LayoutNode::Pane(p) => pane_grid::Configuration::Pane(RestoredPane { snapshot: p }),
            LayoutNode::Split { axis, ratio, first, second } => pane_grid::Configuration::Split {
                axis: axis.into(),
                ratio,
                a: Box::new(layout_to_config(*first)),
                b: Box::new(layout_to_config(*second)),
            },
        }
    }
    // 復元: pane_grid::State::with_configuration(config)
    ```
  - `Axis` の変換: `xmux_session::Axis` ↔ `iced::widget::pane_grid::Axis` は同型なので `From` impl を追加する.

**P5-T4: ダメージトラッキング描画最適化**
- 入力: P0-T4 (描画)
- 出力: `src/terminal_widget.rs` 更新
- 実装内容:
  - **TermDamage API (検証済み)**:
    ```rust
    // Term::damage() -> TermDamage<'_>
    // TermDamage enum: Full | Partial(TermDamageIterator<'a>)
    // TermDamageIterator は LineDamageBounds をイテレート
    // Term::reset_damage(&mut self) でリセット
    match term.damage() {
        TermDamage::Full => { self.cache.clear(); }
        TermDamage::Partial(iter) => {
            let damaged_lines: Vec<LineDamageBounds> = iter.collect();
            if !damaged_lines.is_empty() { self.cache.clear(); }
        }
    }
    term.reset_damage();
    ```
  - **Cache 統合パターン**: `canvas::Cache` は `Cache::draw()` が呼ばれたとき, 前回から `Cache::clear()` されていなければキャッシュ済みジオメトリを返す. ダメージなしフレームは `cache.clear()` を呼ばず → `draw()` はキャッシュヒットで GPU 負荷ゼロ.
  - **DamageRect は存在しない**: PLAN 記載の `DamageRect` は誤り. 実際は `LineDamageBounds` (行単位のダメージ情報). iced_term の実装では `TermDamage::Full` / `Partial` を区別せず, ダメージがあれば `cache.clear()` する単純パターンで十分.
  - 変更なしフレームスキップ: `terminal.needs_update` フラグ (EventProxy が `Event::Wakeup` 受信時に set) で制御. `needs_update == false` なら `cache.clear()` しない.
  - 非アクティブワークスペースの描画スキップ: `App::view()` で非アクティブペインに `Canvas` を配置しない (or 空の Widget を返す).
  - バッチ描画: iced_term の `BackgroundBatch` パターン (連続同色セルを蓄積して矩形をまとめて描画) を実装. `frame.fill_rectangle()` 呼び出し回数を `O(cols*rows)` → `O(色ブロック数)` に削減.
  - グリフキャッシュ: `canvas::Cache` がジオメトリ単位でキャッシュするため, 追加のグリフアトラスは P0-P5 では不要. 必要なら P7 で `glyphon` 等を検討.
- 検証方法: 100 ワークスペース作成 → `yes` コマンド実行 → フレームレート 60fps 維持. 入力遅延 < 16ms (フレーム計測). `TermDamage::Partial` 時に `cache.clear()` が呼ばれる回数を assert (変化なしフレームでは 0 回)
- 推定規模: M (~250 行, DamageRect 不使用で設計がシンプルになるため削減)
- 使用 crate: `iced` (canvas)
- 注意点:
  - **Iced Canvas の Cache は"再描画するかしないか"の粒度しか制御できない** — 変更行のみ部分再描画は不可能. Full redraw か skip の 2 択. これは Iced の設計上の制約.
  - 60fps 目標に対して 50ms ポーリングは 20fps 上限. P5 でイベント駆動 Subscription (`Subscription::run`) に切り替えて `Event::Wakeup` 受信時のみ再描画を検討する.
  - `Term::damage()` は `&mut self` を要求する (ロック取得が必要). `FairMutex::lock()` でロック保持中に `cache.clear()` の判断まで行う.

---

### P6: 内蔵ブラウザ (wry) (7 日)

**目標**: ペイン内に WebView を埋め込み, ターミナル横にブラウザを表示. URL ナビゲーション・JS 実行・スクリーンショットが RPC から操作可能.

**完了条件**: `xmux browser-open https://localhost:3000 --split right` でターミナル右にブラウザが開く. `xmux browser-eval "document.title"` でページタイトル取得. 開発サーバーのプレビューをターミナル横に表示.

**推定工数**: 7 日 (GTK イベントループ統合の不確実性で +2 日)

#### タスク

**P6-T1: wry WebView 統合**
- 入力: P5 完成
- 出力: `crates/xmux-browser/src/lib.rs`, `crates/xmux-browser/src/webview.rs`
- 実装内容:
  - `BrowserPane`:
    ```rust
    pub struct BrowserPane {
        pub id: PaneId,
        pub webview: wry::WebView,
        pub url: String,
        pub title: String,
        pub can_go_back: bool,
        pub can_go_forward: bool,
    }
    ```
  - **[検証済み] wry の最新バージョンは 0.55.x**. 採用 API:
    - `WebViewBuilder::new()` → `WebView::build<W: HasWindowHandle>(window)` (通常)
    - `WebViewBuilder::build_as_child<W: HasWindowHandle>(parent)` (子ウィンドウ)
    - **Linux 専用**: `WebViewBuilder::build_gtk<W: IsA<Container>>(widget)` (`WebViewBuilderExtUnix` trait)
    - JavaScript 実行: `WebView::evaluate_script(&str) -> Result<()>` (fire-and-forget)
    - JS 結果取得: `WebView::evaluate_script_with_callback(&str, impl Fn(String)) -> Result<()>`
    - ナビゲーション: `WebView::load_url(&str)`, `WebView::load_html(&str)`
  - **統合方式の選定** (3 方式の評価):
    1. **GTK child window 方式** (推奨): `gtk::Fixed` コンテナ内に `build_gtk` で WebView を作成. Iced が winit + GTK 上で動いている場合, winit の GTK ウィンドウから `gtk_window()` を取得し `gtk::Fixed` を overlay する. ペイン領域を GTK 座標に変換して配置. **実現可能性: 高** — `iced_webview_v2` (iced 0.14 対応, 2026 年 5 月現在 v0.1.11) がこのアプローチを採用している.
    2. **Shader widget 方式** (推奨代替): `iced_webview_v2` の実装パターン. WebView を CPU ラスタライズして `wgpu::Queue::write_texture()` でテクスチャ更新 → Iced の `shader` widget で描画. オフスクリーンレンダリング不要. ただしラスタライズエンジンが wry (WebKitGTK) ではなく Blitz/litehtml になる点に注意.
    3. **オフスクリーンレンダリング方式**: wry の issue #391 がオープンのまま (2026 年 6 月時点). **wry 自体はオフスクリーンレンダリングを未サポート** (Linux/WebKitGTK では特に). 採用不可.
  - **採用方針**: GTK child window 方式を第一選択. 問題発生時は `iced_webview_v2` の Shader widget 方式にフォールバック.
  - **GTK イベントループ統合**: Iced が winit を使っている場合, `gtk::init()` を明示的に呼び, winit の `about_to_wait()` ハンドラで `gtk::main_iteration_do(false)` を呼び続ける必要がある.
  - JS → Rust 通信: `WebViewBuilder::with_ipc_handler(|msg| { ... })` で受信
- 検証方法: `xmux browser-open https://example.com --split right` → 右ペインに Web ページ表示
- 推定規模: **XL (~700 行)** (統合の複雑さを反映して元計画の 500 行から増量)
- 使用 crate: `wry 0.55`, `gtk`, `raw-window-handle`
- 注意点:
  - **最大リスク**: Iced (winit ベース) と GTK のイベントループの二重管理. winit の `EventLoop` と `gtk::main_iteration_do` を同一スレッドで協調させる必要がある. Iced のメインループが winit を独占しているため, GTK ポーリングを `Subscription` の tick から呼び出す方法が現実的.
  - **スレッド制約**: WebKitGTK (WebView) はメインスレッドからのみ操作可能. RPC ハンドラ (tokio worker thread) から直接 `WebView` メソッドを呼ぶと UB. `tokio::sync::mpsc` で Iced メインループにコマンドを送り, メインループ側で WebView 操作する.
  - **ペイン分割との統合**: ペインリサイズ時に GTK ウィジェットの位置・サイズを更新する処理が必要. Iced の Canvas 座標 → GTK スクリーン座標変換が必要 (HiDPI スケーリング考慮).
  - **参考実装**: `iced_webview_v2` (github.com/franzos/iced_webview_v2, iced 0.14 対応済み) を参照すること.

**P6-T2: ブラウザ RPC メソッド**
- 入力: P6-T1, P3-T2
- 出力: `crates/xmux-rpc/src/handlers/browser.rs`
- 実装内容:
  - RPC メソッド追加:
    ```
    browser.open_split   — URL + 分割方向
    browser.navigate     — URL ナビゲーション
    browser.back         — 戻る
    browser.forward      — 進む
    browser.reload       — リロード
    browser.url.get      — 現在の URL 取得
    browser.eval         — JavaScript 実行
    browser.screenshot   — スクリーンショット (PNG base64)
    browser.close        — ブラウザペイン閉じ
    ```
  - CLI コマンドも同時追加: `xmux browser-open`, `xmux browser-eval` 等
- 検証方法: `xmux browser-eval "document.title"` → ページタイトル文字列を返す
- 推定規模: M (~300 行)
- 使用 crate: `serde_json`, `base64`
- 注意点:
  - `browser.eval` は非同期 — `WebView::evaluate_script_with_callback` を使い, コールバックで結果を `tokio::sync::oneshot::Sender` に送信してから RPC レスポンスを返す. タイムアウト (5 秒) を設定.
  - `browser.screenshot` は wry が直接スクリーンショット API を提供しないため, JS 経由 (`html2canvas` ライブラリのインジェクション) または GTK ウィジェットのスナップショット API (`gtk::Widget::create_snapshot`) で実装.

**P6-T3: ブラウザ・ターミナル連携**
- 入力: P6-T1, P6-T2
- 出力: `crates/xmux-browser/src/integration.rs`
- 実装内容:
  - ターミナル内の URL クリック → ブラウザペインで開く (Ctrl+Click)
  - 自動リロード: 開発サーバー検出 (localhost ポート) → ファイル変更で WebView リロード
  - DevTools トグル: F12 で WebView の DevTools 表示/非表示
- 検証方法: ターミナルに `http://localhost:3000` が表示 → Ctrl+Click → ブラウザペインで開く
- 推定規模: M (~200 行)
- 使用 crate: なし (内部ロジック)
- 注意点: URL 検出は `renderable_content()` からハイパーリンク属性のあるセルを探す. alacritty_terminal は OSC 8 ハイパーリンクをサポートしている

---

### P7: 数式レンダリング (5 日)

**目標**: ターミナル出力中の LaTeX 数式をインライン描画. 描画された数式領域を選択すると元の LaTeX ソースがコピーされる.

**完了条件**: `echo '$E = mc^2$'` の出力でインラインに数式がレンダリング表示される. `echo -e '\x1b]1337;LaTeX='$(echo -n '\frac{a}{b}' | base64)'\x07'` で分数が表示される. 数式領域を選択してコピーすると元の LaTeX テキストが得られる.

**推定工数**: 5 日

**前提 (調査済み)**:
- RaTeX の parser/layout/render は crates.io 未公開 (ratex-types のみ). 使用不可.
- 代替として typst + mitex (LaTeX→Typst 変換) を採用. 両方 crates.io 公開済み, 純 Rust.
- テキスト選択問題は xmux では非問題: GUI アプリのため描画レイヤーと選択レイヤーが独立. alacritty_terminal のセルグリッドが元のテキストを保持し, 選択/コピーはグリッドから読む.
- 既存実装: MathDetector ($/$$ 検出) と MathRenderer (スタブ) は `crates/xmux-math/` に作成済み.

#### タスク

**P7-T1a: MathDetector + MathRenderer スタブ [完了]**
- xmux-math クレート作成済み. MathDetector ($/$$), MathRenderer (スタブ RGBA), テスト 10 件.

**P7-T1b: レンダリングバックエンド (typst + mitex)**
- 入力: P7-T1a (MathRenderer スタブ)
- 出力: `crates/xmux-math/src/renderer.rs` 更新
- 実装内容:
  - MathRenderer のスタブを typst + mitex バックエンドに差し替え
  - パイプライン: LaTeX → `mitex` (LaTeX→Typst 数式構文) → `typst` (レイアウト) → `typst-render` (tiny-skia ラスタライズ) → RGBA バッファ
  - キャッシュ: LaTeX 文字列をキーに HashMap でレンダリング結果をキャッシュ (既存)
  - RenderedMath にベースライン情報を追加:
    ```rust
    pub struct RenderedMath {
        pub pixels: Vec<u8>,  // RGBA
        pub width: u32,
        pub height: u32,
        pub baseline_offset: f32,  // 画像上端からベースラインまでのピクセル数
    }
    ```
- 検証方法: `MathRenderer::render("\\frac{a}{b}", 16.0, [255,255,255,255])` が分数画像を返す. `render("E=mc^2", ...)` が等式を返す.
- 推定規模: M (~200 行)
- 使用 crate: `mitex-parser`, `mitex-spec`, `typst`, `typst-render`, `tiny-skia`
- 注意点:
  - typst の依存は重い (typst-library, fonts 等). ビルド時間が増加する.
  - mitex の LaTeX カバレッジは KaTeX より狭い可能性がある. 未対応構文はフォールバックとしてソーステキストをそのまま表示.
  - typst/mitex が devcontainer 環境でビルドできるか, 最初に `cargo check` で検証すること.
  - **代替案**: typst がビルドできない場合, `katex-rs` (QuickJS 内蔵) + `resvg` (SVG→PNG) を検討.

**P7-T1c: Canvas オーバーレイ統合**
- 入力: P7-T1b (実レンダラー), P0-T4 (terminal_view.rs)
- 出力: `src/terminal_view.rs` 更新, `crates/xmux-math/src/overlay.rs`
- 実装内容:
  - terminal_view.rs の `draw()` でセルグリッドテキストから数式領域を検出:
    1. 表示中の各行のテキストを取得
    2. MathDetector::find_math() で LaTeX 範囲を特定
    3. 範囲に対応するセル座標 (行, 列開始, 列終了) を算出
  - 検出した数式領域の描画:
    1. 該当セル範囲のテキスト描画をスキップ
    2. MathRenderer でレンダリングした画像を取得
    3. セル領域のピクセル矩形 (`col_start * cell_width`, `row * cell_height`, `num_cols * cell_width`, `cell_height`) を算出
    4. 画像をセル領域内に配置:
       - インライン ($): 高さを `cell_height` にスケール, ベースライン合わせ, 水平中央配置, 余白は背景色
       - ディスプレイ ($$): `ceil(image_height / cell_height)` 行分を使用, 水平中央配置
    5. `frame.fill_rectangle()` で背景クリア後, 画像ピクセルを Canvas に描画
  - 選択/コピー: 変更不要. alacritty_terminal グリッドが元の LaTeX テキストを保持しており, 既存の選択ロジックがそのまま動作する.
  - MathRenderer を App 構造体に保持し, 全ペインで共有
- 検証方法: `echo '$\frac{a}{b}$'` → 分数が描画される. 数式部分をドラッグ選択 → コピーすると `$\frac{a}{b}$` が得られる.
- 推定規模: L (~350 行)
- 使用 crate: `iced` (canvas)
- 注意点:
  - Canvas への画像描画: `iced::widget::canvas::Frame` には直接 image 描画 API がない. `frame.fill_rectangle()` でピクセル単位で描画するか, `iced::widget::image::Handle` + レイヤー合成を検討.
  - パフォーマンス: 可視領域の数式のみレンダリング. スクロール外の数式はスキップ.
  - Iced Canvas の制約: 部分再描画不可のため, 数式が含まれるフレームは全体再描画. ダメージトラッキング (P5-T4) と組み合わせて不要な再描画を抑制.

**P7-T1d: OSC 1337 LaTeX プロトコル**
- 入力: P2-T1 (OscParser)
- 出力: `crates/xmux-notification/src/parser.rs` 更新, `crates/xmux-math/src/detector.rs` 更新
- 実装内容:
  - OscParser に OSC 1337 の LaTeX サブコマンドを追加:
    - `\x1b]1337;LaTeX=<base64>\x07` を検出
    - base64 デコードして LaTeX 文字列を抽出
    - 新しいイベント型 `OscMathExpression { latex: String }` を追加
  - Terminal がこのイベントを受け取り, セルグリッド上に LaTeX ソースを書き込む (表示は P7-T1c のオーバーレイが担当)
- 検証方法: `echo -e '\x1b]1337;LaTeX='$(echo -n 'E=mc^2' | base64)'\x07'` → 数式が表示される
- 推定規模: S (~100 行)
- 使用 crate: `base64`
- 注意点:
  - iTerm2 の OSC 1337 は多機能 (画像, プロファイル等). LaTeX サブコマンドのみ実装, 他は無視.

---

### P8: クロスプラットフォーム + 品質仕上げ (4 日)

**目標**: macOS/Windows のプラットフォーム抽象レイヤー実装. 全体の品質仕上げ.

**完了条件**: macOS で `cargo build` が通り, 基本的なターミナル操作が動作する.

**推定工数**: 4 日

#### タスク

**P8-T1: macOS プラットフォーム実装**
- 入力: P0-T1 (platform traits)
- 出力: `crates/xmux-platform/src/macos.rs`
- 実装内容:
  - `MacPty`: `portable-pty` の macOS バックエンド (自動)
  - `MacNotifier`: `notify-rust` の macOS バックエンド (自動)
  - `MacClipboard`: `arboard` の macOS バックエンド (自動)
  - `MacShell`: default `/bin/zsh`, config_dir `~/Library/Application Support/xmux`
  - 条件コンパイル: `#[cfg(target_os = "macos")]`
- 検証方法: macOS で `cargo build` → `cargo run` → ターミナル動作
- 推定規模: S (~150 行)
- 使用 crate: (既存 crate の macOS バックエンド)
- 注意点: macOS の PTY は Linux と同じ POSIX API だが, シグナルハンドリングに差異あり. `portable-pty` が吸収するため, 大きな問題はないはず. Iced の macOS 対応は良好

**P8-T2: Windows プラットフォーム実装**
- 入力: P0-T1 (platform traits)
- 出力: `crates/xmux-platform/src/windows.rs`
- 実装内容:
  - `WindowsPty`: `portable-pty` の Windows バックエンド (ConPTY)
  - `WindowsNotifier`: `notify-rust` の Windows バックエンド (Win Toast)
  - `WindowsClipboard`: `arboard` の Windows バックエンド
  - `WindowsShell`: default `powershell.exe`, config_dir `%APPDATA%\xmux`
  - ソケット: 名前付きパイプ `\\.\pipe\xmux-{username}`
- 検証方法: Windows で `cargo build` → `cargo run` → ターミナル動作
- 推定規模: M (~200 行)
- 使用 crate: (既存 crate の Windows バックエンド)
- 注意点: Windows の ConPTY は制約が多い (ANSI シーケンスの一部未対応). `alacritty_terminal` の Windows 対応状況を確認. 名前付きパイプの JSON-RPC サーバーは `tokio::net::windows::named_pipe` を使う

**P8-T3: 品質仕上げ**
- 入力: 全フェーズ
- 出力: 各所のバグ修正, パフォーマンスチューニング
- 実装内容:
  - True Color (24-bit) 描画の検証と修正
  - Unicode 幅 (全角文字, 絵文字) の正確な描画
  - マウスモード (アプリケーションが要求するマウスイベント転送)
  - IME (入力メソッド) 対応
  - Alt Screen バッファ対応 (vim, less 等)
  - フォント fallback (不足グリフの代替フォント検索)
  - メモリプロファイリング: 100 ワークスペース × 100,000 行スクロールバックでのメモリ使用量測定
- 検証方法: `vttest` ターミナルテストスイート実行. `neofetch`, `htop`, `vim`, `tmux` (入れ子), `bat` 等のツールが正常動作
- 推定規模: L (~500 行, 分散)
- 使用 crate: 各種
- 注意点: この作業は全フェーズにまたがるため, 各フェーズで最低限の品質を確保しつつ, P7 でまとめて仕上げる

---

## 4. リスク一覧

| リスク | 影響 | 確率 | 対策 |
|---|---|---|---|
| **[更新] Iced + wry GTK イベントループ競合** | P6 ブラウザ機能が実装不可または不安定 | 高 | **第一選択**: GTK child window 方式 (`build_gtk` + `gtk::Fixed`). winit の `about_to_wait` で `gtk::main_iteration_do` を呼ぶ. **代替**: `iced_webview_v2` (iced 0.14 対応済み) の Shader widget 方式を採用. **最終手段**: 外部ブラウザ起動で代替. **廃止**: wry オフスクリーンレンダリングは Linux 未実装のため採用不可. |
| **[更新] wry WebView のスレッド制約** | RPC からの WebView 操作で UB | 高 | WebKitGTK はメインスレッド専用. RPC コマンドは `tokio::sync::mpsc` で Iced メインループに中継し, メインループ側で WebView を操作する. |
| **[新規] RPC ↔ Iced メインループ通信の非同期設計** | RPC レスポンスのデッドロック・タイムアウト | 高 | `iced::subscription::channel()` + `oneshot` パターンで実装. 全 RPC メソッドに 5 秒タイムアウトを設定. Iced の `update()` が重い場合はキューイングで対処. |
| **[新規] OSC インターセプト方式の実装複雑性** | バイトストリームタップで EventLoop 改造が必要 | 中 | 方式 A (バイトストリームタップ) に確定. EventLoop のバイト処理フックは alacritty_terminal のフォーク不要 — PTY reader の出力を OscParser と Term 両方に渡すラッパーで対処. |
| **[新規] arboard の `Send + Sync` 非互換** | `PlatformClipboard` trait が `Send + Sync` を要求するが arboard が満たさない可能性 | 中 | `Mutex<Clipboard>` でラップするか, 専用スレッド + mpsc チャネル委譲パターンで対処. |
| Iced Canvas の描画パフォーマンスが不足 | 大量テキスト描画で遅延 | 中 | ダメージトラッキング + グリフキャッシュ. 改善不足なら `iced_wgpu` の低レベル API でカスタムパイプライン |
| alacritty_terminal 0.26 の API 破壊的変更 | コンパイルエラー, 動作不良 | 低 | バージョンを固定 (`=0.26.0`). COSMIC Terminal のコードを参考パターンとして参照 |
| **[更新] RaTeX 未公開 → typst + mitex 採用** | 数式レンダリングバックエンド変更 | 確実 (既知) | RaTeX の parser/layout/render は crates.io 未公開 (ratex-types のみ). typst + mitex (LaTeX→Typst 変換) に切り替え. typst の依存が重いためビルド時間増加. mitex の LaTeX カバレッジ不足時はソーステキストをフォールバック表示. |
| **[更新] `pane_grid::State` の非シリアライズ** | セッション保存でコンパイルエラー | 確実 (既知) | `pane_grid::State` は Serialize/Deserialize を持たない (検証済み). `pane_grid::Configuration` 経由の変換レイヤーで対処する (P5-T1 に設計済み). |
| **[更新] tmux 互換シムの互換性不足** | Claude Code teammate モードが動作しない | 中 | Claude Code は `$TMUX` 非空検出で tmux セッション利用を決定. tmux コマンドの最小セット (`new-session`, `split-window`, `send-keys`, `capture-pane`, `list-panes`, `display-message`) を正確に実装. |
| **[解消] ~~portable-pty の Linux PTY 挙動差異~~** | ~~一部ターミナルアプリが動作しない~~ | — | Linux では alacritty_terminal::tty を直接使用する方針に確定 (§2 PTY 管理方式の設計判断を参照). portable-pty は macOS/Windows のみ (P7). |
| Iced 0.14 の API 不安定 | 0.15 で breaking change | 中 | Iced バージョンを `=0.14` で固定. アップグレードは計画的に実施 |
| Wayland 環境でのクリップボード・IME 問題 | Linux デスクトップで操作性低下 | 中 | `arboard` の `wayland-data-control` feature で Wayland ネイティブ対応. 失敗時は X11 にフォールバック |
| 130+ ワークスペースでメモリ膨張 | cmux と同じ問題を再現 | 低 | Rust の所有権で構造的に防止. ただしスクロールバッファが主因なので, スクロールバック上限の動的調整 (非アクティブワークスペースは上限削減) を検討 |

---

## 5. 品質基準

### テストカバレッジ方針

| レイヤー | テスト種別 | カバレッジ目標 |
|---|---|---|
| `xmux-core` | ユニットテスト | 90%+ (型, 設定パース, エラー変換) |
| `xmux-terminal` | ユニットテスト + 統合テスト | 70%+ (PTY 生成, 入力処理, イベント変換) |
| `xmux-notification` | ユニットテスト | 90%+ (OSC パーサー, 通知管理) |
| `xmux-rpc` | 統合テスト | 80%+ (全 RPC メソッドの正常/異常系) |
| `xmux-session` | ユニットテスト + 統合テスト | 80%+ (シリアライズ/デシリアライズ, 保存/復元) |
| `xmux-agent` | ユニットテスト | 70%+ (エージェント検出, フック生成) |
| `xmux-platform` | プラットフォーム固有テスト | 50%+ (CI 環境制約あり) |
| UI (Iced) | 手動テスト + スクリーンショットテスト | 手動チェックリスト |

テスト実行: `cargo test --workspace`

### CI 構成

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo check --workspace --all-targets
      - run: cargo clippy --workspace -- -D warnings
      - run: cargo fmt --all -- --check
      - run: cargo test --workspace

  # macOS ビルド検証 (P7 以降)
  macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo check --workspace

  # Windows ビルド検証 (P7 以降)
  windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo check --workspace
```

システム依存パッケージ (Linux CI):
```
apt-get install -y libxkbcommon-dev libwayland-dev libfontconfig1-dev pkg-config
```

### Lint / Format ルール

- `rustfmt`: デフォルト設定 + `edition = "2021"`, `max_width = 100`
- `clippy`:
  ```toml
  # Cargo.toml or .clippy.toml
  [lints.clippy]
  pedantic = "warn"
  nursery = "warn"
  unwrap_used = "deny"
  expect_used = "warn"
  ```
- `cargo deny`: ライセンス互換性チェック (MIT/Apache-2.0 のみ許可)
- MSRV: Rust 1.85.0 (alacritty_terminal 0.26 の要件)

---

## 6. 工数サマリー

| フェーズ | 目標 | 推定日数 | 累計 | 備考 |
|---|---|---|---|---|
| P0 | 最小ターミナル | 5 日 | 5 日 | P0-T1 は ~700 行に増量 (Config 型定義) |
| P1 | 分割ペイン + サイドバー | **6 日** | **11 日** | ワークスペース独自設計 +1 日 |
| P2 | 通知システム | **5 日** | **16 日** | OSC インターセプト方式 A 実装 +1 日 |
| P3 | CLI + JSON-RPC | **7 日** | **23 日** | RPC ↔ Iced 通信設計 +2 日 |
| P4 | エージェントフック + git | 4 日 | **27 日** | |
| P5 | セッション + 描画最適化 | 4 日 | **31 日** | |
| P6 | 内蔵ブラウザ | **7 日** | **38 日** | GTK イベントループ統合の不確実性 |
| P7 | 数式 + クロスプラットフォーム | 4 日 | **42 日** | RaTeX は公開済みで変更なし |

合計: 約 **42 日** (Sonnet 4.6 実装前提). 旧見積 38 日から +4 日. P1/P2/P3 の工数増が主因. P6 の GTK/wry 統合リスクが顕在化した場合はさらに +3~5 日.

### P0–P3 工数評価 (詳細)

| タスク | 旧推定 | 修正推定 | 変更理由 |
|---|---|---|---|
| P0-T1 (workspace 初期化) | ~300 行 | **~700 行** | Config 全サブ型定義が予想より大きい |
| P0-T4 (Canvas 描画) | ~500 行 | **~450 行** | EventLoop が I/O を担うため若干削減 |
| P1 全体 | 5 日 | **6 日** | COSMIC Terminal にワークスペース概念なく独自設計 |
| P2-T1 (OSC パーサー) | ~250 行 | **~300 行** | レート制限 + バイトストリームタップ |
| P2 全体 | 4 日 | **5 日** | OSC インターセプト方式 A の実装複雑性 |
| P3-T1 (RPC サーバー) | ~300 行 | **~350 行** | NDJSON フレーミング + jsonrpc フィールド検証 |
| P3 全体 | 5 日 | **7 日** | RPC ↔ Iced メインループ通信が最大の難所 |

### P5+P6+P7 工数評価 (詳細)

| タスク | 旧推定 | 修正推定 | 変更理由 |
|---|---|---|---|
| P5-T1 (スナップショット) | 250 行 | 300 行 | `pane_grid::Configuration` 変換レイヤー追加 |
| P5-T2 (自動保存) | 150 行 | 150 行 | 変更なし |
| P5-T3 (セッション復元) | 300 行 | 350 行 | `LayoutNode` → `Configuration` 逆変換実装 |
| P5-T4 (描画最適化) | 300 行 | **250 行** | DamageRect 不使用で設計がシンプルに |
| P6-T1 (wry 統合) | 500 行 | **700 行** | GTK イベントループ統合, スレッド安全 WebView 操作 |
| P6-T2 (ブラウザ RPC) | 300 行 | 350 行 | eval の oneshot チャネル実装追加 |
| P6-T3 (ブラウザ連携) | 200 行 | 200 行 | 変更なし |
| P7-T1 (RaTeX) | 300 行 | 300 行 | crate 公開確認済み, 変更なし |
| P8-T1~T3 (その他) | 850 行 | 850 行 | 変更なし |
| **P5+P6+P7 合計** | **3,150 行** | **3,450 行** | |

P5+P6+P7 は 15 日 (P5: 4 日 + P6: 7 日 + P7: 4 日). P6 の GTK 統合が 1 週間以内に解決しない場合は `iced_webview_v2` への切り替えを即断する判断基準を設けること.

---

## 7. 依存クレート一覧

| クレート | バージョン | 用途 | ライセンス |
|---|---|---|---|
| `iced` | 0.14 | UI フレームワーク | MIT |
| `alacritty_terminal` | 0.26 | ターミナルエミュレーション + PTY (Linux) | Apache-2.0 |
| `portable-pty` | 0.9 | PTY 管理 (macOS/Windows のみ, P7) | MIT |
| `wgpu` | (iced 経由) | GPU レンダリング | MIT/Apache-2.0 |
| `tokio` | 1.x | 非同期ランタイム | MIT |
| `tokio-util` | 0.7 | codec::LinesCodec (NDJSON フレーミング) | MIT |
| `serde` | 1.x | シリアライゼーション | MIT/Apache-2.0 |
| `serde_json` | 1.x | JSON | MIT/Apache-2.0 |
| `clap` | 4.x | CLI パーサー | MIT/Apache-2.0 |
| `uuid` | 1.x | ID 生成 | MIT/Apache-2.0 |
| ~~`parking_lot`~~ | — | ~~同期プリミティブ~~ **削除**: FairMutex は alacritty_terminal::sync に内蔵 | — |
| `arboard` | 3.x | クリップボード (features: `wayland-data-control`) | MIT/Apache-2.0 |
| `notify-rust` | 4.x | デスクトップ通知 | MIT/Apache-2.0 |
| `wry` | **0.55.x** | WebView (P6) | MIT/Apache-2.0 |
| `gtk` | 0.18 | GTK コンテナ (P6, Linux) | MIT |
| `ratex-parser` | **0.1.11** | LaTeX パース (P7) | MIT |
| `ratex-layout` | **0.1.11** | LaTeX レイアウト (P7) | MIT |
| `ratex-render` | **0.1.11** | LaTeX 描画 / tiny-skia (P7) | MIT |
| `ratex-katex-fonts` | **0.1.11** | KaTeX TTF 埋め込み (P7) | MIT |
| `thiserror` | 2.x | エラー型 | MIT/Apache-2.0 |
| `dirs` | **6.0** | プラットフォームディレクトリ | MIT/Apache-2.0 |
| `rand` | 0.8 | トークン生成 | MIT/Apache-2.0 |
| `base64` | 0.22 | エンコーディング | MIT/Apache-2.0 |


---

## 8. API 検証メモ (P0-T2 / P0-T3 / P0-T4 / P5-T4)

検証日: 2026-06-13. ドキュメント: docs.rs, iced_term src, COSMIC Terminal src.

### 8.1 alacritty_terminal 0.26

| 項目 | 検証結果 |
|---|---|
| `Term::new()` | `pub fn new<D: Dimensions>(config: Config, dimensions: &D, event_proxy: T) -> Term<T>` |
| `Term::input()` | `fn input(&mut self, c: char)` — char 単位. バイト列は EventLoop が内部処理 |
| `Term::resize()` | `pub fn resize<S: Dimensions>(&mut self, size: S)` |
| `Term::renderable_content()` | `pub fn renderable_content(&self) -> RenderableContent<'_>` — `T: EventListener` 要求 |
| `Term::damage()` | `pub fn damage(&mut self) -> TermDamage<'_>` — `&mut self` |
| `Term::reset_damage()` | `pub fn reset_damage(&mut self)` |
| `Term::scroll_display()` | `pub fn scroll_display(&mut self, scroll: Scroll)` — **scroll() ではない** |
| `Term::mode()` | `pub fn mode(&self) -> &TermMode` |
| `Config` フィールド | `scrolling_history: usize`, `default_cursor_style`, `vi_mode_cursor_style`, `semantic_escape_chars: String`, `kitty_keyboard: bool`, `osc52: Osc52`. **`Config::default()` が使える** |
| `FairMutex` | **alacritty_terminal::sync::FairMutex** (parking_lot ではない). メソッド: `new()`, `lock()`, `lock_unfair()`, `lease()`, `try_lock_unfair()` |
| `EventListener` trait | `fn send_event(&self, _event: Event) {}` (デフォルト空実装あり). 実装は 1 メソッドのみ |
| `RenderableContent` フィールド | `display_iter: GridIterator<'a, Cell>`, `selection: Option<SelectionRange>`, `cursor: RenderableCursor`, `display_offset: usize`, `colors: &'a Colors`, `mode: TermMode` |
| `TermDamage` | `enum { Full \| Partial(TermDamageIterator<'a>) }`. イテレータは `LineDamageBounds` を yield |
| `Scroll` | `enum { Delta(i32), PageUp, PageDown, Top, Bottom }` |
| `tty::new()` | `pub fn new(config: &Options, window_size: WindowSize, window_id: u64) -> Result<Pty>` |
| `tty::Options` | `shell: Option<Shell>`, `working_directory: Option<PathBuf>`, `drain_on_exit: bool`, `env: HashMap<String, String>` |
| `EventLoop::new()` | `pub fn new(terminal, event_proxy, pty, drain_on_exit, ref_test) -> Result<EventLoop<T, U>>` |
| `EventLoop::spawn()` | `-> JoinHandle<(Self, State)>` — **std::thread ベース** |
| `Notifier::notify()` | `fn notify<B: Into<Cow<'static, [u8]>>>(bytes)` |

### 8.2 iced 0.14

| 項目 | 検証結果 |
|---|---|
| `canvas::Program` | `type State: Default + 'static;` + `draw()` (required) + `update()`, `mouse_interaction()` (provided) |
| `draw()` シグネチャ | `fn draw(&self, state: &Self::State, renderer: &Renderer, theme: &Theme, bounds: Rectangle, cursor: Cursor) -> Vec<<Renderer as Renderer>::Geometry>` |
| `canvas::Cache` | `Cache::draw(renderer, size, closure)` → Geometry をキャッシュ. `Cache::clear()` で無効化 |
| `canvas::Frame::fill_rectangle()` | 存在確認済み (iced_term 使用). 矩形塗りつぶし |
| `canvas::Frame::fill_text()` | 存在確認済み. `canvas::Text` 構造体を引数に取る |
| `canvas::Frame::stroke_line()` 等 | 下線・取り消し線に使用 |
| ポーリング Subscription | `iced::time::every(Duration::from_millis(50)).map(\|_\| Message::Tick)` |
| イベント駆動 Subscription | `Subscription::run()` または `Subscription::run_with()` |
| `canvas::Text` | `content`, `position`, `color`, `size`, `font`, shaping フィールドあり |

### 8.3 PLAN との差異サマリー

| PLAN 記載 | 実際 | 対処 |
|---|---|---|
| `portable-pty` で PTY 生成 | `alacritty_terminal::tty` + `EventLoop` が正しいパターン | P0-T2 を設計変更. Linux では portable-pty 不使用 |
| `FairMutex` は `parking_lot` | `alacritty_terminal::sync::FairMutex` (独自実装) | P0-T3 注意点に記載. parking_lot を依存から削除 |
| `term.lock().input(data)` でバイト列処理 | `input()` は char 受け取り. バイト列は EventLoop が処理 | P0-T3 を修正 |
| `Terminal::scroll()` | `Term::scroll_display()` が正しいメソッド名 | P0-T3 修正済み |
| `TermDamage` から `DamageRect` を取得 | `LineDamageBounds` が正しい型名. `DamageRect` は存在しない | P5-T4 修正済み |
| cosmic-term の描画を参考に | cosmic-term は canvas ではなく cosmic-text の Buffer を直接描画. **iced_term の方が参考になる** | P0-T4 注意点に記載 |
| `parking_lot` を依存に追加 | 不要 (FairMutex は alacritty_terminal に内蔵) | 依存クレート表から削除済み |
| `iced::application("xmux", ...)` | 第1引数は boot 関数. `iced::application(App::new, App::update, App::view).title(...)` | P0-T1 注意点を修正済み |
| `pane_grid::State::split()` → `(State, Option<...>)` | `&mut self` → `Option<(Pane, Split)>` | P1-T1 注意点を修正済み |
| `semantic_search_left/right` で単語選択 | 関数は存在しない. `SelectionType::Semantic` で代替 | P0-T6 注意点に記載済み |
| JSON-RPC リクエストに `jsonrpc` フィールドなし | JSON-RPC v2 仕様で `"jsonrpc": "2.0"` は必須 | P3-T1 プロトコル定義を修正済み |
| OSC 通知は EventListener 経由で取得 | EventListener からは取得不可能. バイトストリームタップ (方式 A) に確定 | P2-T1 注意点を修正済み |
| `dirs` 5.x | 最新は 6.0.0 | 依存バージョンを更新済み |

### 8.4 参考実装

- **iced_term** (`github.com/Harzu/iced_term`): iced + alacritty_terminal の最も直接的な参考実装. Backend, view.rs のパターンを踏襲する.
- **COSMIC Terminal** (`github.com/pop-os/cosmic-term`): EventProxy パターンと Terminal struct の構造を参照. ただし描画は cosmic-text ベースで xmux とは異なる.
