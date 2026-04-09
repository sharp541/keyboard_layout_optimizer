# T10: CLI と検証を更新する

## 目的

新しい評価系を実行・確認できるよう、CLI と検証項目を更新する。

## 作業内容

- `main.rs` で日本語コーパス入力と評価器選択を扱えるようにする
- 必要な引数を整理する
- 既存の `--ja-weight`, `--en-weight` を統合評価器へ接続する
- `cargo test`, `cargo check`, `cargo clippy --all-targets --all-features` で検証する
- README または補助ドキュメントを必要に応じて更新する

## 成果物

- 更新された CLI
- 検証結果
- 必要なら説明文書

## 変更候補ファイル

- `src/main.rs`
- `README.md`

## 完了条件

- 新しい評価器を CLI から実行できる
- 主要コマンドが通る、または未実施理由が明記される
