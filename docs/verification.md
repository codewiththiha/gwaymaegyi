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
