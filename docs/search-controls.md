# Search behavior and integration

Playing style, approximate Elo/skill presets, compute budgets, and search algorithm
controls are separate. The backend has no GUI and requires no HTTP server.

## Applied search controls

SearchTuning is a validated value owned by each Options/Engine instance. There is
no mutable global parameter registry. Parameter::SPECS describes only implemented
controls; unsupported algorithms are not advertised as tunable placeholders.

The applied behavior switches are aspiration, null-move, reverse-futility,
quiet-pruning, late-reductions, razoring, internal-reductions, exchange-pruning,
history-pruning, probcut, and singular-extensions. Numeric parameters cover aspiration depth/window,
null-move depth/reduction, reverse futility, quiet futility/LMP, reduction depth,
and history updates. Invalid names or out-of-range values leave state unchanged.

Rust callers modify an Options::tuning copy and configure it atomically. UCI
advertises each numeric option and `Behavior NAME` checks. Direct WASM exposes
`set_behavior(name, enabled)`, `set_parameter(name, value)`, `tuning_json()`, and
`search_controls_json()` discovery. Worker requests use:

```js
worker.postMessage({id: 'policy-1', type: 'behavior', name: 'null-move', enabled: false});
worker.postMessage({id: 'policy-2', type: 'parameter', name: 'AspStartWindow', value: 30});
```

Policy changes cancel incompatible analysis and its stale callbacks. Start a new
search afterward. Live depth/node/time/scheduling changes remain available through
the performance request and retain compatible running continuations.

Aspiration retry state is explicit and survives work slices. Failed bounds widen
monotonically; partial/retried passes do not replace completed variations. Node
caps and stop/reset remain effective during retries.

## Human-like play

Approximate-strength selections use an accumulated human-style policy with
seeded, reproducible choices:

- Opening diversity: before move 7, a line within 25 cp of the best that still
  leads may replace it when a seeded draw matches its index.
- Mistake budget: from move 7 on, the first line whose deficit is positive and
  below the accumulated budget plus 10 cp may be chosen; the chosen deficit is
  subtracted from the budget and a per-move budget of 120 minus nominal Elo divided by 25
  centipawns is added.
- Sacrifice preference: a leading line that already trades material may replace
  the choice at progressively wider material deficits.

The budget persists inside one `Engine` across searches and resets on new
games, position resets, or configuration commits. Full strength and analysis
always take the best line.

## Native CPU parallelism

`gwaymaegyi::analyze_batch` runs independent AnalysisRequest values using 1–16
scoped native workers. Each request gets fresh owned search resources; output order
matches input order regardless of worker count. A caller-owned AtomicBool provides
cooperative cancellation between slices. Combined hash budgets are bounded.
All supplied positions, moves, roots, and limits are checked before starting work.
This is batch concurrency, not yet single-position SMP or a shared mutable cache.

No Tokio dependency is required for this CPU-only API. An application with async
network I/O can own its own runtime and bridge to these bounded workers. Do not
assume that aborting an async wrapper stops an already running CPU search.

Browser builds retain the portable engine and task-scheduled worker protocol. The
native batch module is not exported for WASM; creating Rust std threads on the
minimal browser target is not a supported fallback. Independent browser jobs can
be hosted in separate Web Workers without changing the portable engine.

## Research rationale

Tokio documents that CPU work needs bounded concurrency, running spawn_blocking
jobs cannot be aborted, and long-lived work is better served by dedicated threads:
[1](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html).

The Rust target documentation specifies that std::thread::spawn panics on
wasm32-unknown-unknown and that OS-dependent std facilities are unavailable:
https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html.

Web Workers provide background execution and message-based communication:
https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Using_web_workers.

Keep measured speed, correctness fixtures, and playing-strength calibration
separate. Compiler SIMD support is not a universal native-parity or speed guarantee.
