"""C0205 (collision file) で hyperparameter sweep して最良設定を探す。

検証する組合せ:
- num_rounds: 1500, 5000, 10000
- num_leaves: 4096, 8192
- learning_rate: 0.1, 0.05, 0.02
- features: v8 (53), v11 (84)

各組合せで最終 n_diffs を測定。
"""

import struct
import subprocess
import time
from pathlib import Path

import lightgbm as lgb
import numpy as np
import polars as pl

LF2_DIR = Path("/mnt/hdd6tb/lf2_archive/先人のお手本/Windows95版のTo Heart/LVNS3DAT")
WORK_DIR = Path("/tmp/lf2_ml_tune")
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


def get_diffs(model, df, feature_cols):
    X = df.select(feature_cols).to_numpy()
    y = df["is_leaf"].to_numpy()
    groups = df.group_by(["file", "token_idx"], maintain_order=True).agg(pl.len().alias("g"))["g"].to_list()
    pred = model.predict(X)
    n_correct, n_total = 0, len(groups)
    offset = 0
    for g in groups:
        chunk_pred = pred[offset:offset+g]
        chunk_label = y[offset:offset+g]
        chosen_idx = int(np.argmax(chunk_pred))
        if chunk_label[chosen_idx] == 1:
            n_correct += 1
        offset += g
    return n_total - n_correct, n_total


def train_eval(csv_path, num_rounds, num_leaves, lr):
    df = pl.read_csv(csv_path)
    df = categorical_to_numeric(df)
    feature_cols = [c for c in df.columns if c not in NON_FEATURE_COLS]
    X = df.select(feature_cols).to_numpy()
    y = df["is_leaf"].to_numpy()
    groups = df.group_by(["file", "token_idx"], maintain_order=True).agg(pl.len().alias("g"))["g"].to_list()

    train_set = lgb.Dataset(X, label=y, group=groups)
    params = {
        "objective": "lambdarank", "metric": "ndcg", "ndcg_at": [1],
        "learning_rate": lr, "num_leaves": num_leaves, "min_data_in_leaf": 1,
        "max_depth": -1, "verbosity": -1, "deterministic": True, "seed": 42,
    }
    model = lgb.train(params, train_set, num_boost_round=num_rounds)
    n_diffs, n_total = get_diffs(model, df, feature_cols)
    return n_diffs, n_total


def main():
    lf2_path = LF2_DIR / "C0205.LF2"
    csv_v8 = WORK_DIR / "C0205_v8.csv"
    csv_v11 = WORK_DIR / "C0205_v11.csv"

    if not csv_v8.exists():
        subprocess.run(["/home/ariori/repos/2025/retro-decode/target/release/lf2_pairwise_dataset_v8",
                       str(lf2_path), str(csv_v8)], check=True)
    if not csv_v11.exists():
        subprocess.run(["/home/ariori/repos/2025/retro-decode/target/release/lf2_pairwise_dataset_v11",
                       str(lf2_path), str(csv_v11)], check=True)

    print(f"{'features':<10}{'rounds':<10}{'leaves':<10}{'lr':<10}{'diffs/total':<20}{'time':<10}")
    print("-" * 80)
    for features, csv_path in [("v8", csv_v8), ("v11", csv_v11)]:
        for num_rounds in [1500, 5000, 10000]:
            for num_leaves in [4096, 8192]:
                for lr in [0.1, 0.05, 0.02]:
                    t = time.time()
                    n_diffs, n_total = train_eval(csv_path, num_rounds, num_leaves, lr)
                    elapsed = time.time() - t
                    print(f"{features:<10}{num_rounds:<10}{num_leaves:<10}{lr:<10}{n_diffs}/{n_total:<14}{elapsed:.1f}s")


if __name__ == "__main__":
    main()
