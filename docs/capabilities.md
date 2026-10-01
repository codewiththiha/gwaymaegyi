# Capability completion gates

The engine is usable, but complete capability coverage has not been established.
Passing current fixtures must not be presented as complete search or strength parity.
Every remaining row requires implementation, exposed controls where appropriate,
and executed acceptance evidence before it becomes verified.

| Capability | Current coverage | Remaining acceptance work |
| --- | --- | --- |
| Standard/Chess960 rules | Verified legal moves, FEN, special moves, perft | Broader differential position corpus |
| Position identity and game history | Verified incremental keys and draw boundaries | Retain history parity during new search probes |
| Three neural models | Exact raw-score fixtures, incremental updates, and dynamic aggressive/middle/endgame phase switching verified | Broader match and tuning calibration |
| Aggressive evaluation | Persistent game-history sacrifice detection, queen/material multiplier adjustments, and draw contempt verified | Broader match and tuning calibration |
| Score correction | Pawn/non-pawn/continuation corrections applied and trained on completed exact quiet nodes; worker-local tables persist across positions and clear on new game/policy reset | Feature-index/parity calibration and broader search fixture comparison |
| Move ordering | Quiet/capture/continuation history, killers, SEE-aware winning/losing capture stages and exchange evaluation | Strength/parity calibration and broader ordering fixtures |
| Search pruning | PVS, quiescence, null/RFP/futility/LMP/LMR, razoring, probcut, internal reductions, history and SEE pruning | Broader tactical corpus and controlled strength measurement |
| Search extensions | Singular verification with multi-cut and positive/negative extensions | Broader tactical corpus and tuning validation |
| Iteration policy | Resumable aspiration windows and full-window fallback implemented | Compiled native/WASM verification and search-strength testing |
| Time management | UCI and WASM adapt soft limits without extending host deadlines; both check between bounded work slices | Match/time-control calibration |
| Human-style selection | Seeded opening variety, accumulated mistake budget and sacrifice preference | Controlled playing-style validation |
| Skill controls | Verified nominal levels 1–21 and direct Elo targets | Controlled playing-strength calibration |
| Tunable parameters | 38 validated numeric parameters, eleven behavior switches, and `printparams` tuning export verified | Controlled tuning runs |
| Native multi-worker search | Configurable independent-request batch workers and single-position SMP (`Threads`, lock-striped `SharedTable`, `analyze_parallel`) verified | Scaling measurements on high-core hosts |
| Resource controls | Native Hash 128 GiB, MultiPV 255, Threads 1,024, depth 127; separate smaller WASM limits and expanded-range regressions verified | High-resource host measurements |
| Endgame tables | Native Pyrrhic/Fathom WDL interior probes and rule-50-aware root DTZ selection with `.rtbz` data; UCI path, batch sharing and hit counters | Broader supplied-table coverage; interior WDL skips castling/nonzero clocks; root DTZ skips castling; no bundled tables |
| Benchmarks | Reproducible 50-position full-engine `bench` suite across CLI, UCI, and WASM plus perft and WASM timing samples | Ongoing hardware comparisons |
| Data generation | Seeded self-play (`datagen`) with opening normalization, early `MultiPV` randomization, quiet filtering, and adjudication verified | Large-scale training corpus generation |
| Training conversion | Checked 32-byte packed `BulletRecord` decoding/encoding (`convert`) across native CLI and WASM | Large-corpus streaming benchmarks |
| Position filters | All 11 tactical/aggressive predicates (`filter` 1–11: material sacrifice, king safety, piece activity, pawn storms, tactical compensation, and combined pipeline) verified | Corpus distribution studies |
| WASM access | Verified controller, modes, skills, live budgets, behavior/parameter tuning and tablebase-hit reporting (always zero without a provider) | Broader browser/device matrix and further portable capabilities |
| GUI | Out of scope | No GUI will be implemented |

Native-only facilities must be declared as such rather than silently advertised in
WASM. Portable algorithms belong below platform adapters; native threads, paths,
and file streaming must not enter the rules/search core. Keep model/data notices
when introducing derived components. Reproduce useful behavior, not undefined
arithmetic, invalid memory access, data races, or uninitialized history reads.
