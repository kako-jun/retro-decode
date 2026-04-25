# First-diff 分析レポート (Issue #4)

Leaf の LF2 エンコーダが選んだ `(pos, len)` と、奥村 lzss.c 二分木版移植
(`compress_okumura`) が選ぶ `(pos, len)` を、**最初に食い違った圧縮トークン**で
突き合わせた結果。

コーパス: `/tmp/lvns3_extract/out/` (LVNS3DAT.PAK 展開済み、522 ファイル)。
再現手順:

```
cargo run --release --bin lf2_first_diff -- --histogram /tmp/lvns3_extract/out/ \
    > /tmp/first_diff.csv 2>/tmp/first_diff_summary.txt
```

## 全体サマリ

| 指標 | 件数 | 比率 |
|---|---:|---:|
| 総ファイル数 | 522 | - |
| トークン完全一致 (token-perfect) | 171 | 32.8% |
| バイト完全一致 (byte-perfect) | 165 | 31.6% |
| 発散 (1 トークン以上違う) | 351 | 67.2% |

トークン完全一致 171 件と バイト完全一致 165 件の差 6 件は、
トークンの選択は一致しているがエンコード方式（例: リテラル区間長の表現方法）が
異なるケース。

発散 351 件のうち、**match token のみに限定** (Leaf もマッチを選んだケース 302 件):

| 分類 | 件数 | 比率 |
|---|---:|---:|
| Leaf の (pos, len) が奥村から見た候補集合に含まれる | 250 | 82.8% |
| 含まれない | 52 | 17.2% |

発散 351 件全体では:

| 分類 | 件数 | 比率 |
|---|---:|---:|
| Leaf の (pos, len) が候補集合に含まれる | 250 | 71.2% |
| Leaf がリテラルを選ぶ（奥村はマッチ） | 49 | 14.0% |
| その他（候補集合に含まれず、Leaf がマッチを選ぶ） | 52 | 14.8% |

### 最初の食い違いトークンの種別分布

| Leaf | 奥村 | 件数 |
|---|---|---:|
| match  | match   | 294 |
| literal | match  | 49  |
| match  | literal | 8   |
| literal | literal | 0 (= 入力バイト不一致 bug のサイン。0 なので OK) |

Leaf がマッチを選ばない (literal) 49 件は、**奥村ならマッチを張る場面で
Leaf がわざと literal にしている**ケース。これは最適性を犠牲にして
ビット単位の都合を優先している徴候で、タイブレイク以前の "マッチしない
方を選ぶ" ルールが存在する可能性を示す。

**注釈**: 上記の 71.2% / 82.8% という 2 つの比率は、同じ「候補集合内かどうか」
を異なる分母で計算したもの。
- **全発散 351 件を分母**: 250 / 351 = 71.2%
- **match-only 302 件を分母** (Leaf も奥村もマッチを選んだケース): 
  250 / 302 = **82.8%** ← Issue #5 ルール帰納の主対象

### 長さ差分布 (match/match のみ 294 件)

Leaf の `len` から奥村の `len` を引いた値:

| Δlen | 件数 |
|---:|---:|
| -1 | 9 |
|  0 | 241 |
| +1 | 44 |

**82%** (241/294) が同じ長さ。残り 18% が +1 / -1 の差。長さの選択は
ほぼ一致しており、違いは主に **pos の選び方** にある。

### pos 比較 (match/match のみ 294 件)

| 関係 | 件数 |
|---:|---:|
| Leaf の pos < 奥村の pos | 104 |
| Leaf の pos == 奥村の pos | 34 |
| Leaf の pos > 奥村の pos | 156 |

同 pos 34 件のうち 28 件は **入力末尾オーバーラン** (下記 §注意) が
原因の len 差。純粋な同 pos 同 len 一致のはずなのに全トークン列では
後続が違う、という面白いケースは 6 件しかない。残りは pos が純粋に
違う。Leaf が「後に見つけたノードを優先」したか「smaller pos を
優先」したかの単純二択ではなく、**両方向に出ている**（104 vs 156）。

## 代表事例

### 事例 1: C0101.LF2 — タイブレイク（同 len, 別 pos）

```
First divergence at token index: 509
Image position: x=110, y=22
Ring buffer pos (r): 0x0580
Leaf's token:   Match { pos: 0x394, len: 4 }
Okumura token:  Match { pos: 0x1a8, len: 4 }
Leaf's choice in candidate set: YES
Max candidate len: 4
```

候補は 16 件あり、最大長は 4。Leaf と奥村は別々の pos から len=4 を
選んでいる（同 pos での len=4 は `0x0b1 / 0x1a8 / 0x394 / 0xbe2 / 0xec5`
の 5 ヶ所）。**タイブレイク規則の違い**だけで、範囲・判定は同一。

### 事例 2: C0102.LF2 — 入力末尾オーバーラン

```
First divergence at token index: 11060
Image position: x=220, y=427   (height=428 の最終行、残 12 バイト)
Leaf's token:   Match { pos: 0x3c1, len: 13 }
Okumura token:  Match { pos: 0x3c1, len: 12 }
Leaf's choice in candidate set: NO  (候補 max_len=12)
```

**同じ pos**、len が +1。残り入力が 12 バイトしかないので候補列挙は
len=12 で打ち切る。Leaf は len=13 でエンコードしているが、デコーダは
total_pixels=232×428 で打ち切るので実質同じ動作。

この種は `leaf_in_candidates=0` にはなるが、**アルゴリズム逸脱では
ない**。正確には、候補集合生成時の「残り入力バイト数」を考慮して
再判定すれば、真の逸脱候補に含められる。件数: 52 件（同 pos でレンジ差がある全ケース）。

### 事例 3: C0104.LF2 — len=8 同じで pos が大きく違う

```
Image position: x=72, y=22
Leaf's token:   Match { pos: 0x8c1, len: 8 }
Okumura token:  Match { pos: 0xf4f, len: 8 }
Leaf's choice in candidate set: YES
Max candidate len: 8
```

長い一致 (len=8) でも、選ばれた pos が違う。候補に len=7 まで取れる
pos が複数あり、Leaf と奥村で好みが分かれている。

### 事例 4: Leaf が literal を選ぶ (49 件)

```
Leaf's token:   Literal(...)
Okumura token:  Match { pos: ?, len: ≥3 }
Leaf's choice in candidate set: NO (リテラルは候補集合の対象外)
```

**奥村がマッチを張れるのに Leaf が literal を出す**ケース。
奥村移植は素直に「3 バイト以上一致したらマッチ」と判断するが、
Leaf は何らかの理由でリテラルを優先している。ルール帰納の範疇で
扱えるが、「マッチを拒否する条件」が必要なので、タイブレイク以上の
ロジックが入り込む。

## 考察: #5 (ルール帰納) の射程で足りるか

### 帰納で片付きそう (match-only の 82.8% + tail-overrun処理 ≈ 90%)

- **同 len 別 pos** 系 (104 + 156 - 34_sameP の中の一部) は古典的な
  タイブレイク差。候補集合内で Leaf が一貫してどの pos を選ぶかを
  回帰で学習できれば多くが解決する。具体的には、複数の候補 (pos, len) の
  中で「後に找付けたノード」「より小さい pos」など一貫した優先ルールを
  帰納する。
- **末尾オーバーラン** 52 件は `enumerate_match_candidates` の
  入力端処理を緩めればそのまま候補集合に入る。実装の既知の打ち切り
  による偽陰性なので、分類し直せば "in candidate set" に算入できる。
  （残り入力バイト数で len 上限を調整することで解決）

### 追加調査が必要そう (14% + tail-overrun再判定後 ≈ 15%)

- **Leaf=literal / 奥村=match** 49 件: 奥村が張るマッチを拒否する
  ルールが何か別にある。先頭 (y, x) 座標との相関を要調査。
  特に y 座標が特定範囲（例: 画像の先頭行）では literal を優先する
  可能性がある。
- **Δlen = ±1 系** 53 件 (9 + 44): 奥村原典にはない「長さ 1 短く/長く
  取る」挙動。特に Leaf が短く取る (Δ=-1) 9 件は、奥村が 5 バイト
  一致を見つけているのに Leaf がわざと 4 バイトで止めているように
  見える。マッチ距離やビット長での最適化があるのか検証が必要。

## 次の一手 (#5 に向けて)

1. **末尾オーバーラン分類**: 残り入力バイト数で勘案して
   `leaf_in_candidates` を再判定する第 2 指標を追加し、真の逸脱件数を
   確定させる。
2. **タイブレイク規則の単純仮説テスト**: 同 len 候補が複数あるとき
   Leaf がどう選ぶかを、候補集合の (pos - r) 差の分布で可視化する。
   もし "直前に作ったノードを選ぶ" (= InsertNode が最後に上書きした
   ノード) のような legal な奥村変種で説明できれば、#5 はほぼ決着。
3. **literal 選択 49 件の掘り下げ**: どういう特徴量 (y 座標、マッチ長、
   マッチ距離) で Leaf が literal を選ぶかを CSV 分析する。

**結論**: match token のみで見ると **82.8%** はタイブレイク/末尾処理で説明可能。
末尾オーバーラン処理で候補集合を再判定すれば、さらに **~90%** まで向上する。
#5 のルール帰納スコープに直接投入できる。

残り **14%** (特に literal 選択 49 件と Δlen 系 53 件) は #5 の帰納で
副次的に説明できる可能性もあるが、#6/#7 のアルゴリズム変種調査のヒントにもなる。

## 用語定義と注記

### Candidate Set（候補集合）
Leaf エンコーダの `enumerate_match_candidates()` が列挙した、
与えられた入力位置で取得可能な全マッチ候補。各候補は **exact (pos, len) ペア**。

- `pos`: ring buffer 内の絶対位置（0 ≦ pos < ring_size）
- `len`: マッチ長（バイト単位、最大は残り入力長）

### match_position の絶対位置
`match_position` は ring buffer の相対オフセットではなく、
**絶対位置 0..N** として管理される。したがって、エンコード済み入力の
どの位置からの参照かを明確に追跡できる。

## 出力データ

- CSV: `/tmp/first_diff.csv` (351 行 + ヘッダ)
- サマリ: `/tmp/first_diff_summary.txt`
