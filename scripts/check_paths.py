#!/usr/bin/env python3
"""Reject tracked aliases on case-insensitive filesystems."""
import subprocess


def find_collisions(paths):
    seen = {}
    collisions = set()
    for path in paths:
        parts = path.split("/")
        for end in range(1, len(parts) + 1):
            prefix = "/".join(parts[:end])
            previous = seen.setdefault(prefix.casefold(), prefix)
            if previous != prefix:
                collisions.add((previous, prefix))
    return sorted(collisions)


def main():
    paths = subprocess.check_output(["git", "ls-files", "-z"]).decode("utf-8").split("\0")
    paths = [path for path in paths if path]
    collisions = find_collisions(paths)
    for previous, path in collisions:
        print(f"error: case-only path aliases: {previous!r} and {path!r}")
    if not collisions:
        print(f"Validated {len(paths)} tracked paths")
    return int(bool(collisions))


if __name__ == "__main__":
    raise SystemExit(main())
