# AZIK 系拡張の実装タスク

## 目的

`docs/draft.md` の仕様草案を、実装可能な単位へ分解したタスク一覧。
各タスクは個別ファイルを参照する。

## 実装順

1. `docs/tasks/T01-ja-corpus-mora.md`
2. `docs/tasks/T02-ja-corpus-structure.md`
3. `docs/tasks/T03-key-role-model.md`
4. `docs/tasks/T04-azik-decoder.md`
5. `docs/tasks/T05-sequence-cost.md`
6. `docs/tasks/T06-japanese-evaluator.md`
7. `docs/tasks/T07-english-evaluator.md`
8. `docs/tasks/T08-combined-evaluator.md`
9. `docs/tasks/T09-ga-role-layout.md`
10. `docs/tasks/T10-cli-and-validation.md`

## 依存関係

- `T01` は `T02` の前提
- `T02` は `T06` の前提
- `T03` は `T04` と `T09` の前提
- `T04` と `T05` は `T06` の前提
- `T06` と `T07` は `T08` の前提
- `T08` は `T09` の前提
- `T10` は全体の締め

## 分割方針

- データ前処理
- 日本語評価基盤
- 英語評価分離
- 統合評価
- 最適化器接続

の順で切っている。
