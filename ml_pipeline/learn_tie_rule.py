"""LF2 tie-break decision tree learner.

Reads tie dataset CSV produced by `lf2_tie_dataset` Rust bin and learns a shallow
decision tree (max_depth=3..5) that picks the candidate match position chosen by
the original LF2 encoder.

Run:
    uv run python3 learn_tie_rule.py /tmp/lf2_tie_dataset.csv

Expected CSV columns:
    file, tok_idx, r, cand_pos, cand_len, dist, cand_idx_in_candidates,
    is_chosen, input_byte, next_byte, n_candidates
"""

import argparse
import sys
from pathlib import Path

import polars as pl
from sklearn.tree import DecisionTreeClassifier, export_text
from sklearn.model_selection import GroupKFold
import numpy as np


BASE_FEATS = [
    "cand_pos",
    "cand_len",
    "dist",
    "cand_idx_in_candidates",
    "input_byte",
    "next_byte",
    "n_candidates",
    "r",
]


def add_groupwise_feats(df: pl.DataFrame) -> pl.DataFrame:
    """Within each tie scene (file, tok_idx), add features relative to peer
    candidates: rank by dist, normalized rank, dist - min_dist, etc."""
    return df.with_columns(
        [
            (pl.col("dist") - pl.col("dist").min().over(["file", "tok_idx"])).alias(
                "dist_minus_min"
            ),
            (pl.col("dist").max().over(["file", "tok_idx"]) - pl.col("dist")).alias(
                "max_minus_dist"
            ),
            (pl.col("cand_pos") - pl.col("cand_pos").min().over(["file", "tok_idx"])).alias(
                "pos_minus_min"
            ),
            pl.col("dist")
            .rank("ordinal")
            .over(["file", "tok_idx"])
            .cast(pl.Int32)
            .alias("dist_rank"),
            pl.col("cand_pos")
            .rank("ordinal")
            .over(["file", "tok_idx"])
            .cast(pl.Int32)
            .alias("pos_rank"),
            (pl.col("r") % 0x1000).alias("r_mod_ring"),
            (pl.col("cand_pos") % 0x1000).alias("pos_mod_ring"),
        ]
    )


def evaluate_listwise(
    model: DecisionTreeClassifier,
    df: pl.DataFrame,
    feats: list[str],
) -> dict:
    """Score = for each tie scene, argmax of predict_proba == is_chosen?"""
    X = df.select(feats).to_numpy()
    proba = model.predict_proba(X)[:, 1]
    df2 = df.with_columns(pl.Series(name="score", values=proba))

    grouped = df2.group_by(["file", "tok_idx"], maintain_order=True).agg(
        [
            pl.col("score").arg_max().alias("argmax_idx"),
            pl.col("is_chosen").arg_max().alias("chosen_idx"),
            pl.col("is_chosen").sum().alias("chosen_count"),
        ]
    )
    valid = grouped.filter(pl.col("chosen_count") == 1)
    correct = (valid["argmax_idx"] == valid["chosen_idx"]).sum()
    total = valid.height
    return {
        "scenes": total,
        "correct": correct,
        "accuracy": correct / max(total, 1),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("csv", type=Path)
    parser.add_argument("--max-depth", type=int, default=5)
    parser.add_argument("--min-samples-leaf", type=int, default=20)
    parser.add_argument("--cv", action="store_true", help="run 5-fold group CV")
    args = parser.parse_args()

    if not args.csv.exists():
        print(f"missing csv: {args.csv}", file=sys.stderr)
        return 1

    df = pl.read_csv(args.csv)
    print(f"loaded {df.height} rows, {df.select('file').n_unique()} files")
    print(
        f"tie scenes total: "
        f"{df.group_by(['file', 'tok_idx']).len().height}"
    )

    # Filter to genuine tie scenes (n_candidates >= 2)
    df = df.filter(pl.col("n_candidates") >= 2)
    df = add_groupwise_feats(df)

    feats = BASE_FEATS + [
        "dist_minus_min",
        "max_minus_dist",
        "pos_minus_min",
        "dist_rank",
        "pos_rank",
        "r_mod_ring",
        "pos_mod_ring",
    ]

    # Sanity: each tie scene should have exactly one is_chosen=1
    chosen_per_scene = (
        df.group_by(["file", "tok_idx"])
        .agg(pl.col("is_chosen").sum().alias("c"))["c"]
        .value_counts(sort=True)
    )
    print("is_chosen-per-scene distribution:")
    print(chosen_per_scene)

    X = df.select(feats).to_numpy()
    y = df.select("is_chosen").to_numpy().ravel()
    groups = df.select("file").to_numpy().ravel()

    print(f"\n=== full-data fit, max_depth={args.max_depth} ===")
    clf = DecisionTreeClassifier(
        max_depth=args.max_depth,
        min_samples_leaf=args.min_samples_leaf,
        class_weight="balanced",
        random_state=0,
    )
    clf.fit(X, y)
    res = evaluate_listwise(clf, df, feats)
    print(
        f"listwise scene accuracy: {res['correct']}/{res['scenes']}"
        f" = {res['accuracy']:.4f}"
    )
    print("\nfeature importances:")
    for name, imp in sorted(
        zip(feats, clf.feature_importances_), key=lambda x: -x[1]
    ):
        print(f"  {name:24s} {imp:.4f}")

    print("\n=== learned rule (export_text) ===")
    print(export_text(clf, feature_names=feats, max_depth=args.max_depth))

    if args.cv:
        print("\n=== 5-fold group CV (split by file) ===")
        gkf = GroupKFold(n_splits=5)
        accs = []
        for fold, (tr, te) in enumerate(gkf.split(X, y, groups)):
            sub_clf = DecisionTreeClassifier(
                max_depth=args.max_depth,
                min_samples_leaf=args.min_samples_leaf,
                class_weight="balanced",
                random_state=0,
            )
            sub_clf.fit(X[tr], y[tr])
            test_df = df[te]
            res_te = evaluate_listwise(sub_clf, test_df, feats)
            accs.append(res_te["accuracy"])
            print(
                f"fold {fold}: {res_te['correct']}/{res_te['scenes']}"
                f" = {res_te['accuracy']:.4f}"
            )
        print(f"mean: {np.mean(accs):.4f} +/- {np.std(accs):.4f}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
