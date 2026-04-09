# AGENTS.md

このファイルはポインタ専用。現状説明や詳細なスタイルガイドは書かない。
不足する文脈はコードと設定を読むこと。ポインタが壊れていたら修正を優先する。

## Routing
- Rust の依存関係とエントリポイントは `Cargo.toml` と `src/main.rs` を読む。
- 公開 API と主要モジュールの入口は `src/lib.rs` を読む。
- 最適化ロジックは `src/algorithms/genetic.rs` を読む。
- レイアウトモデルは `src/keyboard_layout.rs` と `src/keyboard_layout/` 以下を読む。
- n-gram と既存テストは `src/n_gram.rs` を読む。
- 追加の背景が必要なら `README.md` だけ確認する。

## Commands
- Build: `cargo check`
- Test: `cargo test`
- Format: `cargo fmt --all`
- Lint: `cargo clippy --all-targets --all-features`
- Run optimizer: `cargo run --release`

## Guardrails
- 実装と矛盾する説明をこのファイルへ追記しない。真実のソースはコードとテスト。
- 手動整形で差分を作らない。`cargo fmt --all` の結果を優先する。
- Clippy 警告を無言で増やさない。`cargo clippy --all-targets --all-features` で確認する。
- テストを壊したまま完了扱いにしない。`cargo test` を通すか、未実施理由を明記する。
- 存在しないディレクトリや運用ルールを参照しない。参照先は実在パスだけにする。
