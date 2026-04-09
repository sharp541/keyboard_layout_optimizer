# AZIK拡張対応 速度改善タスク

## 目的

`docs/azik_implementation_tasks.md` で実装した範囲を前提に、計算速度の改善余地を効果順で整理する。

主対象は次の 2 系統。

- 最適化本体の fitness 評価コスト
- AZIK 拡張割当まわりの余分な割り当てと探索コスト

## ステータス凡例

- `TODO`: 未着手
- `DOING`: 作業中
- `BLOCKED`: 依存待ち
- `DONE`: 完了

## 優先度付きタスク一覧

| Priority | ID | Status | Task | Main files | Expected impact |
| --- | --- | --- | --- | --- | --- |
| P0 | P01 | DONE | 評価並列化の一層化 | `src/algorithms/genetic.rs`, `src/keyboard_layout/logical_layout.rs` | 大 |
| P1 | P02 | DONE | tri-gram 評価データの lookup 定数化 | `src/algorithms/genetic.rs`, `src/keyboard_layout/logical_layout.rs`, `src/azik_extension.rs` | 大 |
| P2 | P03 | DONE | 拡張割当操作の配列化 | `src/algorithms/genetic.rs`, `src/keyboard_layout/logical_layout.rs` | 中〜大 |
| P3 | P04 | TODO | 全件再評価の削減 | `src/algorithms/genetic.rs` | 中〜大 |
| P4 | P05 | TODO | crossover / mutation の一時 allocation 削減 | `src/algorithms/genetic.rs` | 中 |
| P5 | P06 | TODO | NGramDB 生成時の文字列 allocation 削減 | `src/n_gram.rs`, `src/japanese_preprocessor.rs` | 中 |
| P6 | P07 | TODO | 計測基盤とベンチマーク追加 | `src/algorithms/genetic.rs`, `src/main.rs`, `benches/` 追加可 | 中 |

## 推奨実装順

1. P01
2. P02
3. P03
4. P07
5. P05
6. P04
7. P06

## タスク詳細

### P01 評価並列化の一層化

- Status: `DONE`
- 目的: Rayon の多重並列を避け、スレッド分割の overhead を減らす
- 背景:
- 現状は「島ごと並列」「個体ごと並列」「tri-gram ごと並列」が重なっている
- 実際のホットパスは評価処理なので、並列粒度は 1 層に絞るほうが安定しやすい
- 作業項目:
- `islands.par_iter_mut()` と `population.par_iter_mut()` と `evaluate_ids().par_iter()` の責務を整理する
- まずは `evaluate_ids()` を逐次化し、島または個体単位のどちらか 1 層だけ並列にする
- 実測して最も速い構成を採用する
- 完了条件:
- 並列化が 1 層に整理されている
- `cargo run --release` 相当の実測で改善前より遅くならない
- 実装メモ:
- `islands.par_iter_mut()` を唯一の Rayon 並列化ポイントとして残し、`population` 評価ループと `LogicalLayout::evaluate_ids()` は逐次化した
- これにより島並列 + 個体並列 + tri-gram 並列の多重ネストを解消した

### P02 tri-gram 評価データの lookup 定数化

- Status: `DONE`
- 目的: 評価ホットパスから hash lookup を減らす
- 背景:
- 通常文字は `CharId` 化されているが、AZIK 拡張トークンは親キー解決に map lookup を使っている
- tri-gram 数は多く、1 要素あたりの小さな分岐削減が効く
- 作業項目:
- AZIK 拡張トークンを `char` ではなく固定 index で扱えるように整理する
- `LayoutLookup` を固定長の軽量表現へ寄せる
- `LogicalLayout` 側で拡張親キー参照を `HashMap<char, usize>` から固定長配列へ置き換える
- 完了条件:
- 評価ホットパスで AZIK 拡張解決に hash lookup を使わない
- 既存の評価系テストが通る
- 実装メモ:
- `tri_grams_to_ids()` で AZIK 拡張トークンを `LayoutLookup::AzikExtension` に変換し、評価データ側で `char` lookup を残さないようにした
- `LogicalLayout` の拡張親参照は `HashMap<char, usize>` ではなく固定長 `extension_parent_indices` 配列へ置き換えた
- これにより `evaluate_ids()` の AZIK 拡張解決は token index から直接親キー index を読むだけになった

### P03 拡張割当操作の配列化

- Status: `DONE`
- 目的: 9 個固定の拡張割当処理から `HashMap`/`HashSet`/`Vec` 再構築を外す
- 背景:
- extension mutation / repair / crossover は固定サイズ問題に対して汎用コレクションを使っている
- 割当数が少ないので配列と bitset のほうが適している
- 作業項目:
- `extension_assignments()` 依存の処理を固定長配列ベースへ寄せる
- occupied host を bitset または `bool` 配列で管理する
- token -> host index、host index -> token の表現を整理する
- 完了条件:
- 拡張割当操作で主要な一時 `HashMap`/`HashSet` 生成がなくなる
- GA の制約テストがそのまま通る
- 実装メモ:
- `LogicalLayout` の拡張保持を `HashMap<usize, AzikExtensionToken>` から固定長 `extension_by_index` / `extension_parent_indices` 配列へ変更した
- `extension_mutation` / `repair_extensions` / `crossover_extension_assignments` は token-indexed 配列と occupied `bool` 配列で処理し、一時 `HashMap` / `HashSet` / `Vec` 再構築を外した
- `base_crossover` の cycle visited も固定長配列へ置き換え、世代内の短命 allocation を追加で削減した

### P04 全件再評価の削減

- Status: `TODO`
- 目的: 突然変異や交叉後に毎回全 tri-gram を再評価する設計を見直す
- 背景:
- 現状は個体ごとに毎回 JA/EN の全 tri-gram を走査している
- ここが最終的な最大ボトルネックになりやすい
- 作業項目:
- base mutation の影響キーだけを使った差分評価の可能性を検討する
- extension mutation の影響が親キー解決だけに限られる点を利用できるか確認する
- full reevaluation と incremental reevaluation を切り替えられる形にする
- 完了条件:
- 少なくとも一部の mutation で差分評価が導入されている
- スコア一致テストまたは比較検証がある

### P05 crossover / mutation の一時 allocation 削減

- Status: `TODO`
- 目的: 世代ごとの小さな allocation を減らす
- 背景:
- `thread_rng()` の都度生成、`visited` ベクタ、assignment 再構築ベクタなどが世代内で頻繁に発生している
- 単体では小さくても反復回数が大きい
- 作業項目:
- RNG を島スレッド単位で使い回す
- cycle crossover の訪問管理を固定長配列へ置き換える
- child 生成時の一時ベクタ再利用を検討する
- 完了条件:
- GA の内部で不要な短命 allocation が減っている
- 挙動が既存テストと一致する

### P06 NGramDB 生成時の文字列 allocation 削減

- Status: `TODO`
- 目的: DB 再生成の前処理コストを下げる
- 背景:
- `generate_n_grams` は `Vec<String>` を構築しており、前処理器側でも token 判定時に毎回 `Vec<char>` を作る
- 通常実行頻度は低いが、DB 再構築時には目立つ
- 作業項目:
- `generate_n_grams` を集計直結のストリーミング処理に置き換える
- `longest_matching_extension` の token 判定を定数テーブル化する
- `String` 生成回数を最小化する
- 完了条件:
- DB 構築時に全文 `Vec<String>` を保持しない
- 日本語前処理テストと NGramDB テストが通る

### P07 計測基盤とベンチマーク追加

- Status: `TODO`
- 目的: 速度改善の効果を比較できるようにする
- 背景:
- 最適化は乱数依存で、体感だけでは改善判断が難しい
- 並列粒度変更は環境差も出やすい
- 作業項目:
- 評価関数単体のベンチマークを追加する
- 代表的な mutation / crossover のマイクロベンチを追加する
- `main` とは別に短い固定反復で比較しやすい実行経路を用意する
- 完了条件:
- 少なくとも評価関数の before/after を比較できる
- 今後の最適化で退行検知に使える

## 実装メモ

- まず P01-P03 を優先する。これらは設計変更の割に効果が見込みやすい
- P04 は効果が大きい可能性があるが、正しさ確認コストも高いので P07 とセットで進める
- P06 は最適化本体ではなく DB 構築時間向けなので後回しでよい

## 最小マイルストーン

### M1 ホットパス整理

- P01
- P02

### M2 AZIK 固有処理の軽量化

- P03
- P05

### M3 評価戦略の改善

- P07
- P04

### M4 補助処理の改善

- P06
