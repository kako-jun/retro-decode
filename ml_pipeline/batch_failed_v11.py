"""M22: 失敗ファイルを num_boost_round=5000 + larger num_leaves で再学習。

入力: /tmp/lf2_ml_failed.txt (1ファイル名/行)
出力: /tmp/lf2_ml_batch_v2_results.csv
"""

import csv
import struct
import subprocess
import sys
import time
from pathlib import Path

import lightgbm as lgb
import numpy as np
import polars as pl

LF2_DIR = Path("/mnt/hdd6tb/lf2_archive/先人のお手本/Windows95版のTo Heart/LVNS3DAT")
WORK_DIR = Path("/tmp/lf2_ml_batch")
RESULTS_CSV = Path("/tmp/lf2_ml_batch_v11_results.csv")
RUST_BIN = Path("/home/ariori/repos/2025/retro-decode/target/release/lf2_pairwise_dataset_v11")
FAILED_LIST = Path("/tmp/lf2_ml_failed.txt")

WORK_DIR.mkdir(parents=True, exist_ok=True)

NON_FEATURE_COLS = {"file", "token_idx", "is_leaf"}


def categorical_to_numeric(df):
    str_cols = [c for c in df.columns if df[c].dtype == pl.Utf8 and c != "file"]
    for c in str_cols:
        df = df.with_columns(
            pl.when(pl.col(c) == "L").then(0)
            .when(pl.col(c) == "M").then(1)
            .otherwise(-1).alias(c)
        )
    return df


def decode_full_tokens(lf2_bytes):
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


def process_one(lf2_path, num_rounds, num_leaves):
    name = lf2_path.stem
    csv_path = WORK_DIR / f"{name}_v11.csv"

    if not csv_path.exists():
        result = subprocess.run(
            [str(RUST_BIN), str(lf2_path), str(csv_path)],
            capture_output=True, text=True
        )
        if result.returncode != 0:
            return {"file": name, "status": "csv_failed", "match": 0,
                    "tokens": 0, "multi": 0, "rounds": num_rounds}

    try:
        df = pl.read_csv(csv_path)
    except Exception as e:
        return {"file": name, "status": f"csv_load_failed", "match": 0,
                "tokens": 0, "multi": 0, "rounds": num_rounds}

    df = categorical_to_numeric(df)
    feature_cols = [c for c in df.columns if c not in NON_FEATURE_COLS]
    X = df.select(feature_cols).to_numpy()
    y = df["is_leaf"].to_numpy()
    groups = df.group_by(["file", "token_idx"], maintain_order=True).agg(pl.len().alias("g"))["g"].to_list()

    train_set = lgb.Dataset(X, label=y, group=groups)
    params = {
        "objective": "lambdarank", "metric": "ndcg", "ndcg_at": [1],
        "learning_rate": 0.05, "num_leaves": num_leaves, "min_data_in_leaf": 1,
        "max_depth": -1, "verbosity": -1,
    }
    model = lgb.train(params, train_set, num_boost_round=num_rounds)

    pred = model.predict(X)
    df_with_pred = df.with_columns(pl.Series("pred", pred))
    chosen_rows = (
        df_with_pred.group_by("token_idx", maintain_order=True)
        .agg(pl.all().sort_by("pred", descending=True).first())
    )
    chosen_dict = {
        row["token_idx"]: (row["cand_pos"], row["cand_len"])
        for row in chosen_rows.iter_rows(named=True)
    }

    lf2_bytes = lf2_path.read_bytes()
    leaf_tokens, payload_start = decode_full_tokens(lf2_bytes)
    reconstructed = []
    n_diffs_in_csv = 0
    for idx, (kind, a, b) in enumerate(leaf_tokens):
        if idx in chosen_dict:
            mp_pos, mp_len = chosen_dict[idx]
            if not (kind == "M" and a == mp_pos and b == mp_len):
                n_diffs_in_csv += 1
            reconstructed.append(("M", int(mp_pos), int(mp_len)))
        else:
            reconstructed.append((kind, a, b))
    new_payload = frame_payload(reconstructed)
    orig_payload = lf2_bytes[payload_start:]
    match = 1 if new_payload == orig_payload else 0

    try:
        csv_path.unlink()
    except:
        pass

    return {"file": name, "status": "ok", "match": match,
            "tokens": len(leaf_tokens), "multi": len(chosen_dict),
            "rounds": num_rounds, "n_diffs": n_diffs_in_csv}


def main():
    files = [LF2_DIR / f"{name.strip()}.LF2" for name in FAILED_LIST.read_text().splitlines() if name.strip()]
    print(f"failed files to retry: {len(files)}")
    num_rounds = 3000
    num_leaves = 8192

    done = set()
    if RESULTS_CSV.exists():
        with open(RESULTS_CSV, "r") as f:
            for row in csv.DictReader(f):
                done.add(row["file"])
        print(f"resuming, {len(done)} already done")

    write_header = not RESULTS_CSV.exists()
    with open(RESULTS_CSV, "a") as f:
        writer = csv.DictWriter(f, fieldnames=["file", "status", "match", "tokens", "multi", "rounds", "n_diffs", "elapsed_s"])
        if write_header:
            writer.writeheader()

        for i, p in enumerate(files):
            if p.stem in done:
                continue
            t = time.time()
            try:
                r = process_one(p, num_rounds=num_rounds, num_leaves=num_leaves)
            except Exception as e:
                r = {"file": p.stem, "status": f"exception: {e}", "match": 0,
                     "tokens": 0, "multi": 0, "rounds": num_rounds, "n_diffs": -1}
            r["elapsed_s"] = round(time.time() - t, 1)
            writer.writerow(r)
            f.flush()
            print(f"[{i+1}/{len(files)}] {r['file']}: match={r['match']} diffs={r.get('n_diffs', '?')} ({r['elapsed_s']}s)", flush=True)


if __name__ == "__main__":
    main()
