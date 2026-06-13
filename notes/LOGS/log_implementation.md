# 実装ログ

## 2026-06-13: P1 分割ペイン + 縦タブサイドバー

Workflow ツールで haiku モデルの実装エージェント 5 体 + 検証エージェント 1 体を順次実行. 約 11 分, 274k トークンで完了.

### 実装内容

| タスク | コミット | 概要 |
|---|---|---|
| P1-T1 | `e62ac74` | pane_grid 統合. PaneState (Terminal + Cache + PaneId). Ctrl+Shift+D/E で分割, Ctrl+Shift+W で閉じ |
| P1-T2 | `b0ef0e5` | WorkspaceManager + Workspace. 複数ワークスペース管理. Ctrl+Shift+T/N/P |
| P1-T3 | `fc1063e` | 縦タブサイドバー (200px, Column+Button). Ctrl+B トグル. アクティブタブハイライト |
| P1-T4 | `7671fa3` | 動的リサイズ. Cell<(u16, u16)> で interior mutability. compute_grid_size() + 5 テスト |
| P1-T5 | `176dc4d` | スクロールバック. マウスホイール, Shift+PageUp/Down, Canvas スクロールバー, ALTERNATE_SCROLL 対応 |

### テスト結果

- xmux: 31 テスト (input 23 + terminal_view 8)
- xmux-terminal: 2 テスト (pty_echo, terminal_echo)
- 全 33 テスト pass

### 設計判断

- PaneState に `Cell<(u16, u16)>` を使用し, draw() 内で immutable 参照のままリサイズ検出を実現
- スクロールバーは iced::widget::Scrollable を使わず Canvas 上に手描き (alacritty_terminal の内部バッファと競合回避)
- 全ワークスペースの全ペインで Tick 時にイベント処理 (非アクティブワークスペースもバックグラウンドで出力受信)
- ワークスペースショートカットは TerminalView::update() で PTY 転送前にインターセプト

## 2026-06-13: P2 通知システム

Workflow ツールで haiku モデルの実装エージェント 4 体 + 検証エージェント 1 体を実行. 約 7.5 分, 204k トークンで完了.

### 実装内容

| タスク | コミット | 概要 |
|---|---|---|
| P2-T1 | `34252f5` | xmux-notification クレート. OscParser: OSC 9/99/777 のバイトレベルパーサー. BEL/ST 両対応, ストリーミング入力対応. 11 テスト |
| P2-T2 | `e955d5e` | NotificationManager (add/list/mark_read/clear) + socketpair ベースの PTY バイトストリーム傍受. relay スレッドで OSC パース + 双方向転送. resize は orig_fd への直接 ioctl |
| P2-T3 | `a41f3dc` | 通知 UI. サイドバーにワークスペース別未読バッジ. 通知パネル (scrollable, Read All/Clear ボタン). NotificationManager を App に統合 |
| P2-T4 | `0f29ba6` | Ctrl+Shift+I でテスト通知注入. InjectTestNotification メッセージ |

### テスト結果

- xmux: 31 テスト (input 23 + terminal_view 8)
- xmux-notification: 16 テスト (parser 11 + manager 5)
- xmux-terminal: 2 テスト (pty_echo, terminal_echo)
- 全 49 テスト pass

### 設計判断

- PTY バイトストリーム傍受に Unix socketpair + relay スレッドを採用. alacritty_terminal の EventLoop は変更不要
- socketpair 上で ioctl(TIOCSWINSZ) が失敗し die!() するため, resize 時は保存した orig_fd に直接 ioctl を呼ぶ
- OSC 99 の ID フィールドはパース時に読み捨て, xmux 内部では UUID を新規生成 (外部 ID との紐付けは P3 以降で検討)
- Terminal::new() と new_with_notifications() を分離し, テストの後方互換性を維持

## 2026-06-13: P3 CLI + JSON-RPC

Workflow ツールで haiku モデルの実装エージェントを実行.

### 実装内容

| タスク | コミット | 概要 |
|---|---|---|
| P3-T1 | `49731dd` | xmux-rpc クレート. JSON-RPC v2 サーバー, NDJSON フレーミング (LinesCodec), Unix ソケット |
| P3-T2 | `6943619` | RPC メソッドハンドラ + iced subscription 統合. RpcResponder (Arc<Mutex<Option<oneshot::Sender>>>) |
| P3-T3 | `2055082` | CLI クライアント. clap サブコマンド (ping, list-workspaces, notify 等) |
| P3-T4 | `b7e47bf` | 環境変数自動設定. XMUX, XMUX_PANE_ID, XMUX_SOCKET_PATH, TERM, COLORTERM |
| P3-T5 | `0b00107` | ソケットセキュリティ. SO_PEERCRED による peer UID 検証 |

### テスト結果

- 全 54 テスト pass

## 2026-06-13: P4 エージェントフック + git 連携

Workflow ツールで haiku モデルの実装エージェント 4 体 + 検証エージェント 1 体を実行. 約 4.5 分, 112k トークンで完了.

### 実装内容

| タスク | コミット | 概要 |
|---|---|---|
| P4-T1 | `f65aa77` | xmux-agent クレート. AgentRegistry + 組み込みエージェント 5 種 (claude-code, codex, gemini, copilot, amp). detect_agent() |
| P4-T2 | `9b47ae3` | HookInstaller. generate_hook_script(), install_claude_code_hooks(), config_path() |
| P4-T3 | `7faae5c` | GitInfo::from_dir(). git rev-parse + git status --porcelain でブランチ名・dirty 状態取得 |
| P4-T4 | `52b949d` | tmux 互換シム. parse_tmux_command() で tmux コマンドを xmux RPC にマッピング. TMUX 環境変数設定 |
| P4-T5 | `20e0931` | ポート検出. /proc/net/tcp + tcp6 パースで LISTEN ポート検出 |

### テスト結果

- xmux: 45 テスト (input 23 + terminal_view 8 + git_info 2 + tmux_shim 10 + port_monitor 2)
- xmux-agent: 9 テスト (registry 6 + hooks 3)
- xmux-notification: 16 テスト (parser 11 + manager 5)
- xmux-rpc: 5 テスト
- xmux-terminal: 2 テスト
- 全 77 テスト pass

### 設計判断

- GitInfo は同期 std::process::Command で実装 (非同期ポーリングは将来のサイドバー統合時に追加)
- tmux シムは parse_tmux_command() で最小セットのみ対応 (new-session, split-window, send-keys, list-sessions, list-panes, display-message)
- ポート検出は /proc/net/tcp の状態 0A (LISTEN) を直接パース, IPv4/IPv6 両対応

## 2026-06-13: P5 セッション保存/復元 + 描画最適化

Workflow ツールで haiku モデルの実装エージェント 4 体 + 検証エージェント 1 体を実行. 約 4 分, 116k トークンで完了.

### 実装内容

| タスク | コミット | 概要 |
|---|---|---|
| P5-T1 | `f0ec075` | xmux-session クレート. SessionSnapshot, WorkspaceSnapshot, LayoutNode, PaneSnapshot. アトミック保存 (tmp + rename), 最新 5 世代保持 |
| P5-T2 | (同上) | AutoSaver. Arc<AtomicBool> dirty flag, mark_dirty/clear_dirty/is_dirty |
| P5-T3 | (同上) | セッション復元. SessionSnapshot::restore() → RestoreResult. collect_panes() で LayoutNode からペイン列挙 |
| P5-T4 | (同上) | ダメージトラッキング. Terminal::reset_damage() 追加, Tick ハンドラで cache.clear() 後に reset_damage() 呼び出し |

### テスト結果

- xmux: 45 テスト
- xmux-agent: 9 テスト
- xmux-notification: 16 テスト
- xmux-rpc: 5 テスト
- xmux-session: 9 テスト (snapshot 3 + autosave 2 + restore 4)
- xmux-terminal: 2 テスト
- 全 86 テスト pass

### 設計判断

- SessionSnapshot は pane_grid::State を直接シリアライズせず, LayoutNode 中間表現を経由 (pane_grid::State は Serialize 未実装)
- 保存はアトミック: tmp ファイルに書き込み → rename で上書き (POSIX 保証)
- AutoSaver は Relaxed ordering で十分 (dirty flag はベストエフォート)
- ダメージトラッキングは Iced Canvas の制約上, Full/Skip の 2 択. process_events() の Wakeup 検出と組み合わせて不要な再描画を抑制

## 2026-06-13: P6 内蔵ブラウザ

Workflow ツールで haiku モデルの実装エージェント 3 体 + 検証エージェント 1 体を実行. 約 3.5 分, 101k トークンで完了.

### 実装内容

| タスク | コミット | 概要 |
|---|---|---|
| P6-T1 | `63b964b` | xmux-browser クレート. BrowserState, BrowserManager, BrowserCommand, BrowserEvent. trait ベースの抽象化 (wry バックエンドは後付け可能) |
| P6-T2 | (同上) | ブラウザ RPC メソッド (browser.list/open/close/navigate/eval) + CLI コマンド (browser-open/list/navigate/eval/close) |
| P6-T3 | (同上) | UrlDetector. バイトレベル URL 検出 (http/https). 末尾句読点除去, 境界文字処理 |

### テスト結果

- xmux: 45 テスト
- xmux-agent: 9 テスト
- xmux-browser: 14 テスト (webview 8 + integration 6)
- xmux-notification: 16 テスト
- xmux-rpc: 5 テスト
- xmux-session: 9 テスト
- xmux-terminal: 2 テスト
- 全 100 テスト pass

### 設計判断

- wry (WebKitGTK) はビルド環境制約のため直接依存せず, trait ベースの抽象化で実装. BrowserManager がブラウザ状態を管理し, 実際の WebView バックエンドは後から接続可能
- ブラウザ RPC はスタブ応答 (not_implemented) を返す. 実際の wry 統合は GTK イベントループ統合が必要なため, 環境が整った段階で実装
- URL 検出はバイトレベル走査で効率的に実装. OSC 8 ハイパーリンク対応は将来の課題

## 2026-06-13: P7-T1 数式レンダリング

Workflow ツールで haiku モデルの実装エージェント 1 体 + 検証エージェント 1 体を実行. 約 2.5 分, 53k トークンで完了.

### 実装内容

| タスク | コミット | 概要 |
|---|---|---|
| P7-T1 | `4a490d6` | xmux-math クレート. MathDetector ($/$$ 検出), MathRenderer (スタブ + キャッシュ), RenderedMath (RGBA バッファ) |

### テスト結果

- xmux: 45 テスト
- xmux-agent: 9 テスト
- xmux-browser: 14 テスト
- xmux-math: 10 テスト (detector 7 + renderer 3)
- xmux-notification: 16 テスト
- xmux-rpc: 5 テスト
- xmux-session: 9 テスト
- xmux-terminal: 2 テスト
- 全 110 テスト pass

### 設計判断

- RaTeX の parser/layout/render クレートは crates.io 未公開のため, スタブレンダラーで抽象化. 将来 ratex-render が公開されたら差し替え可能
- LaTeX 検出は $...$ (インライン) と $$...$$ (ディスプレイ) をバイトレベルで走査. エスケープ (\$) も考慮
- レンダリング結果は HashMap でキャッシュ (同じ数式の再レンダリング回避)
- OSC 1337;LaTeX= (iTerm2 スタイル) の検出は MathDelimiter enum で対応予定だが, パーサーは未実装
