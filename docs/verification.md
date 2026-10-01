# Foundation verification

Verified on 2026-10-01 with stable Rust 1.98.1 (edition 2024).
Source revision: `a02658665542370e62f42dbe6a492ac1938aa0a7`.
[CI evidence](https://github.com/codewiththiha/gwaymaegyi/actions/runs/36792320823).
Later documentation/tooling commits do not change these rules fixtures.

## Passed

- Default Rust formatting, commit-message validation, and Python tooling tests.
- Native Clippy (all, pedantic, nursery; warnings are errors), rustdoc, release build.
- 16 native Rust tests covering six standard perft fixtures, Chess960 edge cases,
  FEN, illegal input, promotion, en passant, clocks, and command output.
- Native starting-position perft: depth 3 = 8,902; depth 4 = 197,281.
- WASM target Clippy and optimized build; browser and Node binding generation.
- 15 assertions executed against the compiled WASM module, including malformed
  input, Chess960 castling, fractional depths, overflow input, NaN, and signed zero.
- No handwritten unsafe Rust; workspace code forbids it.

## Not claimed

- Search, evaluation, UCI sessions, skill control, or parallel search are implemented.
- Engine strength or tuned throughput has been measured.
- The declared minimum Rust version has been separately tested.
- Windows/macOS runs are required on every push; they are opt-in extended checks.

Use the roadmap's next gate rather than guessing that missing subsystems already
exist. Keep this record tied to executed checks when the engine evolves.

## Position identity and game history

Verified revision: `9f4231ca214199dd2d6606fec63cc626fd84ca46`.
[CI evidence](https://github.com/codewiththiha/gwaymaegyi/actions/runs/36798276197).
Incremental and recomputed full/pawn/non-pawn keys agree across legal transitions.
Identity distinguishes rook origins, excludes counters, and includes only legal
en passant availability. Tests cover claims, automatic repetition/clock draws,
checkmate precedence, and insufficient-material boundaries. All 21 native tests
and the existing compiled WASM checks pass.

## Neural evaluation

Verified revision: `061bd1236b2b51fa09760703fbacac404d0daa55`.
[CI evidence](https://github.com/codewiththiha/gwaymaegyi/actions/runs/36799470740).
All 342 scalar score fixtures pass, spanning 57 positions, three models, and
both perspectives. Incremental refresh comparisons cover captures, en passant,
promotions, orthodox/overlapping castling, and long deterministic play.
The native suite now has 24 tests. This run verifies the evaluation on native
Rust; compiled WASM evaluation/search checks are added with their adapter.

## Playable engine: native, portable WASM, SIMD128

Verified code revision: `f502f2d67a1ef05872ab6d81a2f0d17361894e78`.
Verified on 2026-10-01 with stable Rust 1.98.1.

- [Native and both WASM backends](https://github.com/codewiththiha/gwaymaegyi/actions/runs/36806278006)
- [Extended Windows/macOS verification](https://github.com/codewiththiha/gwaymaegyi/actions/runs/36806582387)

All jobs succeeded, including formatting, strict native/WASM lints, documentation,
38 native tests, 19 Python tooling tests, actual UCI-process smoke checks, and eight
identical deterministic search snapshots on Linux, Windows, macOS, portable WASM,
and SIMD128 WASM. Both compiled WASM backends match all 342 raw model scores and
pass quantum-size invariance, invalid-number/transactional-input, stop, reset,
terminal-position, and actual-worker stale-suppression/control checks.

Additional regressions cover model changes after captures, root-restricted cache
bounds, optional root draw claims versus a mating move, automatic-outcome move
rejection, and contextual score-cache identity. The scalar/incremental evaluation
checks include castling, promotions, en passant, and long deterministic play.

CI module footprint, including all three models:

| Backend | Raw WASM bytes | Gzip bytes |
| --- | ---: | ---: |
| portable | 4,861,881 | 2,795,387 |
| SIMD128 | 4,861,942 | 2,795,457 |

Packages contain native/browser/Node deliverables and applicable license notices.
The browser bootstrap is packaged; actual worker scheduling is exercised through
Node worker_threads with the compiled WASM, not a mocked search backend. This is
not a claim of a full browser-device matrix, calibrated Elo, or literal error-free
software. The declared language minimum remains distinct from the tested current
stable compiler. Parallel native search and real tablebase probes are not implemented.

## Release optimization measurement

At the verified code revision, a local Node v20.20.2 benchmark on an Intel Xeon
2.60 GHz virtual CPU compared the former size-oriented WASM setting with level 3.
Each sample used a fresh default engine, balanced mode, start position, depth 6,
and 100,000-node cap; three warmups preceded eight timing samples. Both settings
finished the same 5,587 nodes. Engine construction is outside the timed interval.

| Setting | Median elapsed ms | Packaged WASM bytes |
| --- | ---: | ---: |
| opt-level s | 85.0932 | 4,859,971 |
| opt-level 3 | 54.5062 | 4,861,881 |

This sample shows about 36% lower search latency for 1,910 extra module bytes.
It is a single-position virtual-machine measurement, not a universal throughput
claim or strength estimate. SIMD128 versus portable speed has not been measured
in this comparison. Run `node scripts/bench_wasm.cjs PATH_TO_NODE_BINDINGS` to repeat
on deployment hardware; preserve correctness snapshots when changing profiles.

## Contributor headers and live controls

Verified revision: `676c207435cbb8ccc39c96c03e6fb6ca7fae2336`.
[Native and both WASM backends](https://github.com/codewiththiha/gwaymaegyi/actions/runs/36812257064).
[Extended Windows/macOS checks](https://github.com/codewiththiha/gwaymaegyi/actions/runs/36812954619).
All jobs passed on 2026-10-01, including 42 native tests, 22 Python tests, code-header
validation, and the existing raw evaluation/search snapshots.

Rust module docs and language-appropriate summaries now describe code-file
responsibility without decorative comments. Shared validated skill controls expose
all 20 nominal Elo presets plus level 21 for full strength. Both compiled WASM
backends verify every mapping, invalid-level rollback, custom-Elo getter semantics,
requested/effective budgets, and live continuation invariance.

Actual worker tests verify live slice/report/time/node adjustments, invalid update
rollback, full-budget defaults, skill/style changes, stale-target rejection, and
stop/reset handling. Increasing compute caps does not bypass deliberately limited
strength. Policy changes cancel incompatible analysis; compute-only changes keep
valid running stacks. Windows/macOS retain identical search snapshots and UCI
skill-option behavior.

This closes the control exposure gate, not full feature/search-strength parity or
rating calibration. Full compute uses the maximum capabilities implemented in this
single-worker engine; it does not imply native SMP/ISA throughput on every device.

## Search histories, adaptive limits, native batch analysis and Syzygy

Locally verified on 2026-10-01 with stable Rust 1.98.1. Strict workspace Clippy,
all native tests, rustdoc, optimized native build, formatting, Python tooling, JS
syntax, UCI smoke tests and eight deterministic search snapshots passed. The native
suite exercises score-correction training/persistence, legal SEE, root-tablebase
filters, adaptive time formulas, WDL counters, and bounded batch analysis. No rating
or universal speed claim is made.

Both portable and SIMD128 WASM targets passed strict Clippy, optimized builds,
compiled Node runtime and worker checks. Each matched all 342 evaluation scores and
eight native search snapshots; worker tests covered live budgets, stale suppression,
controls, and cancellation. The WASM worker applies the shared adaptive time factors
under the host `timeMs` deadline; deadline checks occur between synchronous
worker slices, so one slice may overrun. Search reports expose lossless `bestMoveNodes`
and `tablebaseHits` strings; WASM has no Syzygy provider, so its hit count is zero.

A separate optional native integration test was run with external Lichess KRvK WDL
and DTZ files (`.rtbw` 208 bytes, `.rtbz` 7,632 bytes). For
`8/8/8/2R5/1K6/8/5k2/8 w - - 0 1`, it returned WDL Win, `c5c4`, DTZ 21, and an
immediate `Completion::Tablebase` report. A native UCI process using the same data
returned `bestmove c5c4`, `score cp 29000`, and `tbhits 1`. The fixture is not
committed or bundled; `GWAYMAEGYI_SYZYGY_PATH` enables the reproducible optional test
documented in [development](development.md). Path startup is a shallow preflight;
failed probes fall back to search and do not increase `tbhits`.

Coverage remains intentionally bounded. Interior WDL calls skip castling positions
and nonzero halfmove clocks because the upstream WDL API omits the rule-50 clock.
Root DTZ uses the clock but skips castling and runs only for full-strength single-PV
searches; limited-strength, human-style and MultiPV searches fall back to search.
Wider table coverage, single-position SMP, controlled strength calibration and
broader tactical/parity tests remain open work. These latest checks ran on Linux;
the earlier Windows/macOS CI records above predate the Syzygy adapter. No tablebase
files are bundled.
