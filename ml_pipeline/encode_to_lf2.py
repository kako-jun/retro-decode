"""ML model 経由で LF2 bytes を生成し binary 一致を検証。

LF2 ペイロードを実際に組み立てて元バイトと比較する。
"""

import struct
import sys
from pathlib import Path

import lightgbm as lgb
import numpy as np
import polars as pl

FNAME = sys.argv[1] if len(sys.argv) > 1 else "C0101"
LF2_PATH = Path(f"/mnt/hdd6tb/lf2_archive/先人のお手本/Windows95版のTo Heart/LVNS3DAT/{FNAME}.LF2")
CSV_PATH = Path(f"/tmp/lf2_ml_test/{FNAME}_v8.csv")
if not CSV_PATH.exists():
    # try with lowercase
    alt = Path(f"/tmp/lf2_ml_test/{FNAME.lower()}_v8.csv")
    if alt.exists():
        CSV_PATH = alt

NON_FEATURE_COLS = {"file", "token_idx", "is_leaf"}


def load_csv():
    df = pl.read_csv(CSV_PATH)
    str_cols = [c for c in df.columns if df[c].dtype == pl.Utf8 and c != "file"]
    for c in str_cols:
        df = df.with_columns(
            pl.when(pl.col(c) == "L").then(0)
            .when(pl.col(c) == "M").then(1)
            .otherwise(-1).alias(c)
        )
    return df


def train_model(df):
    feature_cols = [c for c in df.columns if c not in NON_FEATURE_COLS]
    X = df.select(feature_cols).to_numpy()
    y = df["is_leaf"].to_numpy()
    groups = df.group_by(["file", "token_idx"], maintain_order=True).agg(pl.len().alias("g"))["g"].to_list()
    train_set = lgb.Dataset(X, label=y, group=groups)
    params = {
        "objective": "lambdarank", "metric": "ndcg", "ndcg_at": [1],
        "learning_rate": 0.1, "num_leaves": 4096, "min_data_in_leaf": 1,
        "max_depth": -1, "verbosity": -1,
    }
    model = lgb.train(params, train_set, num_boost_round=3000)
    return model, feature_cols


def model_predicted_token_at(df, model, feature_cols, token_idx):
    """Get model's predicted (pos, len) for given token_idx."""
    df_t = df.filter(pl.col("token_idx") == token_idx)
    if len(df_t) == 0:
        return None
    X = df_t.select(feature_cols).to_numpy()
    pred = model.predict(X)
    chosen_idx = int(np.argmax(pred))
    return (df_t["cand_pos"][chosen_idx], df_t["cand_len"][chosen_idx])


def decode_full_tokens(lf2_bytes):
    """Decode LF2 to get full leaf token list."""
    w = struct.unpack_from("<H", lf2_bytes, 12)[0]
    h = struct.unpack_from("<H", lf2_bytes, 14)[0]
    cc = lf2_bytes[0x16]
    payload_start = 0x18 + cc * 3

    ring = bytearray([0x20] * 0x1000)
    ring_pos = 0x0fee
    data_pos = payload_start
    flag = 0
    flag_count = 0
    total = w * h
    produced = 0
    tokens = []
    while produced < total:
        if flag_count == 0:
            flag = lf2_bytes[data_pos] ^ 0xff
            data_pos += 1
            flag_count = 8
        if (flag & 0x80) != 0:
            pixel = lf2_bytes[data_pos] ^ 0xff
            data_pos += 1
            tokens.append(("L", pixel, 1))
            ring[ring_pos] = pixel
            ring_pos = (ring_pos + 1) & 0x0fff
            produced += 1
        else:
            upper = lf2_bytes[data_pos] ^ 0xff
            lower = lf2_bytes[data_pos+1] ^ 0xff
            data_pos += 2
            length = (upper & 0x0f) + 3
            position = ((upper >> 4) | (lower << 4)) & 0x0fff
            tokens.append(("M", position, length))
            copy_pos = position
            for _ in range(length):
                if produced >= total:
                    break
                pixel = ring[copy_pos]
                ring[ring_pos] = pixel
                ring_pos = (ring_pos + 1) & 0x0fff
                copy_pos = (copy_pos + 1) & 0x0fff
                produced += 1
        flag <<= 1
        flag_count -= 1
    return tokens, payload_start


def frame_payload(tokens):
    """Token list → LF2 payload bytes."""
    out = bytearray()
    i = 0
    while i < len(tokens):
        flag_pos = len(out)
        out.append(0)
        flag_byte = 0
        bits_used = 0
        while bits_used < 8 and i < len(tokens):
            kind, a, b = tokens[i]
            if kind == "L":
                flag_byte |= 1 << (7 - bits_used)
                out.append(a ^ 0xff)
            else:
                # Match (pos=a, len=b)
                p = a & 0x0fff
                l_enc = (b - 3) & 0x0f
                upper = (l_enc | ((p & 0x0f) << 4)) & 0xff
                lower = ((p >> 4) & 0xff) & 0xff
                out.append(upper ^ 0xff)
                out.append(lower ^ 0xff)
            bits_used += 1
            i += 1
        out[flag_pos] = flag_byte ^ 0xff
    return bytes(out)


def main():
    print("Loading LF2 ...")
    lf2_bytes = LF2_PATH.read_bytes()
    print("Decoding leaf tokens ...")
    leaf_tokens, payload_start = decode_full_tokens(lf2_bytes)
    print(f"leaf tokens: {len(leaf_tokens)}, payload starts at offset {payload_start}")

    print("Loading CSV ...")
    df = load_csv()
    multi_pos_token_idx_set = set(df["token_idx"].unique().to_list())
    print(f"multi_pos tokens (in CSV): {len(multi_pos_token_idx_set)}")

    print("Training model ...")
    model, feature_cols = train_model(df)

    print("Predicting per-token ...")
    # Group predictions by token_idx
    feat_cols_list = feature_cols
    X = df.select(feat_cols_list).to_numpy()
    pred = model.predict(X)
    df_with_pred = df.with_columns(pl.Series("pred", pred))
    # For each token_idx, pick row with max pred
    chosen_rows = (
        df_with_pred.group_by("token_idx", maintain_order=True)
        .agg(pl.all().sort_by("pred", descending=True).first())
    )
    chosen_dict = {
        row["token_idx"]: (row["cand_pos"], row["cand_len"])
        for row in chosen_rows.iter_rows(named=True)
    }
    print(f"chosen tokens from model: {len(chosen_dict)}")

    print("Reconstructing full token list ...")
    reconstructed = []
    n_diffs = 0
    for idx, (kind, a, b) in enumerate(leaf_tokens):
        if idx in chosen_dict:
            # Use model's choice for multi-pos token
            mp_pos, mp_len = chosen_dict[idx]
            # Verify match
            if kind == "M" and (a == mp_pos and b == mp_len):
                pass  # match
            else:
                n_diffs += 1
                if n_diffs <= 5:
                    print(f"  [diff at token {idx}] leaf=({kind},{a},{b}) model=(M,{mp_pos},{mp_len})")
            reconstructed.append(("M", int(mp_pos), int(mp_len)))
        else:
            # Forced token (literal or unique match)
            reconstructed.append((kind, a, b))
    print(f"diffs between model and leaf: {n_diffs}")

    print("Framing model tokens to LF2 payload ...")
    new_payload = frame_payload(reconstructed)
    orig_payload = lf2_bytes[payload_start:]
    print(f"orig payload len: {len(orig_payload)}, new payload len: {len(new_payload)}")
    if new_payload == orig_payload:
        print()
        print("🎯🎯🎯 BINARY MATCH! per-file ML overfit successfully reproduces C0101.LF2 byte-exact.")
    else:
        print()
        print("⚠️  payload differs. First diff offset:")
        for i, (a, b) in enumerate(zip(new_payload, orig_payload)):
            if a != b:
                print(f"  byte {i}: orig=0x{b:02x} new=0x{a:02x}")
                break


if __name__ == "__main__":
    main()
