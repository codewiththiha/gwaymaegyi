#!/usr/bin/env python3
"""Entry point for the read-only GitHub Actions log watcher."""
from ci_watch.cli import main

if __name__ == "__main__":
    raise SystemExit(main())
