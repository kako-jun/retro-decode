"""522 ファイル全部で leaf decode → re-frame → trailing byte patch して
binary 完全一致を達成する。

これは「known LEAF LF2 files の round-trip identity」の完成版。
新画像エンコードには使えないが retro-decode の round-trip 検証に必須。
"""

import struct
from pathlib import Path
import sys

sys.path.insert(0, '.')
from hybrid_encoder import decode_full_tokens, frame_payload

LF2_DIR = Path("/mnt/hdd6tb/lf2_archive/先人のお手本/Windows95版のTo Heart/LVNS3DAT")


def round_trip_with_phantom(lf2_bytes):
    """Decode tokens + frame. If orig is longer, infer phantom Literal tokens
    from orig trailing bytes and re-frame."""
    cc = lf2_bytes[0x16]
    payload_start = 0x18 + cc * 3
    tokens, _ = decode_full_tokens(lf2_bytes)
    new_payload = frame_payload(tokens)
    orig_payload = lf2_bytes[payload_start:]

    if new_payload == orig_payload:
        return new_payload

    # Try adding phantom Literal tokens until size matches
    # (the encoder fills a partial last group with phantom Literals)
    diff_size = len(orig_payload) - len(new_payload)
    if diff_size <= 0:
        return new_payload  # can't fix

    # Compute how many tokens are in last partial group
    # If tokens.len() % 8 != 0, last group has tokens.len() % 8 tokens
    # We need to add (8 - last_group_size) phantom tokens to fill, but we don't
    # need to fill all 8 -- just the actual trailing data delta.
    # Each phantom Literal adds 1 byte. Each phantom Match adds 2 bytes. Plus 0
    # if last group already had its flag byte, +1 if a NEW flag byte starts.

    # We add phantom Literal tokens. Each adds:
    # - 1 byte (data) if last group has space
    # - 2 bytes (new flag + 1 data) if starting a new group
    # Adding 1 phantom Match adds exactly 2 bytes.
    # diff_size=1: add 1 Literal phantom
    # diff_size=2: add 2 Literals or 1 Match
    # diff_size=3: 3 Literals or 1 Match + 1 Literal
    # ...

    # Try N=1..diff_size phantom Literals; each contributes 1 byte (data) +
    # possibly 1 byte (new flag) if crossing group boundary.
    # Find N such that framed length matches orig length.
    last_group_size = len(tokens) % 8
    for n_phantom in range(1, diff_size + 2):
        # Compute resulting framed length
        # = pure_new_len + n_phantom (data bytes) + #new_flag_bytes
        # #new_flag_bytes = number of times we cross group boundary
        # If last_group_size == 0, first phantom needs new flag → 1 new flag
        # then every 8 phantoms after needs another new flag
        if last_group_size == 0:
            new_flags = 1 + max(0, (n_phantom - 1)) // 8
        else:
            space = 8 - last_group_size
            new_flags = max(0, (n_phantom - space + 7) // 8) if n_phantom > space else 0
        target_len_diff = n_phantom + new_flags
        if target_len_diff != diff_size:
            continue

        # Compute byte position of each phantom literal's data byte
        # Walk the framing simulation to find them
        extra_tokens = list(tokens)
        for _ in range(n_phantom):
            extra_tokens.append(("L", 0, 1))
        # Frame with placeholder Literal(0)
        framed_dummy = frame_payload(extra_tokens)
        # Find positions where framed_dummy differs from orig (= phantom data bytes)
        diff_positions = []
        for i in range(len(new_payload), len(framed_dummy)):
            # These bytes are NEW. Some are flag bytes, some are data bytes.
            # framed_dummy has dummy literal data of (0 ^ 0xff = 0xff)
            # For flag bytes, they should match orig (since both add same flag).
            if framed_dummy[i] == 0xff and i < len(orig_payload):
                diff_positions.append(i)

        # Replace dummy literals with values derived from orig at those positions
        actual_tokens = list(tokens)
        for j in range(n_phantom):
            if j < len(diff_positions):
                pos = diff_positions[j]
                lit_val = orig_payload[pos] ^ 0xff
                actual_tokens.append(("L", lit_val, 1))
            else:
                actual_tokens.append(("L", 0, 1))
        framed_actual = frame_payload(actual_tokens)
        if framed_actual == orig_payload:
            return framed_actual

    return new_payload  # all attempts failed


def main():
    n_total = 0
    n_match = 0
    n_match_after_patch = 0
    fails = []
    for p in sorted(LF2_DIR.glob("*.LF2")):
        d = p.read_bytes()
        if d[:8] != b'LEAF256\0':
            continue
        n_total += 1

        cc = d[0x16]
        ps = 0x18 + cc * 3
        orig_payload = d[ps:]

        tokens, _ = decode_full_tokens(d)
        pure_new = frame_payload(tokens)

        if pure_new == orig_payload:
            n_match += 1
            n_match_after_patch += 1
        else:
            patched = round_trip_with_phantom(d)
            if patched == orig_payload:
                n_match_after_patch += 1
            else:
                fails.append(p.name)

    print(f"=== Round-Trip Summary ===")
    print(f"total: {n_total}")
    print(f"pure round-trip match: {n_match}")
    print(f"after phantom-token patch: {n_match_after_patch}")
    if fails:
        print(f"still failed ({len(fails)}):")
        for f in fails[:20]:
            print(f"  {f}")


if __name__ == "__main__":
    main()
