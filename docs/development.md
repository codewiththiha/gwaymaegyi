# Development

## Rust checks

The toolchain file tracks current stable with edition 2024. The language minimum
is declared separately in the workspace; dependency updates may raise it.
Default rustfmt applies, including its default 100-column width.

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-targets --all-features
cargo doc --locked --workspace --no-deps
cargo clippy --locked -p gwaymaegyi-wasm --target wasm32-unknown-unknown --all-targets -- -D warnings
python3 scripts/check_paths.py
python3 scripts/check_headers.py
python3 -m unittest discover -s tests -p 'test_*.py' -v
```

Clippy enables all, pedantic, and nursery groups; unwrap/expect and undocumented
unsafe blocks are denied. Unsafe code is forbidden throughout the workspace.
Compiler compatibility/style warnings and documentation warnings are also checked.
Public error documentation uses plain prose; those modules explicitly expect only
Clippy's Markdown-heading requirement. Binding methods additionally expect only
individual lints that conflict with the ABI (owned arrays and non-const exports).
All warning groups stay enabled.

## WASM runtime

After the build and matching binding-tool install described in the README:

```sh
wasm-bindgen target/wasm32-unknown-unknown/wasm-release/gwaymaegyi_wasm.wasm --target nodejs --out-dir pkg/node
node tests/wasm.cjs pkg/node/gwaymaegyi_wasm.js
node tests/wasm_search.cjs pkg/node/gwaymaegyi_wasm.js portable
node tests/worker.cjs pkg/node/gwaymaegyi_wasm.js
```

Repeat with `RUSTFLAGS='-Dwarnings -Ctarget-feature=+simd128'` and the `simd128`
argument to verify that backend. CI executes both independently. Browser and Node
packages contain the same engine module. Counts use decimal
strings at the JavaScript boundary to preserve full integer precision.

## Native protocol and deterministic snapshots

```sh
cargo build --locked --release -p gwaymaegyi
python3 tests/uci_smoke.py target/release/gwaymaegyi
cargo run --locked -q -p gwaymaegyi-search --example emit_fixtures > /tmp/search.json
python3 scripts/check_search_fixtures.py fixtures/search.json /tmp/search.json
```

Snapshots cover all modes, approximate/full strength, mate, promotion, Chess960,
root filters, and model phase changes. Inspect changed decisions/node counts before
intentionally regenerating `fixtures/search.json`. Do not update fixtures merely to
hide a failed check. The compiled WASM backends compare against these same results.

## Automated checks

The CI workflow runs formatting/tooling, native lint/tests, and the portable/SIMD128
WASM lint/runtime matrix in parallel. Actions are pinned to immutable revisions; dependencies use a checked-in
lockfile. Dependency caches are separate for native and each WASM backend and saved only on main.
Obsolete runs are cancelled. Builds expose small, short-retention artifacts rather
than generated files in Git. A manual `extended` input also tests Windows/macOS protocols and snapshots.
Artifacts include applicable model/license notices. The workflow is callable from another workflow without custom credentials.

## Watching CI

The standard-library Python watcher reads Actions status and completed job logs.
It does not cancel runs, push changes, or download build artifacts.
GitHub exposes completed job logs, not a live stream of an active job's lines.

```sh
export GH_TOKEN=... # optional for public repos; use an environment secret
python3 scripts/watch_ci.py watch --sha "$(git rev-parse HEAD)"
python3 scripts/watch_ci.py watch --run-id 123 --output .ci-logs/run-123 --background
python3 scripts/watch_ci.py read .ci-logs/run-123 --job WASM --grep error
```

Use `--repo OWNER/REPO`, `--workflow`, `--branch`, `--event`, `--token-file`, or `--token-env`
when needed. Use `--event workflow_dispatch` to distinguish manual runs from push
runs for the same commit. The default is the exact HEAD commit, not whichever run
is newest.
Adaptive polling ranges from 8 to 45 seconds. Network retries and API rate limits
are handled; overall timeout defaults to 45 minutes. `--once` takes one snapshot.

The output directory contains atomic `status.json`, redacted plain-text job logs,
and, in detached mode, `monitor.log`/`monitor.pid`. Job logs are capped at 2 MiB
per file and 8 MiB total; separate directories keep separate runs. Auth headers
are removed on cross-host redirects. Never put credentials in URLs or tracked files.

Exit codes: 0 success (or an active one-shot snapshot), 1 unsuccessful run,
2 configuration/API error or run not found in one-shot mode, 124 timeout,
130 interruption. Monitor interruption does not cancel the GitHub run.
The reader highlights compiler/test diagnostics rather than post-job cleanup;
`--tail` is the fallback when no matching diagnostic lines are present.

## Contributor navigation

Every code file starts with a short responsibility summary. Rust uses standard
inner module docs (`//!`); other languages use their normal module/header convention.
`scripts/check_headers.py` verifies that these entry points exist. Keep summaries
brief and explain ownership or offered APIs rather than restating file syntax.
