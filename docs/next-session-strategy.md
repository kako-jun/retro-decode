# 次セッション以降の作戦書 (セッション 320 末で凍結)

セッション 320 の対話で明確化された次の作戦と、その背景にある方法論。次回フリーザ起動時はまずこのドキュメントを読み、その上で `ai-route-2026-04-27-v3-to-v8.md` で v8 までの戦況を確認する。

## 方法論の整理 (kako-jun の問いから抽出)

### 我々が持っている中間状態の正解

LF2 は可逆エンコードなので、Leaf 製ファイルを decode すれば:

```
入力 byte 列: [B0, B1, ..., Bn]
正解 token 列 (decode で取得): [T0, T1, ..., Tm]
正解 ring states: [R0, R1, ..., Rm]    ← Leaf がエンコード時に見ていたもの
step k での正解 candidates: 計算可能 (Rk と入力から)
step k で Leaf が選んだ choice: Tk
```

= **任意の中間ステップの正解 (リング状態 + 候補集合 + Leaf 選択) が全 522 ファイル × 全 token で取れる**。これが pairwise CSV の中身であり、ML 訓練の信号源。

### 評価の 3 段階フレームワーク

```
段階 1: per-token 評価       ← 各 step で Rk を正解に固定して評価。現状 v8 で 99.4%
段階 2: per-file per-token 100% ← ファイル単位で全 token 当たる状態。C1001 達成
段階 3: end-to-end byte-exact   ← ML をエンコーダ化、出力 LF2 を Leaf 製と diff
```

**段階 2 → 段階 3 は自動到達**。なぜなら全 token が当たれば cascade 起きず、ring 状態も Leaf と同期するから。C1001 がこれ。

### cascade 問題

段階 1 で 99.4% でも、実エンコードに組み込むと:
- step 0 で 1 個間違える → R1 が Leaf と異なる
- step 1 以降は前提が崩れて評価不能、全 token が芋づる式にズレる
- 出力 LF2 は数 KB 全 diff になる

**つまり per-token 100% (= 段階 2) を達成しないと、実エンコード検証 (段階 3) は意味を成さない**。これが「per-token で詰める」現方式の根拠。

## kako-jun が指摘した未検証仮説 (v9 候補)

### 仮説 A: リング回転回数依存の if

C1001 (50 tokens、リング 0 周、100% 達成) と OC_07 (3000+ tokens、リング複数周、99.29% 止まり) の分布から、**Leaf エンコーダは「リング N 周目以降に発動する if」を持つ可能性**。

OC_07 の 23 miss は全て input_pos 7508-11365 (リング複数周後)。一方 C1001 はリング 1 周もしない範囲で完結。1 周目までは greedy + 縦アラインメントで足りるが、2 周目以降に別の if が発動する可能性。

**追加特徴量案 (token-level)**:
- `wrap_count = input_pos / 4096` (リング何周目か)
- `wrap_phase = input_pos % 4096 / 4096.0` (周回内位相)

### 仮説 B: 画像サイズ依存の if

LF2 ヘッダの img_w, img_h, colors を読んでエンコーダが冒頭で分岐している可能性。現状 img_w は importance rank 6 で使われているが、派生特徴量がない。

**追加特徴量案 (token-level)**:
- `total_pixels = img_w * img_h`
- `size_log = log(total_pixels)`
- `colors_bucket` (pal8/pal16/pal256 で 3 値カテゴリ化)
- `aspect_ratio = img_w / img_h`

### 仮説 C: 交互作用項

決定木は単独特徴量から非線形を学べるが、明示的な交互作用は学習が遅い。

**追加特徴量案 (candidate-level)**:
- `wrap_x_dist = wrap_count * cand_dist`
- `wrap_x_mod_w = wrap_count * cand_dist_mod_w`
- `size_x_max_len = total_pixels * max_len`

## v9 として実装する手順

1. `lf2_pairwise_dataset_v9.rs` を v8 から複製
2. token-level に `wrap_count`, `wrap_phase`, `total_pixels`, `colors_bucket`, `aspect_ratio` 追加 (5 列)
3. candidate-level に `wrap_x_dist`, `wrap_x_mod_w` 追加 (2 列)
4. CSV 53→60 列、522 並列再生成 (~8 分、`/home/kako-jun/work/lf2_pairwise_csvs_v9/` へ)
5. ML スクリプト 26a/26b 作成、BIG (3000 round) で学習・評価
6. 期待: AUC 0.857+ → 0.86+、CWEEK_02 / CSNO_07 → 100%、OC 系列が 99.5% を越えるか観察

### 観察ポイント

- **wrap_count が importance top 10 に入るか**: 入れば仮説 A 的中
- **CWEEK_02 / CSNO_07 が 100% に到達するか**: あと 1 token 系の確実な突破
- **OC_07 が 99.5% を越えるか**: リング ラップ依存の if が捕捉できたかの分岐点
- **完全一致が 1 → 3 以上に増えるか**: メソッドの健全性確認

## v9 で届かない場合の方針転換

仮説 A/B/C を全部入れても OC_07 が動かない場合、**ML 近似の限界 = 真のルールの境界がシャープすぎる**ことが確定する。次は方法論転換:

### α: ツリー蒸留 (rule extraction)
- 学習済み LightGBM の決定パスを解析、score > 0.7 を生成する条件式を抽出
- 抽出した if-then を手動で精緻化 (= Leaf エンコーダ仕様の復刻)
- 確率的揺らぎ排除で per-token 100% を狙う

### β: per-file finetune (D 路線)
- OC 系列だけで追加学習し、過学習で OC 系列の if を学習
- ファイル間で違うルールを使っているなら有効
- ただし「単一エンコーダ仕様」と矛盾する性質、慎重判断

### γ: DP backtrack
- 各ファイルで Leaf 出力を生成可能な全 LZSS パース列挙
- 共通選択ロジックを集合演算で抽出
- 計算量大、kako-rog 並列必須

## 関連メモ

### kako-jun が確認済の制約
- 外部支援禁止: ソース入手・人間コンタクト・Ghidra は選択肢外 (memory `feedback_no_external_help`)
- 100% byte-exact が達成条件、妥協リリース禁止 (memory `feedback_retro_decode_100pct`)
- 「30 分でできる」発言禁止: データ確認後に発言 (memory `feedback_avoid_short_term_promises`)

### Leaf エンコーダの真の関数形 (推定)

```c
// 1996 年 Leaf 内部の推定構造
TokenChoice leaf_pick_candidate(
    Candidate *candidates,    // 候補集合 ← 我々は計算済
    RingBuffer *ring,         // リング状態 ← 我々は復元済
    InputStream *input,       // 入力 + 先読み ← 取れている
    History *prev_tokens,     // 過去 N token ← prev_3 まで取得
    ImageInfo *img,           // w/h/colors ← 取れている
    EncoderState *state       // wrap_count? その他? ← 一部未取得 ← v9 で攻める
) {
    if (specific_condition_A) {  // ← v8 で発見: dist が img_w 倍数
        return rule_X(...);
    } else if (specific_condition_B) {  // ← 未発見
        return rule_Y(...);
    } else if (specific_condition_C) {  // ← 未発見
        return rule_Z(...);
    } else {
        return rule_default(...);
    }
}
```

我々の方法論は「不足している EncoderState の中身を ML が学習しやすい形で漏れなく与える」+「if-then の境界を ML が近似 → 蒸留で抽出」の二段攻め。

## 次セッションの最初の一手

```bash
# 1. status と本ドキュメント確認
cat repos/2025/retro-decode/docs/next-session-strategy.md
cat repos/2025/retro-decode/docs/ai-route-2026-04-27-v3-to-v8.md

# 2. v9 binary 実装開始
cp src/bin/lf2_pairwise_dataset_v8.rs src/bin/lf2_pairwise_dataset_v9.rs
# wrap_count, total_pixels, 交互作用項を追加

# 3. kako-rog で並列生成 + 学習 + 評価
# ML スクリプト ~/work/lf2_ml/26a/26b_v9_binary_big.py を 25a/25b ベースで作成
```

戦況が大きく動いたら本ドキュメントを更新、または `ai-route-2026-04-28-v9-and-beyond.md` を新規作成して履歴を保つ。
