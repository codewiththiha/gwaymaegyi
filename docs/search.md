# Search and playing controls

The shared `gwaymaegyi-search` crate has no threads, clocks, file access, global
mutable state, or platform intrinsics. Each `Engine` owns its game and resources.
A host starts a search and calls `step(work)` until its status changes. A work
unit advances one explicit node transition. The next slice resumes the exact
continuation. A slice is a work bound, not a millisecond guarantee.

```rust
use gwaymaegyi_search::{Engine, Mode, SearchLimits};

let mut engine = Engine::new()?;
let mut options = engine.options();
options.set_mode(Mode::Balanced);
options.set_elo(1800)?;
engine.configure(options)?;
engine.start(SearchLimits { depth: 8, nodes: 100_000 })?;
while engine.searching() {
    let report = engine.step(256)?;
    // The host can inspect or stop the search between slices.
}
```

Configuration/position changes discard stale search results. Invalid inputs are
transactional and leave the existing game/continuation intact. `stop()` retains
completed analysis and a legal fallback, including when called before depth one.
Automatic outcomes are terminal; optional draw claims do not end a root search.
The game API rejects moves after automatic outcomes, while Board transitions
remain available for position-only analysis. Terminal games have no fallback move. Quiescence searches every legal evasion
when checked; it does not arbitrarily stop after a small number of moves.

Search includes iterative deepening, alpha-beta/PVS, full check evasions,
quiescence, mate-distance bounds, late-move reductions with re-search, conservative
null-move/reverse-futility pruning, quiet history/killers, and a bounded hash table.
Hash identity includes halfmove, reversible-history, and root evaluation context
so cached results
do not cross incompatible draw histories or root-relative style policies. Models
refresh when material changes the neural evaluation phase. Synthetic null paths do not claim game
repetition or populate the real-position cache. MultiPV searches distinct root
moves and publishes only completed sets of variations.

## Modes

- **balanced**: neutral neural middle/endgame evaluation.
- **aggressive**: aggressive middle-game model, material scaling, and a bounded
  compensation preference when sacrificing material with a favorable score.
- **human-like**: balanced model with compensation preferences and, at limited
  strength, extra candidate error tolerance. Decisions are reproducible by seed.
- **analysis**: best-score selection, optional MultiPV, and no strength cap even
  when an approximate target is configured.

## Strength is not calibrated

`set_elo(0)` selects full strength. Values **500–3000** select approximate
resource/error-tolerance presets, **not measured Elo ratings**. They bound depth,
nodes, and candidate loss tolerance. Elo cannot be claimed until controlled
match testing against rated opponents establishes a reproducible calibration.
The modes are style policies, not guarantees of a human opponent's behavior.

Limits: depth 1–64, positive node count, MultiPV 1–5, hash 1–64 MiB, and slices
1–65536 work units. Browser workers should use small slices such as 128 or 256
and yield a task between them; synchronous calls still block their calling thread.
Time allocation, protocol syntax, and asynchronous scheduling belong to adapters.
