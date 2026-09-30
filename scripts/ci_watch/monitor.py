"""Status polling follows one exact commit and one workflow attempt."""
import time

from .api import ApiError


class Monitor:
    def __init__(self, api, store, *, workflow="ci.yml", sha=None, branch=None, run_id=None,
                 interval=8, max_interval=45, timeout=2700, once=False,
                 sleep=time.sleep, clock=time.monotonic, report=print):
        self.api = api
        self.store = store
        self.workflow = workflow
        self.sha = sha
        self.branch = branch
        self.run_id = run_id
        self.interval = interval
        self.max_interval = max_interval
        self.timeout = timeout
        self.once = once
        self.sleep = sleep
        self.clock = clock
        self.report = report

    def run(self):
        deadline = self.clock() + self.timeout
        delay = self.interval
        previous = None
        run = None
        while self.clock() < deadline:
            if self.run_id:
                run = self.api.run(self.run_id)
            else:
                try:
                    run = self.api.find_run(workflow=self.workflow, sha=self.sha, branch=self.branch)
                except ApiError as error:
                    if error.status != 404:
                        raise
                    run = None
            if run is None:
                if previous != "waiting":
                    self.report(f"Waiting for {self.workflow} on commit {self.sha}")
                    previous = "waiting"
                if self.once:
                    return 2
            else:
                if self.sha and run["head_sha"] != self.sha:
                    raise ApiError(0, "selected run does not match the requested commit")
                self.run_id = run["id"]
                jobs = self.api.jobs(run)
                attempt = run.get("run_attempt", 1)
                signature = (run["status"], run.get("conclusion"), attempt,
                             tuple((job["id"], job["status"], job.get("conclusion")) for job in jobs))
                changed = signature != previous
                if changed:
                    self.report(f"Run {run['id']} attempt {attempt}: {run['status']} / {run.get('conclusion')}")
                    for job in jobs:
                        self.report(f"  {job['name']}: {job['status']} / {job.get('conclusion')}")
                    previous = signature
                    delay = self.interval
                else:
                    delay = min(delay * 1.5, self.max_interval)
                for job in jobs:
                    key = str(job["id"])
                    if job["status"] == "completed" and key not in self.store.log_files:
                        try:
                            path = self.store.write_job(job, self.api.job_logs(job["id"]))
                            if path:
                                self.report(f"Saved {path.name}")
                        except ApiError as error:
                            if error.status not in (404, 409, 410, 413):
                                raise
                            self.store.unavailable[key] = str(error)
                self.store.snapshot(run, jobs)
                if run["status"] == "completed":
                    self.report(run["html_url"])
                    return 0 if run.get("conclusion") == "success" else 1
                if self.once:
                    return 0
            self.sleep(min(delay, max(0, deadline - self.clock())))
        self.report("Watcher timed out; the GitHub run was not cancelled")
        return 124
