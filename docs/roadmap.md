# Roadmap

Close each verification gate before claiming completion. Do not create empty
placeholder crates for future work. Evidence belongs in [verification](verification.md).

| Increment | Acceptance gate | State |
| --- | --- | --- |
| Portable rules/interfaces | Standard perft, Chess960 boundaries, invalid inputs, compiled WASM | Verified |
| Position identity/game state | Incremental/recomputed keys, claims, automatic outcomes, history rollback | Verified |
| Evaluation | Exact scalar scores, incremental refresh agreement, native/WASM fixtures | Verified |
| Single-worker search | Deterministic score/move/node snapshots, terminals, bounded limits | Verified |
| Native/browser control | Real UCI process tests, actual worker stop/reset, stale suppression, transactional inputs | Verified |
| Playing controls | Validated full/approximate strength, modes, reproducible decisions | Implemented and verified; 21-level controls, ratings uncalibrated |
| Platform portability | Windows/macOS lints, tests, protocol and snapshot checks | Verified |
| Parallel native search | Race-free ownership/shutdown and repeatable match testing | Future work |
| Endgame probing | Real backend fixtures, option tests, legal root selection | Future work |
| Data generation/tools | Reproducible seeds, round-trip datasets, bounded resources | Future work |
| Strength calibration | Controlled matches against rated opponents, reproducible rating estimates | Future work |
| Measured optimization | Throughput/memory and match comparisons while retaining native/WASM correctness | Ongoing |

Portable and SIMD128 WASM builds use optimized compiler settings and pass the same
behavioral checks. This is not evidence of a measured strength rating or a particular
speed improvement. Keep correctness tests and calibration/performance evidence separate.
