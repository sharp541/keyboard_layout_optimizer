# T03: キー役割モデルを追加する

## 目的

文字そのものではなく、母音・子音・拡張キーといった役割を物理位置へ割り当てられる表現を追加する。

## 作業内容

- `KeyRole` を定義する
- 母音、子音、`ann / inn / unn / enn / onn`、`ai / uu / ei / ou` を表現できるようにする
- 現行の `LogicalLayout` を拡張するか、新しい役割付きレイアウト型を追加する
- 役割から物理インデックスを引ける API を用意する

## 成果物

- 役割モデル
- 役割付きレイアウト表現

## 変更候補ファイル

- `src/keyboard_layout.rs`
- `src/keyboard_layout/logical_layout.rs`
- `src/input_method/role.rs`

## 完了条件

- 各役割を物理位置へ割り当てられる
- GA 側から扱える形のデータ構造になっている
