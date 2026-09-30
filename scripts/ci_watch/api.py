"""Bounded GitHub API requests with credential-safe redirects."""
import json
import time
import urllib.error
import urllib.parse
import urllib.request


class ApiError(RuntimeError):
    def __init__(self, status, message):
        super().__init__(message)
        self.status = status


class SafeRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, msg, headers, newurl):
        if urllib.parse.urlsplit(newurl).scheme != "https":
            raise ApiError(code, "refusing an insecure download redirect")
        redirected = super().redirect_request(request, fp, code, msg, headers, newurl)
        if redirected is not None:
            original = urllib.parse.urlsplit(request.full_url).netloc
            destination = urllib.parse.urlsplit(newurl).netloc
            if original != destination:
                redirected.remove_header("Authorization")
        return redirected


class GitHub:
    def __init__(self, repo, token="", *, request_timeout=20, retries=4, sleep=time.sleep):
        self.repo = repo
        self.token = token
        self.request_timeout = request_timeout
        self.retries = retries
        self.sleep = sleep
        self.opener = urllib.request.build_opener(SafeRedirect())

    def _request(self, path, *, limit=8 * 1024 * 1024):
        headers = {
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
            "User-Agent": "gwaymaegyi-ci-watcher",
        }
        if self.token:
            headers["Authorization"] = "Bearer " + self.token
        url = "https://api.github.com/repos/" + self.repo + path
        for attempt in range(self.retries + 1):
            try:
                request = urllib.request.Request(url, headers=headers)
                with self.opener.open(request, timeout=self.request_timeout) as response:
                    data = response.read(limit + 1)
                    if len(data) > limit:
                        raise ApiError(413, "response exceeds the configured download limit")
                    return data
            except urllib.error.HTTPError as error:
                throttled = error.code == 429 or (
                    error.code == 403 and (
                        error.headers.get("X-RateLimit-Remaining") == "0"
                        or error.headers.get("Retry-After") is not None
                    )
                )
                transient = error.code in (500, 502, 503, 504)
                if attempt == self.retries or not (throttled or transient):
                    raise ApiError(error.code, f"GitHub API returned HTTP {error.code}") from None
                delay = min(2 ** attempt, 30)
                if throttled:
                    try:
                        delay = float(error.headers.get("Retry-After") or 0)
                        if not delay:
                            delay = float(error.headers.get("X-RateLimit-Reset", 0)) - time.time()
                    except ValueError:
                        delay = 30
                    delay = min(max(delay, 1), 120)
                self.sleep(delay)
            except (urllib.error.URLError, TimeoutError, OSError):
                if attempt == self.retries:
                    raise ApiError(0, "GitHub request failed after network retries") from None
                self.sleep(min(2 ** attempt, 30))
        raise ApiError(0, "GitHub request did not finish")

    def json(self, path):
        try:
            return json.loads(self._request(path, limit=2 * 1024 * 1024))
        except (json.JSONDecodeError, UnicodeDecodeError):
            raise ApiError(0, "GitHub returned invalid JSON") from None

    def find_run(self, *, workflow, sha, branch=None):
        query = {"per_page": 30, "head_sha": sha}
        if branch:
            query["branch"] = branch
        runs = self.json(f"/actions/workflows/{urllib.parse.quote(workflow, safe='')}/runs?" + urllib.parse.urlencode(query))
        matches = [run for run in runs.get("workflow_runs", []) if run["head_sha"] == sha]
        return max(matches, key=lambda run: (run.get("run_number", 0), run.get("run_attempt", 1)), default=None)

    def run(self, run_id):
        return self.json(f"/actions/runs/{int(run_id)}")

    def jobs(self, run):
        jobs = []
        page = 1
        attempt = run.get("run_attempt", 1)
        while True:
            data = self.json(f"/actions/runs/{run['id']}/attempts/{attempt}/jobs?per_page=100&page={page}")
            jobs.extend(data.get("jobs", []))
            if len(jobs) >= data.get("total_count", len(jobs)):
                return jobs
            page += 1

    def job_logs(self, job_id):
        return self._request(f"/actions/jobs/{int(job_id)}/logs")
