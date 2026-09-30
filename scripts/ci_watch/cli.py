"""CLI options and optional detached execution of the watcher."""
import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

from .api import ApiError, GitHub
from .logs import LogStore, read_summary, redact
from .monitor import Monitor


def infer_repo():
    remote = subprocess.check_output(["git", "remote", "get-url", "origin"], text=True).strip()
    match = re.search(r"github\.com[:/]([A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+?)(?:\.git)?$", remote)
    if not match:
        raise ValueError("use --repo OWNER/REPO for a GitHub repository")
    return match[1]


def options(argv):
    parser = argparse.ArgumentParser(description="Watch GitHub Actions without downloading build artifacts")
    commands = parser.add_subparsers(dest="command", required=True)
    watch = commands.add_parser("watch", help="follow status and collect completed job logs")
    watch.add_argument("--repo", help="OWNER/REPO; otherwise infer from origin")
    watch.add_argument("--sha", help="exact commit; defaults to HEAD when --run-id is absent")
    watch.add_argument("--branch")
    watch.add_argument("--event", help="filter trigger type, such as push or workflow_dispatch")
    watch.add_argument("--workflow", default="ci.yml")
    watch.add_argument("--run-id", type=int)
    watch.add_argument("--interval", type=float, default=8)
    watch.add_argument("--max-interval", type=float, default=45)
    watch.add_argument("--timeout", type=float, default=2700)
    watch.add_argument("--request-timeout", type=float, default=20)
    watch.add_argument("--output", type=Path, default=Path(".ci-logs"))
    watch.add_argument("--token-env", default="GH_TOKEN", help="environment variable containing the token")
    watch.add_argument("--token-file", type=Path, help="private file; its contents are never printed")
    watch.add_argument("--once", action="store_true", help="take one snapshot, then exit")
    watch.add_argument("--background", action="store_true", help="detach; save PID and monitor output")
    read = commands.add_parser("read", help="inspect a saved summary without network access")
    read.add_argument("output", type=Path)
    read.add_argument("--tail", type=int, default=25)
    read.add_argument("--job", help="select job names containing this text")
    read.add_argument("--grep", help="show matching log lines with diagnostic context")
    args = parser.parse_args(argv)
    if args.command == "watch":
        if min(args.interval, args.max_interval, args.timeout, args.request_timeout) <= 0:
            parser.error("poll intervals and timeouts must be positive")
        if args.max_interval < args.interval:
            parser.error("--max-interval must be at least --interval")
        if args.once and args.background:
            parser.error("--once and --background are mutually exclusive")
    elif args.tail < 1:
        parser.error("--tail must be positive")
    return args


def detach(args, argv):
    args.output.mkdir(parents=True, exist_ok=True)
    executable = Path(__file__).resolve().parents[1] / "watch_ci.py"
    child_args = [sys.executable, "-u", str(executable)] + [value for value in argv if value != "--background"]
    child_args += ["--repo", args.repo]
    if args.sha:
        child_args += ["--sha", args.sha]
    kwargs = {}
    if os.name == "nt":
        kwargs["creationflags"] = subprocess.CREATE_NEW_PROCESS_GROUP | subprocess.DETACHED_PROCESS
    else:
        kwargs["start_new_session"] = True
    with (args.output / "monitor.log").open("a", encoding="utf-8") as output:
        child = subprocess.Popen(child_args, stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT, **kwargs)
    (args.output / "monitor.pid").write_text(str(child.pid) + "\n", encoding="utf-8")
    print(f"Watcher PID {child.pid}; output: {args.output / 'monitor.log'}")
    return 0


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    args = options(argv)
    try:
        if args.command == "read":
            read_summary(args.output, args.tail, job_filter=args.job, grep=args.grep)
            return 0
        args.repo = args.repo or infer_repo()
        if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", args.repo):
            raise ValueError("--repo must have OWNER/REPO form")
        if not args.sha and not args.run_id:
            args.sha = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
        if args.sha and not re.fullmatch(r"[a-fA-F0-9]{40}", args.sha):
            raise ValueError("--sha must be a full 40-character commit hash")
        if args.background:
            return detach(args, argv)
        token = args.token_file.read_text().strip() if args.token_file else os.environ.get(args.token_env, os.environ.get("GITHUB_TOKEN", ""))
        api = GitHub(args.repo, token, request_timeout=args.request_timeout)
        store = LogStore(args.output, token)
        monitor = Monitor(api, store, workflow=args.workflow, sha=args.sha, branch=args.branch, event=args.event,
                          run_id=args.run_id, interval=args.interval, max_interval=args.max_interval,
                          timeout=args.timeout, once=args.once, report=lambda text: print(redact(text, token), flush=True))
        return monitor.run()
    except KeyboardInterrupt:
        print("Watcher interrupted; the GitHub run was not cancelled", file=sys.stderr)
        return 130
    except (ApiError, OSError, ValueError, re.error, subprocess.CalledProcessError) as error:
        print(f"watcher: {error}", file=sys.stderr)
        return 2
