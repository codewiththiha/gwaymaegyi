"""Atomic summaries and bounded, redacted plain-text job logs."""
import json
import re
from pathlib import Path

ANSI = re.compile(r"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1b\\))")
CREDENTIAL = re.compile(r"(?:github_pat_[A-Za-z0-9_]+|gh[pousr]_[A-Za-z0-9]+)")
CONTROL = re.compile(r"[\x00-\x08\x0b-\x1f\x7f]")


def redact(value, token=""):
    text = ANSI.sub("", value)
    if token:
        text = text.replace(token, "[redacted]")
    return CONTROL.sub("", CREDENTIAL.sub("[redacted]", text))


class LogStore:
    def __init__(self, output, token="", *, max_file_bytes=2 * 1024 * 1024, max_total_bytes=8 * 1024 * 1024):
        self.output = Path(output)
        self.output.mkdir(parents=True, exist_ok=True)
        self.token = token
        self.max_file_bytes = max_file_bytes
        self.max_total_bytes = max_total_bytes
        self.log_files = {}
        self.unavailable = {}

    def write_job(self, job, raw):
        name = re.sub(r"[^A-Za-z0-9_.-]", "_", job["name"])[:80]
        path = self.output / f"{int(job['id'])}-{name}.txt"
        used = sum(file.stat().st_size for file in self.output.glob("*.txt") if file != path)
        budget = max(0, min(self.max_file_bytes, self.max_total_bytes - used))
        text = redact(raw.decode("utf-8", errors="replace"), self.token).encode("utf-8")
        if not budget:
            self.unavailable[str(job["id"])] = "total log size limit reached"
            return None
        if len(text) > budget:
            marker = b"[log truncated to its final bytes]\n"
            available = max(0, budget - len(marker))
            text = marker[:budget] + (text[-available:] if available else b"")
        path.write_bytes(text.decode("utf-8", errors="ignore").encode("utf-8"))
        self.log_files[str(job["id"])] = path.name
        self.unavailable.pop(str(job["id"]), None)
        return path

    def snapshot(self, run, jobs):
        summary = {
            "run_id": run["id"],
            "attempt": run.get("run_attempt", 1),
            "sha": run["head_sha"],
            "url": run["html_url"],
            "status": run["status"],
            "conclusion": run.get("conclusion"),
            "jobs": [{
                "id": job["id"], "name": redact(job["name"], self.token),
                "status": job["status"], "conclusion": job.get("conclusion"),
                "failed_steps": [redact(step["name"], self.token) for step in job.get("steps", []) if step.get("conclusion") == "failure"],
                "log": self.log_files.get(str(job["id"])),
            } for job in jobs],
            "unavailable_logs": self.unavailable,
        }
        temporary = self.output / "status.json.tmp"
        temporary.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
        temporary.replace(self.output / "status.json")
        return summary


def read_summary(output, tail=25):
    output = Path(output)
    summary = json.loads((output / "status.json").read_text(encoding="utf-8"))
    print(f"Run {summary['run_id']} attempt {summary['attempt']}: {summary['status']} / {summary['conclusion']}")
    print(summary["url"])
    for job in summary["jobs"]:
        print(f"  {job['name']}: {job['status']} / {job['conclusion']}")
        if job.get("conclusion") in ("failure", "timed_out") and job.get("log"):
            path = output / Path(job["log"]).name
            lines = path.read_text(encoding="utf-8").splitlines()
            print("\n".join(lines[-tail:]))
    return summary
