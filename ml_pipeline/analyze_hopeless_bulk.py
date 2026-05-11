#!/usr/bin/env python3
"""Analyze hopeless_bulk_stats.tsv to look for patterns.

Goals:
1. Overall leaf_rank distribution among tied set
2. Per-file leaf_rank mode/mean — does each file have a consistent rank?
3. tail_eq_fill / tail_eq_next correlation with leaf choice
4. Group files by tail-byte behavior

Usage:
    uv run python3 ml_pipeline/analyze_hopeless_bulk.py /mnt/hdd6tb/hopeless_bulk_stats.tsv
"""

import sys
import csv
from collections import Counter, defaultdict
from statistics import mean, median


def main(path):
    rows = []
    kinds = Counter()
    with open(path, "r") as f:
        reader = csv.DictReader(f, delimiter="\t")
        for r in reader:
            kinds[r["kind"]] += 1
            if r["kind"] == "tied":
                rows.append({
                    "file": r["file"],
                    "token_idx": int(r["token_idx"]),
                    "len": int(r["len"]),
                    "leaf_pos": int(r["leaf_pos"], 16),
                    "leaf_dist": int(r["leaf_dist"], 16),
                    "leaf_rank": int(r["leaf_rank"]),
                    "n_tied": int(r["n_tied"]),
                    "leaf_tail": int(r["leaf_tail"], 16),
                    "tail_min": int(r["tail_min"], 16),
                    "tail_max": int(r["tail_max"], 16),
                    "actual_next": int(r["actual_next"], 16),
                    "tail_eq_fill": int(r["tail_eq_fill"]),
                    "tail_eq_next": int(r["tail_eq_next"]),
                })

    print(f"Kinds: {dict(kinds)}")
    print(f"Total tied rows: {len(rows)}")
    if not rows:
        return

    # 1. Overall rank distribution
    ranks = [r["leaf_rank"] for r in rows]
    rank_min = sum(1 for r in rows if r["leaf_rank"] == 1)
    rank_max = sum(1 for r in rows if r["leaf_rank"] == r["n_tied"])
    print(f"\nOverall rank distribution:")
    print(f"  rank=1 (min dist): {rank_min} ({100*rank_min/len(rows):.2f}%)")
    print(f"  rank=n_tied (max dist): {rank_max} ({100*rank_max/len(rows):.2f}%)")
    print(f"  rank mean / median: {mean(ranks):.2f} / {median(ranks):.1f}")

    # rank-relative position: leaf_rank / n_tied
    relpos = [r["leaf_rank"] / r["n_tied"] for r in rows]
    print(f"  rel_rank (leaf_rank/n_tied) mean: {mean(relpos):.3f}")

    # 2. Tail byte stats
    tail_fill = sum(1 for r in rows if r["tail_eq_fill"])
    tail_next = sum(1 for r in rows if r["tail_eq_next"])
    print(f"\nTail byte:")
    print(f"  leaf_tail == 0x20 (fill): {tail_fill} ({100*tail_fill/len(rows):.2f}%)")
    print(f"  leaf_tail == actual_next: {tail_next} ({100*tail_next/len(rows):.2f}%)")
    # leaf_tail == tail_min ?
    tail_is_min = sum(1 for r in rows if r["leaf_tail"] == r["tail_min"])
    tail_is_max = sum(1 for r in rows if r["leaf_tail"] == r["tail_max"])
    print(f"  leaf_tail == tail_min: {tail_is_min} ({100*tail_is_min/len(rows):.2f}%)")
    print(f"  leaf_tail == tail_max: {tail_is_max} ({100*tail_is_max/len(rows):.2f}%)")

    # 3. Per-file behavior
    per_file = defaultdict(list)
    for r in rows:
        per_file[r["file"]].append(r)
    print(f"\nPer-file (top 20 by hopeless count):")
    print(f"  {'file':22} count  mean_rank  rank=1%  rank=max%  tail==min%  tail==max%  tail==fill%")
    file_summaries = []
    for f, fr in per_file.items():
        n = len(fr)
        mr = mean(r["leaf_rank"] for r in fr)
        r1 = sum(1 for r in fr if r["leaf_rank"] == 1) / n * 100
        rmx = sum(1 for r in fr if r["leaf_rank"] == r["n_tied"]) / n * 100
        tmn = sum(1 for r in fr if r["leaf_tail"] == r["tail_min"]) / n * 100
        tmx = sum(1 for r in fr if r["leaf_tail"] == r["tail_max"]) / n * 100
        tfi = sum(1 for r in fr if r["tail_eq_fill"]) / n * 100
        file_summaries.append((n, f, mr, r1, rmx, tmn, tmx, tfi))
    file_summaries.sort(reverse=True)
    for n, f, mr, r1, rmx, tmn, tmx, tfi in file_summaries[:20]:
        print(f"  {f:22}  {n:5}  {mr:8.2f}  {r1:6.1f}%  {rmx:8.1f}%  {tmn:9.1f}%  {tmx:9.1f}%  {tfi:10.1f}%")

    # 4. Distribution histogram of n_tied
    n_tied_dist = Counter(r["n_tied"] for r in rows)
    print(f"\nn_tied distribution (top 15):")
    for n, c in sorted(n_tied_dist.items())[:15]:
        print(f"  n_tied={n:3}: {c} rows ({100*c/len(rows):.2f}%)")

    # 5. Among 2-tied (binary tie), how often is rank=1 vs rank=2?
    binary = [r for r in rows if r["n_tied"] == 2]
    if binary:
        r1b = sum(1 for r in binary if r["leaf_rank"] == 1)
        r2b = sum(1 for r in binary if r["leaf_rank"] == 2)
        print(f"\n2-tied: rank=1 (min-dist): {r1b} ({100*r1b/len(binary):.2f}%) | rank=2 (max-dist): {r2b}")

    # 6. Files where rank=1 is dominant (>= 90%)
    print(f"\nFiles where >=90% of hopeless have rank=1 (min-dist):")
    for n, f, mr, r1, rmx, tmn, tmx, tfi in file_summaries:
        if r1 >= 90 and n >= 5:
            print(f"  {f}: n={n} rank=1 rate={r1:.1f}%")
    print(f"\nFiles where >=90% have rank=max (max-dist):")
    for n, f, mr, r1, rmx, tmn, tmx, tfi in file_summaries:
        if rmx >= 90 and n >= 5:
            print(f"  {f}: n={n} rank=max rate={rmx:.1f}%")

    # 7. By file prefix (group analysis)
    by_prefix = defaultdict(list)
    for r in rows:
        # Take "C", "CWEEK", "CLNO", "CSNO", "H", "V" prefix
        f = r["file"]
        if f.startswith("CWEEK") or f.startswith("CSNO") or f.startswith("CLNO"):
            p = f.split("_")[0]
        else:
            p = f[0] if f else "?"
        by_prefix[p].append(r)
    print(f"\nBy file group prefix:")
    print(f"  {'prefix':10}  count  mean_rank  rank=1%  rank=max%  tail==min%")
    for p, prows in sorted(by_prefix.items()):
        n = len(prows)
        mr = mean(r["leaf_rank"] for r in prows)
        r1 = sum(1 for r in prows if r["leaf_rank"] == 1) / n * 100
        rmx = sum(1 for r in prows if r["leaf_rank"] == r["n_tied"]) / n * 100
        tmn = sum(1 for r in prows if r["leaf_tail"] == r["tail_min"]) / n * 100
        print(f"  {p:10}  {n:5}  {mr:8.2f}  {r1:6.1f}%  {rmx:8.1f}%  {tmn:9.1f}%")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(f"usage: {sys.argv[0]} <hopeless_bulk_stats.tsv>", file=sys.stderr)
        sys.exit(2)
    main(sys.argv[1])
