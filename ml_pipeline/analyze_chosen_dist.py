"""Analyze the distribution of chosen-candidate distances.

For each tie scene in the subsampled dataset:
- chosen.dist
- min over candidates.dist
- max over candidates.dist
- chosen.dist_rank (1 = closest, n_candidates = farthest)

Save histograms to understand if encoder's choice has predictable distance pattern.

Run:
    uv run python3 analyze_chosen_dist.py /mnt/hdd6tb/lf2_tie_subsample.csv
"""

import argparse
import sys
from pathlib import Path

import polars as pl
import numpy as np


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("csv", type=Path)
    args = parser.parse_args()

    df = pl.read_csv(args.csv)
    print(f"loaded {df.height} rows, {df.select('file').n_unique()} files")

    # Per-scene stats
    df = df.with_columns(
        pl.col("dist").rank("ordinal").over(["file", "tok_idx"]).cast(pl.Int32).alias("dist_rank"),
    )

    chosen = df.filter(pl.col("is_chosen") == 1)
    print(f"chosen rows: {chosen.height}")

    # Distance histogram of chosen candidates
    print("\n=== chosen.dist quantiles ===")
    for q in [0.0, 0.25, 0.5, 0.75, 0.9, 0.95, 0.99, 1.0]:
        v = chosen["dist"].quantile(q)
        print(f"  q={q}: {v}")

    # Chosen.dist_rank distribution (1 = smallest dist among candidates)
    print("\n=== chosen.dist_rank distribution (top 10) ===")
    rank_counts = (
        chosen.group_by("dist_rank")
        .agg(pl.len().alias("c"))
        .sort("c", descending=True)
        .head(10)
    )
    print(rank_counts)

    # Per cand_len: which dist_rank is most often chosen?
    print("\n=== chosen.dist_rank by cand_len ===")
    by_len = (
        chosen.group_by(["cand_len"])
        .agg([
            pl.len().alias("c"),
            pl.col("dist_rank").mean().alias("mean_rank"),
            pl.col("dist_rank").median().alias("median_rank"),
        ])
        .sort("cand_len")
    )
    print(by_len)

    # n_candidates distribution at chosen scenes
    print("\n=== n_candidates quantiles (at chosen scenes) ===")
    for q in [0.0, 0.5, 0.75, 0.9, 0.95, 0.99, 1.0]:
        v = chosen["n_candidates"].quantile(q)
        print(f"  q={q}: {v}")

    # Filter to small n_candidates: rule may be cleaner
    print("\n=== chosen.dist_rank for n_candidates <= 10 ===")
    small = chosen.filter(pl.col("n_candidates") <= 10)
    print(f"rows: {small.height}")
    rank_small = (
        small.group_by("dist_rank")
        .agg(pl.len().alias("c"))
        .sort("dist_rank")
    )
    print(rank_small)

    # Per cand_len for small scenes
    print("\n=== rank by cand_len, small scenes (n_cand <= 10) ===")
    by_len_small = (
        small.group_by("cand_len")
        .agg([
            pl.len().alias("c"),
            pl.col("dist_rank").mean().alias("mean_rank"),
            (pl.col("dist_rank") == 1).sum().alias("rank_1"),
            (pl.col("dist_rank") == pl.col("n_candidates")).sum().alias("rank_max"),
        ])
        .sort("cand_len")
    )
    print(by_len_small)

    return 0


if __name__ == "__main__":
    sys.exit(main())
