# Roadmap

Each increment must close its verification gate before its completion is claimed.
Do not turn this list into empty placeholder crates.

| Increment | Acceptance gate |
| --- | --- |
| Portable rules and interfaces | Standard perft suite, Chess960 edge cases, invalid-input tests, native CLI and actual WASM runtime checks |
| Position identity and game state | Incremental/recomputed hashes agree; repetition and draw-rule tests |
| Evaluation | Scalar correctness, full/incremental accumulator agreement, native/WASM evaluation fixtures |
| Single-worker search | Deterministic score/move/node fixtures; terminal positions and bounded limits |
| Protocol and browser search | UCI transcript tests; responsive stop/reset; worker message tests |
| Parallel native search | Race-free ownership and shutdown; repeatable match testing |
| Endgame probing and skill control | Backend fixtures, option tests, legal move selection |
| Data generation and tools | Reproducible seeds, round-trip datasets, bounded resource use |
| Measured optimization | Throughput and strength comparisons; native and WASM correctness retained |

The rules foundation is the current increment. Search and evaluation are not yet
available; generated binaries must not imply otherwise.
