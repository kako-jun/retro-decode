# 奥村 LZSS 枠の限界解析

旧 memory `feedback_okumura_lzss_dead_end` の技術詳細を移行。
**結論**: 奥村 LZSS 枠の小変更 (dummy 配置・THRESHOLD・tie 規則・サイズ判定・BST 構造) をどう組合せても **224/522 (42.9%) が天井**、純粋 greedy 枠では **14/522 (2.68%)** が天井。これ以上は枠外の構造または ML が必要。

## 16 変種の天井確定 (セッション 296-298)

集合演算 + 19 変種ハイブリッドで確定:

- dummy 配置 5 変種: 全部 215-224 帯
- THRESHOLD=3 (min len=4): 0/522 全壊 (Leaf は 3-長 Match を使う)
- 動的 tie 規則 (短マッチ AllowEq): 0/522 全壊 (Leaf は短マッチでも StrictGt)
- サイズ判定方式 (min_bytes_strict): 198/522 で +8 -25 (半分しか正しくない)
- Leaf token 0 = Literal が 99.2% (518/522) = 単純 token 0 救済は方向違い

### BST 構造変種 (セッション 298)

- `no_dummy_left_first` (cmp init -1 + tie 左): 215/522 = no_dummy と完全一致集合 (新規 0/失った 0)
- `no_dummy_no_swap` (swap-with-r 抑制): 1/522 全壊
- `dummy_no_swap` (F dummy + swap 抑制): 1/522 全壊
- 19 変種ハイブリッド天井依然 **224 で不変**

## LeftFirst が無意味な理由 (構造的洞察)

奥村 BST の inner loop で cmp は `key[i] - text_buf[p+i]` から再計算され、`cmp != 0` で break。`cmp == 0` は F バイト全一致 → outer loop も break。つまり cmp == 0 は次 outer iteration の direction 判定に使われない。`cmp >= 0` vs `cmp > 0` の差は cmp == 0 でのみ生じるが、それは次 iteration に持ち越されない。最初の root 遷移 (cmp init) 以外、Standard と LeftFirst は完全に同じ動作。

## NoSwap が壊れる理由

新ノード r を BST に入れないと後続の長マッチ探索で r の内容を参照できない → 長さの推移が壊れる。Leaf が後続マッチを取れる事実と乖離。

## 純粋 greedy 枠の天井確定 (セッション 316、commit `d140740`)

3 軸 (threshold × init_match × lazy) 全 8 組合せを `lf2_oku_variants` で測定:

| threshold | init_match | lazy  | feasible/522 |
|-----------|------------|-------|--------------|
| 2         | true       | false | 0  (0.00%)   |
| 2         | true       | true  | 1  (0.19%)   |
| 2         | false      | false | 13 (2.49%)   |
| 3         | false      | false | **14 (2.68%)** |
| 3         | false      | true  | 14 (2.68%)   |

**純粋 greedy 枠の天井は 14/522**。224 byte-match との差分 (210 ファイル) は **BST/dummy 軸操作によって偶然合っている**だけで、純粋 greedy では原理的に到達できない。lazy match 単独は無効 (gain=+0 が 213 件あり、+1 の 258 件と拮抗 = 単一基準では捕捉不能)。残り 508 ファイルは AI で押し上げる以外の道がない。

## Ghidra 路線は捨てる

LVNS3.EXE 内 cmp 17 はゲームロジック由来でハズレ確認、MKFONT.EXE は KNJ (無圧縮) 生成ツールで encoder ではない確定。残り関数の総当たり読みは時間対効果が悪い。

## 残された手段

1. **特徴量豊富化 + 機械学習 (現主戦場)**: `docs/ai-route-2026-04-27-v3-to-v8.md` 参照
2. **DP backtrack**: 各ファイルで Leaf 出力を生成可能な全 LZSS パース列挙、共通選択ロジック抽出
3. **完全別構造実装**: hash chain LZ77 / suffix array / lazy DP encoder

## 警告の射程

この解析の警告 (奥村変種を増やすな) は **「奥村 LZSS 枠内の構造変更」のみに適用**、AI 路線 (CSV+ML) には適用しない。両者を混同しないこと。
