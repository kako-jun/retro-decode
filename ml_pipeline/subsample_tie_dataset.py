"""Subsample LF2 tie dataset for ML training (streaming, low-memory).

The full dataset has ~1B rows because some scenes have 2000+ tie candidates.
For each tie scene we keep the chosen candidate + K random non-chosen ones.

Reads CSV row-by-row. Assumes rows are grouped by (file, tok_idx) which is
true because the Rust extractor emits per-file and per-token in order.

Run:
    uv run python3 subsample_tie_dataset.py \
        /mnt/hdd6tb/lf2_tie_dataset.csv \
        /mnt/hdd6tb/lf2_tie_subsample.csv \
        --k 8
"""

import argparse
import csv
import random
import sys
from pathlib import Path


def emit_scene(rows, k, rng, writer):
    """Emit chosen rows + up to k random non-chosen rows from this scene."""
    if not rows:
        return
    chosen = [r for r in rows if r[7] == "1"]
    non_chosen = [r for r in rows if r[7] == "0"]
    if k < len(non_chosen):
        non_chosen = rng.sample(non_chosen, k)
    for r in chosen:
        writer.writerow(r)
    for r in non_chosen:
        writer.writerow(r)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("input_csv", type=Path)
    parser.add_argument("output_csv", type=Path)
    parser.add_argument("--k", type=int, default=8)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--progress-every", type=int, default=10_000_000)
    args = parser.parse_args()

    rng = random.Random(args.seed)

    rows_in = 0
    rows_out = 0
    scenes = 0
    cur_key = None
    buf = []

    with args.input_csv.open("r", newline="") as fin, args.output_csv.open(
        "w", newline=""
    ) as fout:
        reader = csv.reader(fin)
        writer = csv.writer(fout)
        header = next(reader)
        writer.writerow(header)

        for row in reader:
            rows_in += 1
            # row[0]=file, row[1]=tok_idx
            key = (row[0], row[1])
            if key != cur_key:
                if buf:
                    before = rows_out
                    emit_scene(buf, args.k, rng, writer)
                    rows_out += sum(
                        1 for r in buf if r[7] == "1"
                    ) + min(
                        args.k, sum(1 for r in buf if r[7] == "0")
                    )
                    scenes += 1
                cur_key = key
                buf = [row]
            else:
                buf.append(row)

            if rows_in % args.progress_every == 0:
                print(
                    f"[{rows_in:_} read] scenes={scenes:_} written={rows_out:_}",
                    file=sys.stderr,
                )

        if buf:
            emit_scene(buf, args.k, rng, writer)
            rows_out += sum(1 for r in buf if r[7] == "1") + min(
                args.k, sum(1 for r in buf if r[7] == "0")
            )
            scenes += 1

    print(
        f"done: read={rows_in:_} scenes={scenes:_} written={rows_out:_}",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
