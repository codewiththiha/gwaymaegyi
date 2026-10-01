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
- **aggressive**: aggressive middle-game model with dynamic phase switching
  (middle-game -> aggressive when score >= -100 cp, aggressive -> balanced when
  score <= -150 cp, endgame when non-pawn material <= 1300), persistent
  game-history sacrifice detection, queen/material multiplier scaling, and
  interior draw contempt (-25 cp for root mover, +25 cp for opponent).
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

SkillLevel validates presets 1–21; level 21 selects full strength. The complete
nominal mapping is in [controls](controls.md). `SearchLimits::full()` requests the
maximum supported caps. `Engine::set_limits` adjusts a running search without
restarting its stack; exhausted lower caps stop it and retain completed results.
Requested and effective limits are available separately.

Native limits: depth 1–127, MultiPV 1–255, hash 1–131,072 MiB, and positive
unsigned 32-bit slice work. Native default search budgets are full depth/nodes.
WASM limits: depth 1–64, MultiPV 1–32, hash 1–64 MiB, and slices 1–65,536.
Both accept positive unsigned 64-bit node budgets. The compiled constants and
allocation details are documented in [resource ranges](resources.md).
Browser workers should use small slices such as 128 or 256 and yield a task
between them; synchronous calls still block their calling thread.
Time allocation, protocol syntax, and asynchronous scheduling belong to adapters.
