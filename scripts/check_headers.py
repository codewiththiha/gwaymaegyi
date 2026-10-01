#!/usr/bin/env python3
"""Require concise language-appropriate responsibility headers in tracked code."""
import ast
import subprocess
from pathlib import Path

CODE = {".rs", ".py", ".mjs", ".cjs"}


def has_header(path, content):
    suffix = Path(path).suffix
    if suffix == ".py":
        return ast.get_docstring(ast.parse(content)) is not None
    if suffix == ".rs":
        return content.startswith("//!")
    if suffix in {".mjs", ".cjs"}:
        return content.startswith(("//", "/**"))
    return True


def main():
    paths = subprocess.check_output(["git", "ls-files", "-z"], text=True).split("\0")
    code = [path for path in paths if Path(path).suffix in CODE]
    missing = [path for path in code if not has_header(path, Path(path).read_text(encoding="utf-8"))]
    if missing:
        raise SystemExit("Missing code responsibility headers: " + ", ".join(missing))
    print(f"Validated responsibility headers in {len(code)} code files")


if __name__ == "__main__":
    main()
