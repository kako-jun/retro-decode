"""Per-file LightGBM overfit experiment for LF2 encoder reverse engineering.

入力: lf2_pairwise_dataset_v8 が生成した CSV (file, token_idx, candidates, is_leaf)
出力: ファイル別 per-token classification 精度。100% 到達なら encoder loop に組込む。
"""

import sys
from pathlib import Path

import lightgbm as lgb
import numpy as np
import polars as pl

CSV_PATH = Path("/tmp/lf2_v8_all.csv")

# label
LABEL = "is_leaf"
# group key (per-token: 1 leaf + N rejects)
GROUP_KEYS = ["file", "token_idx"]
# excluded as features
NON_FEATURE_COLS = {"file", "token_idx", "is_leaf"}


def load_one_file(file_name: str) -> pl.DataFrame:
    """Load only rows for one specific file (memory friendly)."""
    df = pl.scan_csv(CSV_PATH).filter(pl.col("file") == file_name).collect()
    return df


def categorical_to_numeric(df: pl.DataFrame) -> pl.DataFrame:
    """Convert L/M kind strings to integers."""
    str_cols = [c for c in df.columns if df[c].dtype == pl.Utf8 and c != "file"]
    for c in str_cols:
        df = df.with_columns(
            pl.when(pl.col(c) == "L")
            .then(0)
            .when(pl.col(c) == "M")
            .then(1)
            .otherwise(-1)
            .alias(c)
        )
    return df


def train_overfit(file_name: str, n_iter: int = 5000, num_leaves: int = 4096):
    df = load_one_file(file_name)
    print(f"[{file_name}] rows={len(df)}, columns={len(df.columns)}")

    df = categorical_to_numeric(df)

    feature_cols = [c for c in df.columns if c not in NON_FEATURE_COLS]
    print(f"feature cols: {len(feature_cols)}")

    X = df.select(feature_cols).to_numpy()
    y = df[LABEL].to_numpy()
    # group sizes (= candidate count per token)
    groups = df.group_by(GROUP_KEYS, maintain_order=True).agg(pl.len().alias("g"))["g"].to_list()
    n_tokens = len(groups)
    print(f"tokens with candidates: {n_tokens}, total candidate rows: {len(X)}")

    # Use ranking objective: at each token, leaf candidate should rank highest
    train_set = lgb.Dataset(X, label=y, group=groups)

    params = {
        "objective": "lambdarank",
        "metric": "ndcg",
        "ndcg_at": [1],
        "learning_rate": 0.1,
        "num_leaves": num_leaves,
        "min_data_in_leaf": 1,
        "max_depth": -1,
        "feature_fraction": 1.0,
        "bagging_fraction": 1.0,
        "lambda_l1": 0.0,
        "lambda_l2": 0.0,
        "verbosity": -1,
        "deterministic": True,
        "force_row_wise": True,
    }

    model = lgb.train(
        params,
        train_set,
        num_boost_round=n_iter,
        callbacks=[lgb.log_evaluation(period=500)],
        valid_sets=[train_set],
        valid_names=["train"],
    )

    # Evaluate per-token top-1 accuracy
    pred = model.predict(X)
    top1_correct = 0
    offset = 0
    for g in groups:
        chunk_pred = pred[offset : offset + g]
        chunk_label = y[offset : offset + g]
        chosen_idx = int(np.argmax(chunk_pred))
        if chunk_label[chosen_idx] == 1:
            top1_correct += 1
        offset += g

    acc = top1_correct / n_tokens
    print(f"[{file_name}] per-token top-1 accuracy: {top1_correct}/{n_tokens} = {acc:.4f}")
    return model, acc


if __name__ == "__main__":
    target = sys.argv[1] if len(sys.argv) > 1 else "C0101.LF2"
    n_iter = int(sys.argv[2]) if len(sys.argv) > 2 else 3000
    train_overfit(target, n_iter=n_iter)
