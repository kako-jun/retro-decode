use std::env;
use std::fs;
use std::path::PathBuf;

use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura_eof_retie, EofTieRule, TaxBase, Token,
};
use retro_decode::formats::toheart::verify_harness;

const FILES: &[(&str, TaxBase)] = &[
    ("C0182.LF2", TaxBase::Basic),
    ("C0183.LF2", TaxBase::Basic),
    ("C040E.LF2", TaxBase::Basic),
    ("C040F.LF2", TaxBase::Basic),
    ("C0410.LF2", TaxBase::Basic),
    ("C0411.LF2", TaxBase::Basic),
    ("C0508.LF2", TaxBase::Basic),
    ("C0509.LF2", TaxBase::Basic),
    ("C050A.LF2", TaxBase::Basic),
    ("C0511.LF2", TaxBase::Basic),
    ("C0518.LF2", TaxBase::Basic),
    ("C0805.LF2", TaxBase::Fill00),
    ("C080D.LF2", TaxBase::Fill00),
    ("C1002.LF2", TaxBase::Basic),
    ("C1201.LF2", TaxBase::NoDummy),
    ("C1205.LF2", TaxBase::NoDummy),
    ("C1709.LF2", TaxBase::Basic),
    ("C1E03.LF2", TaxBase::Basic),
    ("C1E05.LF2", TaxBase::Basic),
    ("C1E06.LF2", TaxBase::Basic),
    ("C1E0A.LF2", TaxBase::Basic),
    ("C1E13.LF2", TaxBase::Basic),
    ("C1E16.LF2", TaxBase::Basic),
    ("C1E1A.LF2", TaxBase::Basic),
];

fn frame_payload(tokens: &[Token]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let flag_pos = out.len();
        out.push(0);
        let mut flag_byte: u8 = 0;
        let mut bits_used = 0;
        while bits_used < 8 && i < tokens.len() {
            match tokens[i] {
                Token::Literal(b) => {
                    flag_byte |= 1 << (7 - bits_used);
                    out.push(b ^ 0xff);
                }
                Token::Match { pos, len } => {
                    let p = (pos as usize) & 0x0fff;
                    let l = ((len as usize) - 3) & 0x0f;
                    let upper = (l | ((p & 0x0f) << 4)) as u8;
                    let lower = ((p >> 4) & 0xff) as u8;
                    out.push(upper ^ 0xff);
                    out.push(lower ^ 0xff);
                }
            }
            bits_used += 1;
            i += 1;
        }
        out[flag_pos] = flag_byte ^ 0xff;
    }
    out
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let dir = PathBuf::from(&args[1]);

    for rule in [
        EofTieRule::ClosestDist,
        EofTieRule::FarthestDist,
        EofTieRule::SmallestPos,
        EofTieRule::LargestPos,
        EofTieRule::MostRecentWrite,
        EofTieRule::LeastRecentWrite,
        EofTieRule::MaxPhantomExtension,
        EofTieRule::ClosestToRawPos,
    ] {
        let mut hits = Vec::new();
        for (name, base) in FILES {
            let path = dir.join(name);
            let data = fs::read(&path).unwrap();
            let (w, h, ps) = verify_harness::parse_lf2(&data).unwrap();
            let leaf = retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens(
                &data[ps..],
                w,
                h,
            )
            .unwrap();
            let gen = compress_okumura_eof_retie(&leaf.ring_input, *base, rule);
            let payload = frame_payload(&gen);
            if payload == data[ps..] {
                hits.push(*name);
            }
        }
        println!("{:?} -> {}/24 hits: {:?}", rule, hits.len(), hits);
    }
}
