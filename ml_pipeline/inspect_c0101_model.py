"""Train a per-file LightGBM model on C0101 and dump top feature importances."""
import lightgbm as lgb
import numpy as np
import polars as pl
from pathlib import Path

CSV = "/tmp/lf2_v8_all.csv"
TARGET = "C0104.LF2"

df = pl.scan_csv(CSV).filter(pl.col("file") == TARGET).collect()
print(f"rows: {len(df)}, cols: {len(df.columns)}")
print(f"columns: {df.columns}")

# Convert L/M strings to int
str_cols = [c for c in df.columns if df[c].dtype == pl.Utf8 and c != "file"]
for c in str_cols:
    df = df.with_columns(
        pl.when(pl.col(c) == "L").then(0).when(pl.col(c) == "M").then(1).otherwise(-1).alias(c)
    )

LABEL = "is_leaf"
NON_FEATURE = {"file", "token_idx", "is_leaf"}
features = [c for c in df.columns if c not in NON_FEATURE]
print(f"features: {len(features)}")

X = df.select(features).to_numpy()
y = df[LABEL].to_numpy()
groups = df.group_by(["file", "token_idx"], maintain_order=True).agg(pl.len().alias("g"))["g"].to_list()
print(f"tokens: {len(groups)}, total rows: {len(X)}")

ds = lgb.Dataset(X, label=y, group=groups, feature_name=features)
params = {
    "objective": "lambdarank", "metric": "ndcg", "ndcg_at": [1],
    "learning_rate": 0.1, "num_leaves": 4096, "min_data_in_leaf": 1,
    "max_depth": -1, "verbosity": -1, "deterministic": True, "force_row_wise": True,
}
model = lgb.train(params, ds, num_boost_round=300, valid_sets=[ds], valid_names=["t"], callbacks=[lgb.log_evaluation(100)])

# Eval top-1
pred = model.predict(X)
correct = 0
offset = 0
for g in groups:
    cp = pred[offset:offset+g]
    cl = y[offset:offset+g]
    if cl[int(np.argmax(cp))] == 1: correct += 1
    offset += g
print(f"top-1 acc: {correct}/{len(groups)} = {100*correct/len(groups):.2f}%")

# Feature importances
imp = sorted(zip(features, model.feature_importance("gain")), key=lambda x: -x[1])
print("\nTop 20 features by gain:")
for f, v in imp[:20]:
    print(f"  {f:30}  {v:.0f}")
