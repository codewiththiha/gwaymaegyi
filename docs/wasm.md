# WASM and JavaScript

CI publishes two independently tested packages: **portable** (no SIMD128
requirement) and **simd128** (stable compiler auto-vectorization enabled).
Both use optimized release settings, immutable embedded model bytes, safe
little-endian decoding, and the same portable search engine. Neither requires
shared memory, browser threads, nightly Rust, or platform intrinsics.

Each package has browser modules, Node bindings, generated TypeScript declarations,
worker scripts, and license notices. Host browser files normally as static assets.
The engine is not an HTTP server. `loader.mjs` can feature-detect SIMD128 and select
between supplied portable/SIMD worker URLs.

## Preferred worker interface

```js
const worker = new Worker('/engine/engine-worker.mjs', {type: 'module'});
worker.onmessage = ({data}) => {
  if (data.type === 'progress' || data.type === 'done') console.log(data.report);
};
worker.postMessage({id: 'settings', type: 'configure', options: {
  mode: 'human-like', elo: 1800, hashMiB: 8, seed: '19'
}});
worker.postMessage({id: 'analysis-1', type: 'start', depth: 8, nodes: '100000'});
worker.postMessage({id: 'stop-1', type: 'stop', target: 'analysis-1'});
```

The bootstrap queues at most 128 requests while loading and emits `ready` with
capabilities. Requests need unique string/safe-integer ids. Responses use `ack`,
`progress`, `done`, or `error`. Startup failures emit `init-error`.

| Request type | Fields and behavior |
| --- | --- |
| `configure` | Partial `options`: mode, elo, hashMiB, multiPv, chess960, seed |
| `position` | FEN and optional UCI move list; validates atomically |
| `play` | One legal UCI `move` in the current game |
| `start` | Optional depth, decimal nodes, roots, quantum, timeMs |
| `stop` | Optional target search id; stale targets cannot stop a newer job |
| `reset` | Start position; preserve configuration and discard stale analysis |
| `status` | Game/configuration and latest report |
| `dispose` | Free the engine and close scheduling resources |

MessageChannel tasks avoid nested-timer throttling while allowing stop/reset
messages between slices. Promise/microtask loops are not used for search scheduling.
Old callbacks cannot publish after a successful replacement. Invalid inputs keep
the existing game and active continuation. Progress is throttled; the last complete
set of variations remains available if a later iteration is interrupted.

Default slice: 256 work units; use smaller values for slower devices. A work bound
is not a fixed millisecond guarantee. Optional host time limits stop between slices.
Node counts and seeds are decimal strings to avoid JavaScript precision loss.

## Direct API

```js
import init, {Engine, capabilities_json} from './gwaymaegyi_wasm.js';
await init();
const engine = new Engine();
engine.configure('balanced', 0, 8, 1, false, '19');
engine.start(8, '100000').free();
const report = engine.step(256);
console.log(report.best_move, report.nodes, report.score_cp);
report.free();
engine.free();
```

Direct calls are synchronous. A direct caller must yield tasks between slices or
use the packaged worker. Free owned reports after copying their data; the worker
adapter does this automatically. Engine state is worker-owned; weights are immutable.
WASM linear memory can reuse freed allocations but does not shrink automatically;
terminate a worker when reclaiming its peak memory is necessary.

`capabilities_json()` identifies backend, limits, and **uncalibrated** Elo controls.
`raw_evaluate(fen, model, perspective)` exposes raw model units for exact verification;
engine scores use normalized centipawns, with a separate mate-distance getter.
The original normalize_fen/legal_moves/play_uci/perft helpers remain available.
These position-only helpers differ from the engine game API, which enforces
automatic outcomes and owns repetition history.
