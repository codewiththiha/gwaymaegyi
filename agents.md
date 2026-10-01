# Repository guide

Start with [README.md](README.md), [architecture](docs/architecture.md), and
[roadmap](docs/roadmap.md). Inspect the latest commits and relevant tests before editing.

## Design and code

- Rust edition 2024, current stable toolchain. Default rustfmt; no custom config.
- Keep chess rules and position state platform-independent. Native and WASM
  adapters use the same core. Do not add OS APIs or threads to the core.
- Start code files with a brief responsibility summary; use `//!` module docs
  in Rust and the language's normal convention elsewhere.
- Split modules by ownership and responsibility. Prefer small concrete types;
  introduce abstractions only for a current requirement.
- Keep fields private when they enforce invariants. Use typed domain values,
  explicit errors, and deterministic algorithms.
- Unsafe code is forbidden. If measured requirements cannot be met safely,
  document the safe alternatives tried before proposing a narrowly scoped exception.
- Do not silence warning groups. Any individual lint exception needs a reason.
- Comments explain invariants or non-obvious decisions, not the code's syntax.
  Keep them brief and plain; no decorative headings, diagrams, or repeated narration.
- Do not hide errors, add speculative fallbacks, or emit hot-loop logs.
- Test behaviour at the owning module. Keep public API and WASM tests in sync.
- Preserve applicable third-party notices when introducing dependencies or assets.

## Checks

Run default formatting, Clippy with warnings as errors, native tests, and WASM
checks for affected changes. Commands and the runtime test procedure are in
[development](docs/development.md). Do not claim an unexecuted check passed.
Update the roadmap when a verification gate actually closes.

## Commits

Every authored commit uses Conventional Commits:

`type(scope): short imperative description`

The scope is optional and lowercase. Types: `feat`, `fix`, `perf`, `refactor`,
`docs`, `test`, `ci`, `build`, `chore`. Use a specific repository scope such as
`core`, `movegen`, `fen`, `wasm`, `cli`, or `tooling`.

- Subject: one line, lowercase description unless a proper noun requires otherwise,
  no trailing period; **72 characters maximum including the prefix and spaces**.
  Prefer 50 or fewer. Never truncate meaning to fit.
- Describe one coherent change, not a phase, progress report, test list, or essay.
- Optional body: exactly one blank line after the subject, lines near 72 characters.
  Explain why, constraints, and checks performed. No AI attribution.
- For issue fixes, end with `Fixes #123`. For performance or memory changes,
  include measured before/after results; do not invent measurements.
- Use `!` or `BREAKING CHANGE:` only for intentional interface breaks.

Before committing: inspect the staged diff, choose the primary change and type,
count the complete subject with `len(subject)` in Python, and verify the body.
After committing: inspect `git log -1 --format=%B`, then run
`python3 scripts/check_commit.py --revision HEAD` to check the actual commit.
