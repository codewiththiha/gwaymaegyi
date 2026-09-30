#!/usr/bin/env python3
"""Validate Conventional Commit subjects and subject/body separation."""
import argparse
import json
import os
import re
import subprocess
import sys

SUBJECT = re.compile(r"^(feat|fix|perf|refactor|docs|test|ci|build|chore)(\([a-z0-9][a-z0-9-]*\))?!?: [^\n]+$")


def validate(message):
    lines = message.rstrip("\n").splitlines()
    if not lines:
        return ["empty commit message"]
    subject = lines[0]
    problems = []
    if len(subject) > 72:
        problems.append(f"subject is {len(subject)} characters; maximum is 72")
    if not SUBJECT.fullmatch(subject):
        problems.append("subject must use a supported Conventional Commit type")
    if subject.endswith("."):
        problems.append("subject must not end with a period")
    if len(lines) > 1 and (lines[1] != "" or (len(lines) > 2 and lines[2] == "")):
        problems.append("subject and body must have exactly one blank line between them")
    return problems


def event_revision():
    event_file = os.environ.get("GITHUB_EVENT_PATH")
    if not event_file:
        return "HEAD"
    with open(event_file, encoding="utf-8") as file:
        event = json.load(file)
    if "pull_request" in event:
        pr = event["pull_request"]
        return f"{pr['base']['sha']}..{pr['head']['sha']}"
    before, after = event.get("before"), event.get("after")
    if before and after and before.strip("0"):
        return f"{before}..{after}"
    return "HEAD"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--revision")
    args = parser.parse_args()
    revision = args.revision or event_revision()
    ids = subprocess.check_output(["git", "rev-list", "--no-merges", revision], text=True).splitlines()
    if ".." not in revision:
        ids = ids[:1]
    failed = False
    for commit in ids:
        message = subprocess.check_output(["git", "show", "-s", "--format=%B", commit], text=True)
        problems = validate(message)
        if problems:
            failed = True
            print(f"{commit[:8]}: " + "; ".join(problems), file=sys.stderr)
    if not failed:
        print(f"Validated {len(ids)} commit subject(s)")
    return int(failed)


if __name__ == "__main__":
    raise SystemExit(main())
