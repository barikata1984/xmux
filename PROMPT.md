xmux プロジェクトの実装を開始する. xmux は cmux (manaflow-ai/cmux) の Linux 向け再現実装で, AI コーディングエージェント向けのターミナルマルチプレクサである.

## 計画ファイル

- 実装計画: /home/atsushi/workspace/xmux/notes/PLAN.md (1900+ 行, 8 フェーズ・33 タスク・42 日)
- タスクリスト: /home/atsushi/workspace/xmux/notes/TODO.md

まず PLAN.md と TODO.md を読み, 全体像を把握すること.

## 確定済みアーキテクチャ

- Rust, Iced 0.14 + alacritty_terminal 0.26 + wgpu
- Linux では alacritty_terminal::tty + EventLoop を直接使用 (portable-pty 不使用)
- FairMutex は alacritty_terminal::sync::FairMutex (parking_lot ではない)
- OSC 9/99/777 通知は EventListener 経由で取得不可 → バイトストリームタップ方式
- RPC フレーミングは NDJSON (tokio_util::codec::LinesCodec)
- RPC ↔ Iced 通信は iced::subscription::channel() + oneshot + 5 秒タイムアウト
- pane_grid::State は Serialize 非対応 → pane_grid::Configuration 経由で変換
- 数式レンダリング: ratex-parser/layout/render (crates.io 公開済み v0.1.11)
- ブラウザ統合: wry + iced_webview_v2 を参考 (P6)

## PLAN.md の既知の残存問題 (実装前に修正すべき)

1. §1 pub API 概要の Terminal メソッドシグネチャが旧版:
   - `input(&self, data: &[u8])` → 実際は `Term::input(c: char)` (char 単位). バイト列は EventLoop 経由
   - `scroll(&self, scroll: Scroll)` → 実際は `Term::scroll_display(scroll: Scroll)`
2. P7 用の PtyHandle に `master: Box<dyn MasterPty>` フィールドが必要 (resize 用)
3. P0 工数が 5 日のまま (6 日が適切, P0-T1 が ~700 行に増加したため)

## 実装の進め方

- P0-T1 (Cargo workspace 初期化) から順に着手
- 各タスク完了後に notes/TODO.md の該当項目を [x] にチェック
- テストは各タスク内で記載された検証方法に従う
- git init してからコミット (Conventional Commits 形式)
- PLAN.md のタスク定義に具体的なコード設計・注意点・ハマりポイントが詳述されているので, 実装前に該当セクションを必ず読むこと

## 最初のタスク

P0-T1 (Cargo workspace 初期化) を実施せよ. PLAN.md の P0-T1 セクションを読み, 以下を生成:
- workspace root Cargo.toml (workspace.dependencies 含む)
- crates/xmux-core/ (types.rs, config.rs, error.rs)
- crates/xmux-platform/ (lib.rs + linux.rs)
- src/main.rs (Iced 0.14 の application() ビルダー)
- git init + 初期コミット

完了後 cargo build --workspace の成功と cargo run での空ウィンドウ表示を確認すること.
