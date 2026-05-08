# LF2 Encoder Acquisition Investigation (session 386, 2026-05-08)

## 結論

**公開された LF2 encoder は世界中どこにも存在しない。retro-decode の 251/522 (48.08%) byte-identical 実装が、公開可能な情報で確認できる世界で最も進んだ LF2 encoder である。**

## 調査経緯

### 1. 実 CD-ROM から runtime EXE 取得

| binary | 出所 | サイズ | 結果 |
|---|---|---|---|
| `lvns3.ex` | TO_HEART CD (1997-09-30 ISO9660) | 207,872 bytes PE32 | 0xfee LZSS init を 3 箇所発見、**全て decoder**。SAV writer も raw checksum のみで LZSS なし |
| `kizuato.exe` | KIZUATO CD (1996-07-30) SZDD 圧縮 → msexpand 展開 | 108,544 bytes PE32 | 同様に decoder のみ |
| `inst.exe` | TO_HEART CD | 83,456 bytes PE32 | LZSS なし (installer) |
| `mkfont.ex` | TO_HEART CD | 36,352 bytes PE32 | フォント作成ツール、LF2 encoder なし |

**結論**: LF2 encoder はゲーム runtime に含まれない。Leaf 社内のアセットビルドツール (= `LFENC.EXE` 相当) で生成され、retail 出荷 CD には決して含まれない。

### 2. Internet Archive rr3 RAR 検証

- `archive.org/details/LeafToHeart` から `[18禁ゲーム][970523][Leaf]To Heart(rr3).rar` (525MB) を取得
- MDF/MDS Alcohol 120% raw CD イメージ、736 MiB
- データ部 (39 MiB, 22 ファイル) は実 CD バックアップと完全一致
- 差分 ~700 MiB は CD-DA サウンドトラック
- **開発ツール無し**

### 3. 公開エンコーダ世界調査 (fork agent)

duration 168 秒、tool_uses 40 で広範に検索。

**Confirmed dead-end (decoder のみ)**:

- `catmirrors/xlvns` (Go Watanabe 1999-2000) — `lf2.c`/`lfg.c` decoder
- `laqieer/lfview` (T.F. lfview fork) — image loader、archived
- `vn-tools/arc_unpacker` — maintainer rr- が encoder を明示的に拒否、
  `src/dec/leaf/common/custom_lzss.cc` も decoder のみ
- `morkt/GARbro` — LFG/LGF/BJR/PX クラスはあるが **`ImageLF2.cs` 自体が無い**、
  `LfgFormat.Write` は `NotImplementedException` throw
- `jeeb/aquaplus-sources` (Aquaplus 公式 GPL 開示) — ToHeart2 (2004+) 以降のみ、
  1996-1997 LVNS 期は対象外 (XviD GPL 違反対応で開示された範囲のみ)
- FreshBSD `graphics/leafpak` — 抽出のみ
- ToHeart fan-translation (Seven Nights, Dakkodango cohost) — テキスト/スクリプト抽出が主

**ノイズフィルタ**: GitHub の "LF2" ヒットの大半は **Marti Wong の格闘ゲーム
「Little Fighter 2」**の `.dat` 形式 (azriel91/lf2_codec, JohnDoeAntler/LF2-editor 等)。
すべて Leaf VNS とは無関係。

### 4. arc_unpacker の `custom_lzss.cc` 仕様確認

私たちの実装との照合:

| 項目 | arc_unpacker (decoder) | retro-decode | 一致 |
|---|---|---|---|
| dict サイズ | `0x1000` | `N = 4096` | ✓ |
| 開始位置 | `0xFEE` | `N - F = 4078 = 0xFEE` | ✓ |
| dict 初期値 | `{0}` (= 0x00) | `0x20` | ✗ (LFG 用、LF2 は xlvns で 0x20 と確認) |
| flag XOR | `~*input_ptr` (bitwise NOT) | `^ 0xff` | ✓ (等価) |
| match encoding | 12-bit pos + 4-bit (len-3) | 同 | ✓ |
| match length offset | `+3` | `+3` (THRESHOLD=2 で実質 len ≥ 3) | ✓ |

**decoder の仕様は完全に既知。encoder の挙動 (tie-break, lazy match policy, phantom padding) のヒントは含まれない**。

## 残作業の現実路線

1. **Mode B (TIE_DIFF) 214 ファイル攻略**: 単純 tie 規則 (max_pos / max_dist /
   min_dist / hash_chain) は全て standalone 0 binary match で頓挫。次は
   per-file ML overfit から共通 tie 規則抽出 (decision tree depth 3-5 の学習)、
   kako-rog Windows 16GB + RTX 3050 Ti で長期実行。session 375 の M22 路線続編

2. **Leaf 開発バイナリ流出ルート**: 以下 3 つに問い合わせ
   - **morkt** (GARbro author) — `ImageLF2.cs` 欠落を埋められる立場、Leaf 形式網羅
   - **rr-** (arc_unpacker maintainer) — encoder 否定派だがコミュニティ知識豊富
   - **Dakkodango Translations forum** (dakkodango.com/forums/) — 英語で Leaf engine
     内部に詳しい唯一のコミュニティ

3. **「世界最高公開実装」をリリース・宣伝として活用**: 進捗を諦めず、retro-decode
   の現状 251/522 を「公開する作品」として Zenn/Qiita 記事化、README で
   "world's most advanced public LF2 encoder" と謳う

## 永続化価値ある学び

1. **Visual novel runtime に encoder は含まれない (常識)**: アセットビルドは社内
   ツールで行い、出荷 CD には decoder + 圧縮済アセット + パッカー (PAK 抽出) のみ
2. **Internet Archive 等の retail re-release も LFENC 流出源にはならない**: rr3 RAR
   は CD-DA 込みの完全 CD イメージだが、開発ツールは含まれない
3. **オープンソース VN tool エコシステムは抽出側に強く偏る**: GARbro / arc_unpacker /
   Susie plugin / xlvns / lfview 全て decoder/extractor。encoder は需要が少なく
   (= ファン翻訳でも image 置換は実装されない)、誰も書かない
4. **「LF2」キーワードは Little Fighter 2 (格闘ゲーム) のノイズが極めて多い**:
   検索時は "LEAF256" "LVNS" "ToHeart" "Aquaplus" 等で絞り込みが必須
