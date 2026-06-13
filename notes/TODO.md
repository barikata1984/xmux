# xmux TODO

## P0: 最小ターミナル (5 日)
- [x] P0-T1: Cargo workspace 初期化 (~700 行)
- [x] P0-T2: PTY 生成と I/O ループ (alacritty_terminal::tty + EventLoop)
- [x] P0-T3: alacritty_terminal 統合
- [x] P0-T4: Iced Canvas ターミナル描画 (~450 行)
- [x] P0-T5: キーボード入力処理
- [x] P0-T6: テキスト選択とクリップボード

## P1: 分割ペイン + 縦タブサイドバー (6 日)
- [x] P1-T1: pane_grid 統合
- [x] P1-T2: ワークスペース管理
- [x] P1-T3: 縦タブサイドバー
- [x] P1-T4: リサイズ対応とビューポート計算
- [x] P1-T5: スクロールバック

## P2: 通知システム (5 日)
- [x] P2-T1: OSC シーケンスパーサー (方式 A: バイトストリームタップ)
- [x] P2-T2: 通知マネージャー
- [x] P2-T3: 通知 UI (サイドバーバッジ + 通知パネル)
- [x] P2-T4: xmux notify CLI コマンド

## P3: CLI + JSON-RPC (7 日)
- [x] P3-T1: JSON-RPC v2 サーバー (NDJSON フレーミング)
- [x] P3-T2: RPC メソッドハンドラ (subscription::channel + oneshot)
- [x] P3-T3: CLI クライアント
- [x] P3-T4: 環境変数の自動設定
- [x] P3-T5: セキュリティ (ソケット認証)

## P4: エージェントフック + git 連携 (4 日) ✓ VERIFIED
- [x] P4-T1: エージェントレジストリ
- [x] P4-T2: フック自動インストール
- [x] P4-T3: git ブランチ・PR ステータス表示
- [x] P4-T4: tmux 互換シム
- [x] P4-T5: ポート検出

## P5: セッション + 描画最適化 (4 日) ✓ VERIFIED
- [x] P5-T1: セッションスナップショット (pane_grid::Configuration 経由)
- [x] P5-T2: 自動保存 (dirty flag)
- [x] P5-T3: セッション復元
- [x] P5-T4: ダメージトラッキング描画最適化 (~250 行)

## P6: 内蔵ブラウザ (7 日) ✓ VERIFIED
- [x] P6-T1: wry WebView 統合 (~700 行)
- [x] P6-T2: ブラウザ RPC メソッド
- [x] P6-T3: ブラウザ・ターミナル連携

## P7: 数式 + クロスプラットフォーム (4 日)
- [x] P7-T1: RaTeX 統合 — 数式レンダリング
- [ ] P7-T2: macOS プラットフォーム実装
- [ ] P7-T3: Windows プラットフォーム実装
- [ ] P7-T4: 品質仕上げ

---

合計: 42 日 (旧見積 38 日から +4 日)
