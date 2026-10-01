#!/usr/bin/env python3
"""Compare deterministic search snapshots across builds and platforms."""
import json
import sys
from pathlib import Path


def main(expected_path, actual_path):
    expected = json.loads(Path(expected_path).read_text(encoding="utf-8"))
    actual = json.loads(Path(actual_path).read_text(encoding="utf-8"))
    if expected != actual:
        raise SystemExit("Search snapshots changed; inspect decisions before regenerating fixtures")
    print(f"Matched {len(expected)} native search snapshots")


if __name__ == "__main__":
    main(*sys.argv[1:])
