"""Per-file ML encoder loop POC.

Steps:
1. Train LightGBM lambdarank on C0101 v8 dataset (overfit)
2. Decode original C0101 to get leaf token trajectory
3. Re-encode via model: at each step, score all candidates, pick max
4. Compare model's predictions to leaf's choices token-by-token
5. Generate LF2 bytes from model's tokens, verify binary match
"""

import struct
import subprocess
import sys
from pathlib import Path

import lightgbm as lgb
import numpy as np
import polars as pl

LF2_PATH = "/mnt/hdd6tb/lf2_archive/先人のお手本/Windows95版のTo Heart/LVNS3DAT/C0101.LF2"
CSV_PATH = Path("/tmp/lf2_ml_test/c0101_v8.csv")

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
        "objective": "lambdarank",
        "metric": "ndcg",
        "ndcg_at": [1],
        "learning_rate": 0.1,
        "num_leaves": 4096,
        "min_data_in_leaf": 1,
        "max_depth": -1,
        "verbosity": -1,
    }
    model = lgb.train(params, train_set, num_boost_round=2000,
                      valid_sets=[train_set], valid_names=["train"],
                      callbacks=[lgb.log_evaluation(period=500)])
    return model, feature_cols


def autoregressive_encode_check(df, model, feature_cols):
    """At each token, score all candidates, pick max, verify it matches leaf choice."""
    X = df.select(feature_cols).to_numpy()
    y = df["is_leaf"].to_numpy()
    groups = df.group_by(["file", "token_idx"], maintain_order=True).agg(pl.len().alias("g"))["g"].to_list()
    pred = model.predict(X)

    correct = 0
    total = 0
    offset = 0
    mismatches = []
    for tok_idx, g in enumerate(groups):
        chunk_pred = pred[offset:offset+g]
        chunk_label = y[offset:offset+g]
        chosen = int(np.argmax(chunk_pred))
        leaf_idx = int(np.argmax(chunk_label))
        if chosen == leaf_idx:
            correct += 1
        else:
            if len(mismatches) < 10:
                mismatches.append((tok_idx, chosen, leaf_idx,
                                   chunk_pred[chosen], chunk_pred[leaf_idx]))
        total += 1
        offset += g
    print(f"=== Top-1 match against leaf (autoregressive simulation): {correct}/{total} = {correct/total:.4f} ===")
    if mismatches:
        print("First 10 mismatches:")
        for m in mismatches:
            print(f"  token {m[0]}: model picked idx {m[1]} (score {m[3]:.3f}), leaf picked {m[2]} (score {m[4]:.3f})")
    return correct == total


def main():
    print("Loading CSV ...")
    df = load_csv()
    print(f"rows={len(df)}, columns={len(df.columns)}")
    print("Training model ...")
    model, feature_cols = train_model(df)
    print("Verifying autoregressive encoding ...")
    perfect = autoregressive_encode_check(df, model, feature_cols)
    if perfect:
        print()
        print("🎯 SUCCESS: model can autoregressive-encode C0101 with 100% match.")
        print("Next step: integrate with Rust encoding loop to generate LF2 bytes.")
    else:
        print()
        print("⚠️  Model has mispredictions. Need richer features or more training.")


if __name__ == "__main__":
    main()
