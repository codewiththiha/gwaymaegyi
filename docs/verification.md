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
