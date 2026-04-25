//! Issue kako-jun/retro-decode#4 テスト。
//!
//! 既知の小さい入力で first-diff 検出が期待位置・期待 (pos, len) で
//! 停止することを検証する。

use retro_decode::formats::toheart::lf2_tokens::{
    decompress_to_tokens, enumerate_match_candidates, LeafToken,
};
use retro_decode::formats::toheart::okumura_lzss::{compress_okumura, Token as OkuToken};

/// 奥村エンコーダの出力は全リテラルのシーケンスでも同じになる。
/// これをそのまま「Leaf 側の圧縮ペイロードを LF2 framing で組んだ」
/// と仮定すれば、first-diff は発生しない。
#[test]
fn no_divergence_for_pure_literal_stream() {
    // 短い非反復入力 → okumura は全リテラル化 (THRESHOLD=2 未満マッチ扱い)
    let input: Vec<u8> = (0..10u8).collect();
    let oku = compress_okumura(&input);
    assert!(oku.iter().all(|t| matches!(t, OkuToken::Literal(_))));

    // LF2 framing で全リテラルを作る
    let payload = encode_literals(&input);
    let decoded = decompress_to_tokens(&payload, input.len() as u16, 1).unwrap();
    assert_eq!(decoded.tokens.len(), input.len());
    for (i, t) in decoded.tokens.iter().enumerate() {
        match t {
            LeafToken::Literal(b) => assert_eq!(*b, input[i]),
            other => panic!("expected literal, got {:?}", other),
        }
    }
    assert_eq!(decoded.ring_input, input);
}

/// 意図的に Leaf 側は「違う (pos, len) を選んだ」状態の payload を作って
/// 奥村と比較。first-diff が token 0 で検出され、候補列挙関数で奥村の
/// 選択が拾えることを確認する。
///
/// 入力: `[0x20; 20]` （初期 ring が 0x20 なので先頭から最長一致が取れる）
/// 奥村の選択: pos=?, len=18
/// Leaf の選択（本テストが作る）: pos=0x123, len=3
#[test]
fn divergence_detected_at_first_token() {
    let input = vec![0x20u8; 20];
    let oku = compress_okumura(&input);
    // 先頭は必ず Match { len=18 } になる
    match oku[0] {
        OkuToken::Match { len, .. } => assert_eq!(len, 18),
        _ => panic!("expected match"),
    }

    // Leaf 側の payload を「pos=0x123, len=3 のマッチ + 残りリテラル」で作る
    let mut compressed: Vec<u8> = Vec::new();
    // flag = 0b0_1111111 = 0x7f  (MSB=0: 最初のトークンがマッチ、残り 7 トークンはリテラル)
    compressed.push(0x7f ^ 0xff);
    // Match pos=0x123, len=3
    // upper = (len-3) | ((pos & 0x0f) << 4) = 0 | (0x3 << 4) = 0x30
    // lower = (pos >> 4) & 0xff = 0x12
    compressed.push(0x30 ^ 0xff);
    compressed.push(0x12 ^ 0xff);
    // 残り 7 リテラル（全て 0x20）
    for _ in 0..7 {
        compressed.push(0x20 ^ 0xff);
    }
    // まだ 10 ピクセル処理したので 10 足りない。次の flag byte
    compressed.push(0xff ^ 0xff); // flag = 0xff 全リテラル
    for _ in 0..8 {
        compressed.push(0x20 ^ 0xff);
    }
    // 残り 2 ピクセル
    compressed.push(0xff ^ 0xff);
    compressed.push(0x20 ^ 0xff);
    compressed.push(0x20 ^ 0xff);

    let decoded = decompress_to_tokens(&compressed, 20, 1).unwrap();
    // decode 側の最初のトークンは Match { pos=0x123, len=3 }
    assert_eq!(decoded.tokens[0], LeafToken::Match { pos: 0x123, len: 3 });
    assert_eq!(decoded.ring_input, input);

    // Leaf と Okumura の先頭トークンが食い違うことを検証
    let leaf_first = decoded.tokens[0];
    let oku_first = oku[0];
    let differ = match (leaf_first, oku_first) {
        (LeafToken::Match { pos: lp, len: ll }, OkuToken::Match { pos: op, len: ol }) => {
            lp != op || ll != ol
        }
        _ => true,
    };
    assert!(differ, "tokens at index 0 should differ");

    // 候補列挙: 初期 ring (全部 0x20) に対して input (全部 0x20) を
    // クエリすれば、任意の pos で長さ 18 の候補が全部出る。
    let ring = [0x20u8; 0x1000];
    let candidates = enumerate_match_candidates(&ring, &input, 0);
    // Leaf の選択 (pos=0x123, len=3) が候補集合に含まれる
    assert!(candidates
        .iter()
        .any(|c| c.pos == 0x123 && c.len == 3));
    // Okumura の選択 len=18 も含まれる
    assert!(candidates.iter().any(|c| c.len == 18));
}

/// LF2 framing で `input` の全バイトをリテラルトークンとして並べる
/// ヘルパ。テスト用。
fn encode_literals(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < input.len() {
        let end = (i + 8).min(input.len());
        let count = end - i;
        // flag: 上位 count ビットが 1、残りは 0
        let mut flag: u8 = 0;
        for b in 0..count {
            flag |= 1 << (7 - b);
        }
        out.push(flag ^ 0xff);
        for &b in &input[i..end] {
            out.push(b ^ 0xff);
        }
        i = end;
    }
    out
}
