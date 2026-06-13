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
