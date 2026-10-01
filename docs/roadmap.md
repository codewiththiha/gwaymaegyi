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
| Platform portability | Linux, Windows and macOS lints, tests, protocol and snapshot checks passed for the Syzygy increment | Verified |
| Native parallel analysis | Independent-position batch workers and single-position SMP (`Threads`, `SharedTable`, `analyze_parallel`) are implemented | Verified |
| Endgame probing | Native WDL and rule-50-aware single-PV root DTZ; optional KRvK fixture and UCI path tests | Broader supplied-table corpus and additional edge cases |
| Data generation/tools | Seeded self-play (`datagen`), packed 32-byte record conversion (`convert`), 11 tactical/aggressive filters (`filter`), and 50-position `bench` suite | Verified |
| Native resource ranges | Hash 128 GiB, MultiPV 255, Threads 1,024, depth 127; browser-specific ceilings and expanded-range regressions | Implemented; CI verification pending |
| Strength calibration | Controlled matches against rated opponents, reproducible rating estimates | Future work |
| Measured optimization | Throughput/memory and match comparisons while retaining native/WASM correctness | Ongoing |

Portable and SIMD128 WASM builds use optimized compiler settings and pass the same
behavioral checks. This is not evidence of a measured strength rating or a particular
speed improvement. Keep correctness tests and calibration/performance evidence separate.
