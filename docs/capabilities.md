# Capability completion gates

The engine is usable, but complete capability coverage has not been established.
Passing current fixtures must not be presented as complete search or strength parity.
Every remaining row requires implementation, exposed controls where appropriate,
and executed acceptance evidence before it becomes verified.

| Capability | Current coverage | Remaining acceptance work |
| --- | --- | --- |
| Standard/Chess960 rules | Verified legal moves, FEN, special moves, perft | Broader differential position corpus |
| Position identity and game history | Verified incremental keys and draw boundaries | Retain history parity during new search probes |
| Three neural models | Exact raw-score fixtures and incremental updates verified | Match phase-selection and adjusted evaluation policies |
| Aggressive evaluation | Simplified material/compensation policy | Persistent sacrifice detection, queen/material adjustments, contextual fixtures |
| Score correction | Not implemented | Pawn, non-pawn, and continuation correction histories |
| Move ordering | Quiet history, killers, capture ordering | Capture and continuation histories, staged losing captures, exchange evaluation |
| Search pruning | PVS, quiescence, basic null/RFP/futility/LMP/LMR | Razoring, probcut, internal reductions, history/SEE pruning, full LMR policy |
| Search extensions | Not implemented | Singular verification, multi-cut, double/triple and negative extensions |
| Iteration policy | Resumable aspiration windows and full-window fallback implemented | Compiled native/WASM verification and search-strength testing |
| Native time management | Checked clock budgets | Best-move node share, instability, and score-drop scaling |
| Human-style selection | Reproducible five-candidate tolerance | Accumulated mistake budget, opening variety, and sacrifice preference |
| Skill controls | Verified nominal levels 1–21 and direct Elo targets | Controlled playing-strength calibration |
| Tunable parameters | 15 applied parameters and five behavior switches implemented | Remaining algorithms/parameters and compiled endpoint verification |
| Native multi-worker search | Independent-request native batch parallelism implemented | Single-position SMP, shared-cache policy, cancellation and runtime evidence |
| Endgame tables | Not implemented | Actual WDL/root probes, rights/clock boundaries, paths and backend limits |
| Benchmarks | Utility perft and WASM timing samples | Reproducible full-engine position suite and native benchmark command |
| Data generation | Not implemented | Seeded self-play, opening input, bounded recording, adjudication, and shutdown |
| Training conversion | Not implemented | Checked packed-record decoding/encoding and corpus comparison |
| Position filters | Not implemented | Material, danger, space, storms, development, shelter, attack, outpost and compensation predicates |
| WASM access | Verified controller, modes, skills and live budgets | Carry new portable capabilities through bindings and both backends |
| GUI | Out of scope | No GUI will be implemented |

Native-only facilities must be declared as such rather than silently advertised in
WASM. Portable algorithms belong below platform adapters; native threads, paths,
and file streaming must not enter the rules/search core. Keep model/data notices
when introducing derived components. Reproduce useful behavior, not undefined
arithmetic, invalid memory access, data races, or uninitialized history reads.
