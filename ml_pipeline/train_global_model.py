"""Train a SINGLE global LightGBM on all files in v8 CSV, evaluate per-file top-1 accuracy.

If per-file accuracy is high (~95%+), the model has learned a generalizable rule.
"""
import lightgbm as lgb
import numpy as np
import polars as pl
from collections import defaultdict

CSV = "/tmp/lf2_v8_all.csv"

print("Loading...")
df = pl.read_csv(CSV)
print(f"rows: {len(df)}, cols: {len(df.columns)}, files: {df['file'].n_unique()}")

# Convert L/M strings
str_cols = [c for c in df.columns if df[c].dtype == pl.Utf8 and c != "file"]
for c in str_cols:
    df = df.with_columns(
        pl.when(pl.col(c) == "L").then(0).when(pl.col(c) == "M").then(1).otherwise(-1).alias(c)
    )

LABEL = "is_leaf"
NON_FEATURE = {"file", "token_idx", "is_leaf"}
features = [c for c in df.columns if c not in NON_FEATURE]
print(f"features: {len(features)}")

# 80/20 train/test split by FILE (so we test generalization across files)
all_files = sorted(df["file"].unique().to_list())
np.random.seed(42)
np.random.shuffle(all_files)
n_train = int(0.8 * len(all_files))
train_files = set(all_files[:n_train])
test_files = set(all_files[n_train:])
print(f"train files: {len(train_files)}, test files: {len(test_files)}")

train_df = df.filter(pl.col("file").is_in(train_files))
test_df = df.filter(pl.col("file").is_in(test_files))

def make_dataset(d):
    X = d.select(features).to_numpy()
    y = d[LABEL].to_numpy()
    groups = d.group_by(["file", "token_idx"], maintain_order=True).agg(pl.len().alias("g"))["g"].to_list()
    return X, y, groups

Xtr, ytr, gtr = make_dataset(train_df)
Xte, yte, gte = make_dataset(test_df)
print(f"train rows: {len(Xtr)}, test rows: {len(Xte)}")

# Train
ds_tr = lgb.Dataset(Xtr, label=ytr, group=gtr, feature_name=features)
ds_te = lgb.Dataset(Xte, label=yte, group=gte, feature_name=features)

params = {
    "objective": "lambdarank", "metric": "ndcg", "ndcg_at": [1],
    "learning_rate": 0.05, "num_leaves": 127, "min_data_in_leaf": 50,
    "max_depth": -1, "feature_fraction": 0.8, "bagging_fraction": 0.8,
    "verbosity": -1, "deterministic": True, "force_row_wise": True,
}
model = lgb.train(
    params, ds_tr, num_boost_round=500, valid_sets=[ds_tr, ds_te],
    valid_names=["train", "test"], callbacks=[lgb.log_evaluation(50)],
)

# Per-file top-1 accuracy on test set
test_pred = model.predict(Xte)
test_files_list = test_df["file"].to_list()
test_tokens = test_df.group_by(["file", "token_idx"], maintain_order=True).agg(pl.len().alias("g"))
test_tokens_keys = test_tokens.select(["file", "token_idx"]).to_numpy()

per_file_correct = defaultdict(lambda: [0, 0])
offset = 0
for i, g in enumerate(gte):
    cp = test_pred[offset:offset+g]
    cl = yte[offset:offset+g]
    chosen = int(np.argmax(cp))
    fname = test_tokens_keys[i, 0]
    per_file_correct[fname][1] += 1
    if cl[chosen] == 1:
        per_file_correct[fname][0] += 1
    offset += g

print(f"\nPer-file top-1 accuracy on TEST set (held out files):")
print(f"  {'file':22} {'correct':>8} {'total':>6} {'acc%':>7}")
sorted_files = sorted(per_file_correct.items(), key=lambda x: x[1][0]/x[1][1])
for f, (c, t) in sorted_files[:10]:
    print(f"  {f:22} {c:8} {t:6}  {100*c/t:6.2f}%")
print("  ...")
for f, (c, t) in sorted_files[-5:]:
    print(f"  {f:22} {c:8} {t:6}  {100*c/t:6.2f}%")

# Overall
total_c = sum(v[0] for v in per_file_correct.values())
total_t = sum(v[1] for v in per_file_correct.values())
print(f"\nOverall TEST accuracy: {total_c}/{total_t} = {100*total_c/total_t:.2f}%")
files_100 = sum(1 for v in per_file_correct.values() if v[0] == v[1])
print(f"Files with 100% accuracy: {files_100}/{len(per_file_correct)}")

# Feature importances
imp = sorted(zip(features, model.feature_importance("gain")), key=lambda x: -x[1])
print("\nTop 15 features (gain):")
for f, v in imp[:15]:
    print(f"  {f:25}  {v:.0f}")
