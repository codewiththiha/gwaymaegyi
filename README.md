# gwaymaegyi

A modular **Rust 2024 chess engine** with a safe portable search engine, native
UCI, and first-class WebAssembly/JavaScript interfaces. Current stable Rust,
default rustfmt, and strict compiler/Clippy checks apply throughout.

## What is implemented

- Standard chess and Chess960: legal moves, FEN, perft, incremental position keys,
  repetition history, draw claims, automatic outcomes, and checkmate precedence.
- Three immutable quantized neural models with incremental accumulators and exact
  evaluation fixtures. No unsafe casts or heap-sized weight copies.
- Iterative alpha-beta/PVS search, quiescence with all check evasions, draw-aware
  caching, reductions/re-search, quiet history, distinct-root MultiPV, and limits.
- **Resumable search slices**: progress survives yields; stop/reset does not restart
  an interrupted recursive search. Native and browser adapters use the same engine.
- Validated per-engine search behaviors/parameters and resumable aspiration retries.
- Native batch analysis and single-position SMP with configurable shared hash/threads.
- Optional native-only Syzygy WDL probing and single-PV root DTZ move selection;
  `info tbhits` reports successful probes. Portable/WASM builds remain filesystem-
  and native-FFI-free. Users supply `.rtbw` WDL files; matching `.rtbz` files also
  enable root DTZ ranking. No tablebase data is bundled.
- Native UCI with clocks, root restrictions, ponder, validated options, and legal
  early-stop fallbacks. Optimized **portable and SIMD128 WASM packages**, generated
  TypeScript declarations, and a worker SDK with stale-result suppression.

### Strength and playing style

Select **balanced**, **aggressive**, **human-like**, or **analysis**. Full strength
is the default; nominal **500–3000 Elo** settings select resource/error-tolerance
presets. Skill levels 1–20 map to these nominal targets; level 21 selects full
strength. **These targets are not calibrated ratings.** Analysis ignores the strength
cap. Neither a measured playing rating nor literal error-free software is claimed.
See [control endpoints and skill presets](docs/controls.md) for the complete
Rust/UCI/WASM access matrix and live compute controls.

Native Rust/UCI supports **Hash up to 131,072 MiB (128 GiB), MultiPV up to 255,
Threads up to 1,024, and depth 127**, with no separate 256 MiB parallel hash cap.
Defaults remain practical: 32 MiB hash, one PV, one thread, and full strength.
WASM retains browser limits: 64 MiB hash, 32 PVs, and depth 64. See
[resource ranges](docs/resources.md) for defaults, allocation requirements, and
native/browser differences. MultiPV cannot exceed the available legal root moves.

Single-position SMP, self-play `datagen`, 32-byte training-record `convert`, eleven
position `filter` tools, and the 50-position `bench` suite are implemented.
These capabilities and limits are not a claim of measured strength parity with
another engine. Controlled strength calibration remains future work; see the
[roadmap](docs/roadmap.md) and [verification evidence](docs/verification.md).

## Native UCI and command line

```sh
cargo run --locked --release -p gwaymaegyi -- uci
cargo run --locked --release -p gwaymaegyi -- perft 4
cargo run --locked --release -p gwaymaegyi -- moves
```

No arguments also starts UCI. In a UCI session:

```text
uci
setoption name Mode value human-like
setoption name UCI_Elo value 1800
setoption name UCI_LimitStrength value true
position startpos moves e2e4 e7e5
go wtime 60000 btime 60000 winc 1000 binc 1000
```

Use `stop` during analysis. Turn UCI_LimitStrength off for full strength.
[Native protocol details](docs/uci.md) describe clocks, Chess960, EOF, and options.

## Rust API

The platform-independent `gwaymaegyi-search` crate owns configuration, game state,
and search resources. The native `gwaymaegyi` crate also re-exports this API.

```rust
use gwaymaegyi_search::{Engine, Mode, SearchLimits};

let mut engine = Engine::new()?;
let mut options = engine.options();
options.set_mode(Mode::Balanced);
options.set_elo(1800)?; // zero selects full strength
engine.configure(options)?;
engine.start(SearchLimits { depth: 8, nodes: 100_000 })?;
while engine.searching() {
    engine.step(256)?;
    // Inspect, stop, or yield between slices.
}
let report = engine.report();
```

Native applications can attach user-owned Syzygy WDL data without coupling the
portable search crate to filesystem paths:

```rust
let tables = std::sync::Arc::new(gwaymaegyi::NativeTablebases::open("./syzygy")?);
engine.set_tablebase(Some(tables));
```

Interior WDL probes intentionally skip castling positions and nonzero halfmove
clocks because Pyrrhic's WDL call has no rule-50 argument. With matching DTZ files,
root move ranking uses the rule-50-aware DTZ API in full-strength single-PV searches;
castling positions are unsupported. Syzygy paths and table files are native-only.

Invalid inputs leave existing game/search state intact. Successful position or
configuration changes discard stale analysis. Core/evaluation/search do not own
OS clocks, threads, files, or an HTTP server.

## WebAssembly and JavaScript

Download a tested portable/SIMD128 package from a successful GitHub Actions run,
or build it:

```sh
rustup target add wasm32-unknown-unknown
cargo build --locked -p gwaymaegyi-wasm --target wasm32-unknown-unknown --profile wasm-release
python3 scripts/setup_bindgen.py /tmp/gwaymaegyi-bindgen
/tmp/gwaymaegyi-bindgen/wasm-bindgen target/wasm32-unknown-unknown/wasm-release/gwaymaegyi_wasm.wasm --target web --out-dir pkg/web
cp web/*.mjs pkg/web/
cp LICENSE pkg/web/LICENSE
cp assets/models/LICENSE pkg/web/MODEL-LICENSE
```

For SIMD128, build with `RUSTFLAGS='-Ctarget-feature=+simd128'`. The prebuilt binding
installer supports Linux; other platforms can install the matching locked
wasm-bindgen CLI. Host the resulting browser package as static assets.

```js
const worker = new Worker('/engine/engine-worker.mjs', {type: 'module'});
worker.onmessage = ({data}) => console.log(data);
worker.postMessage({id: 'settings', type: 'configure', options: {
  mode: 'human-like', elo: 1800, seed: '19'
}});
worker.postMessage({id: 'analysis-1', type: 'start', depth: 8, nodes: '100000'});
```

The SDK yields tasks between bounded slices and handles stop/reset while preserving
search progress. Its default full compute profile uses the maximum supported budgets;
use time/node caps or balanced/responsive profiles for bounded play. Live performance
updates retain the continuation, while playing-policy changes cancel stale analysis.
Counts and seeds use decimal strings. Direct `Engine` bindings, raw evaluation, and
the original position-only helpers remain available.
[WASM API and packaging](docs/wasm.md) explain ownership, backend selection, and messages.

## Development and licensing

[Search controls and CPU integration](docs/search-controls.md) ·
[Architecture](docs/architecture.md) · [Checks and CI logs](docs/development.md) ·
[Roadmap](docs/roadmap.md) · [Verification](docs/verification.md) ·
[Contributor/agent guide](agents.md)

MIT licensed; see [LICENSE](LICENSE). Bundled neural weights retain their separate
notice in [assets/models/LICENSE](assets/models/LICENSE). Native packages include
applicable Syzygy-probe notices in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md);
preserve them when redistributing.
