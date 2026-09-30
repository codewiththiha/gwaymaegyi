import contextlib
import io
import json
import re
import sys
import tempfile
import unittest
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from ci_watch.api import ApiError, SafeRedirect
from ci_watch.cli import options
from ci_watch.logs import LogStore, context_lines, redact
from ci_watch.monitor import Monitor

SHA = "a" * 40


def run_data(status="completed", conclusion="success", attempt=1):
    return {"id": 12, "head_sha": SHA, "status": status, "conclusion": conclusion,
            "html_url": "https://github.com/owner/repo/actions/runs/12", "run_attempt": attempt}


class FakeApi:
    def __init__(self, status="completed", conclusion="success"):
        self.data = run_data(status, conclusion)
        self.downloads = 0

    def find_run(self, **kwargs):
        return self.data

    def run(self, run_id):
        return self.data

    def jobs(self, run):
        return [{"id": 42, "name": "native/rules", "status": self.data["status"],
                 "conclusion": self.data["conclusion"], "steps": []}]

    def job_logs(self, job_id):
        self.downloads += 1
        return b"completed job output\n"


class WatcherTests(unittest.TestCase):
    def test_redirect_never_forwards_token_to_storage(self):
        request = urllib.request.Request("https://api.github.com/repos/o/r/actions/jobs/1/logs",
                                         headers={"Authorization": "Bearer secret"})
        redirect = SafeRedirect().redirect_request(request, None, 302, "", {}, "https://storage.example/log")
        self.assertIsNone(redirect.get_header("Authorization"))
        same_host = SafeRedirect().redirect_request(request, None, 302, "", {}, "https://api.github.com/new")
        self.assertEqual(same_host.get_header("Authorization"), "Bearer secret")
        with self.assertRaises(ApiError):
            SafeRedirect().redirect_request(request, None, 302, "", {}, "http://storage.example/log")

    def test_secret_and_terminal_controls_are_removed(self):
        self.assertEqual(redact("\x1b[31msecret\x1b[0m ghp_abcdef github_pat_abcdef", "secret"),
                         "[redacted] [redacted] [redacted]")

    def test_storage_has_hard_limits_and_safe_names(self):
        with tempfile.TemporaryDirectory() as directory:
            store = LogStore(directory, max_file_bytes=80, max_total_bytes=100)
            path = store.write_job({"id": 1, "name": "../../test"}, b"x" * 1000)
            self.assertEqual(path.parent, Path(directory))
            self.assertLessEqual(path.stat().st_size, 80)
            second = store.write_job({"id": 2, "name": "second"}, b"y" * 1000)
            self.assertLessEqual(sum(p.stat().st_size for p in Path(directory).glob("*.txt")), 100)
            self.assertIsNotNone(second)

    def test_success_and_failure_have_distinct_exit_codes(self):
        for conclusion, expected in [("success", 0), ("failure", 1), ("cancelled", 1)]:
            with tempfile.TemporaryDirectory() as directory:
                api = FakeApi(conclusion=conclusion)
                monitor = Monitor(api, LogStore(directory), sha=SHA, report=lambda _: None)
                self.assertEqual(monitor.run(), expected)
                self.assertEqual(api.downloads, 1)
                saved = json.loads((Path(directory) / "status.json").read_text())
                self.assertEqual(saved["conclusion"], conclusion)

    def test_once_does_not_try_to_stream_an_active_job(self):
        with tempfile.TemporaryDirectory() as directory:
            api = FakeApi(status="in_progress", conclusion=None)
            monitor = Monitor(api, LogStore(directory), sha=SHA, once=True, report=lambda _: None)
            self.assertEqual(monitor.run(), 0)
            self.assertEqual(api.downloads, 0)

    def test_timeout_uses_deadline_instead_of_busy_polling(self):
        with tempfile.TemporaryDirectory() as directory:
            now = [0.0]
            sleeps = []
            def sleep(delay):
                sleeps.append(delay)
                now[0] += delay
            monitor = Monitor(FakeApi(status="in_progress", conclusion=None), LogStore(directory),
                              sha=SHA, interval=3, max_interval=9, timeout=10,
                              clock=lambda: now[0], sleep=sleep, report=lambda _: None)
            self.assertEqual(monitor.run(), 124)
            self.assertEqual(now[0], 10)
            self.assertLess(len(sleeps), 5)

    def test_exact_commit_is_required(self):
        with tempfile.TemporaryDirectory() as directory:
            monitor = Monitor(FakeApi(), LogStore(directory), sha="b" * 40, report=lambda _: None)
            with self.assertRaises(ApiError):
                monitor.run()

    def test_context_selection_keeps_diagnostics_not_cleanup(self):
        lines = ["setup", "error[E0000]: broken", "source line", "help: correct this", "post-job cleanup"]
        self.assertEqual(context_lines(lines, re.compile("error"), context=2), lines[:4])
        self.assertEqual(context_lines(lines, re.compile("absent")), [])

    def test_utf8_logs_never_exceed_byte_budget(self):
        with tempfile.TemporaryDirectory() as directory:
            store = LogStore(directory, max_file_bytes=41, max_total_bytes=41)
            path = store.write_job({"id": 3, "name": "unicode"}, ("😊" * 200).encode())
            self.assertLessEqual(path.stat().st_size, 41)
            path.read_text(encoding="utf-8")

    def test_invalid_polling_options_are_rejected(self):
        with self.assertRaises(SystemExit), contextlib.redirect_stderr(io.StringIO()):
            options(["watch", "--interval", "0"])


if __name__ == "__main__":
    unittest.main()
