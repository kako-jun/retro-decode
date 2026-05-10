"""Take first N tie scenes from each file in the dataset.

Streaming, low memory. Useful for getting a diverse sample for ML.

Run:
    uv run python3 sample_per_file.py \
        /mnt/hdd6tb/lf2_tie_dataset.csv \
        /mnt/hdd6tb/lf2_tie_diverse.csv \
        --scenes-per-file 200 \
        --k 8
"""

import argparse
import csv
import random
import sys
from pathlib import Path


def emit_scene(rows, k, rng, writer):
    if not rows:
        return 0
    chosen = [r for r in rows if r[7] == "1"]
    non_chosen = [r for r in rows if r[7] == "0"]
    if k < len(non_chosen):
        non_chosen = rng.sample(non_chosen, k)
    for r in chosen:
        writer.writerow(r)
    for r in non_chosen:
        writer.writerow(r)
    return len(chosen) + len(non_chosen)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("input_csv", type=Path)
    parser.add_argument("output_csv", type=Path)
    parser.add_argument("--k", type=int, default=8)
    parser.add_argument("--scenes-per-file", type=int, default=200)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--progress-every", type=int, default=10_000_000)
    args = parser.parse_args()

    rng = random.Random(args.seed)

    rows_in = 0
    rows_out = 0
    cur_file = None
    cur_key = None
    file_scene_count = 0
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
            f = row[0]
            key = (row[0], row[1])

            # Skip until we exhaust per-file budget
            if f == cur_file and file_scene_count >= args.scenes_per_file:
                # Drop until file changes
                if rows_in % args.progress_every == 0:
                    print(
                        f"[{rows_in:_} read] file={cur_file} sceneN={file_scene_count} written={rows_out:_}",
                        file=sys.stderr,
                    )
                continue

            if f != cur_file:
                if buf:
                    rows_out += emit_scene(buf, args.k, rng, writer)
                    file_scene_count += 1
                cur_file = f
                cur_key = key
                file_scene_count = 0
                buf = [row]
            elif key != cur_key:
                if buf:
                    rows_out += emit_scene(buf, args.k, rng, writer)
                    file_scene_count += 1
                cur_key = key
                buf = [row]
                if file_scene_count >= args.scenes_per_file:
                    buf = []
                    continue
            else:
                buf.append(row)

            if rows_in % args.progress_every == 0:
                print(
                    f"[{rows_in:_} read] file={cur_file} sceneN={file_scene_count} written={rows_out:_}",
                    file=sys.stderr,
                )

        if buf and file_scene_count < args.scenes_per_file:
            rows_out += emit_scene(buf, args.k, rng, writer)

    print(f"done: read={rows_in:_} written={rows_out:_}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
