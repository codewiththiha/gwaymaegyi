# gwaymaegyi

A modular chess engine project written in Rust 2024, with native and WebAssembly
interfaces sharing one portable core.

## Current scope

The first increment provides typed positions and moves, standard/Chess960 FEN,
legal move generation, perft, a native CLI, and browser/Node WASM bindings.
Search, neural evaluation, UCI sessions, and parallel workers are not implemented
in this increment. See the [roadmap](docs/roadmap.md) for verification gates.

## Native

Install current stable Rust, then:

```sh
cargo run --release -- perft 4
cargo run --release -- moves
cargo run --release -- fen '4k3/8/8/8/8/8/8/4K3 w - - 0 1'
```

With no FEN, `perft` and `moves` use the starting position. `perft 0` returns one.
Externally supplied moves are checked for legality; invalid input is an error.

## WebAssembly

```sh
cargo build --locked -p gwaymaegyi-wasm --target wasm32-unknown-unknown --profile wasm-release
python3 scripts/setup_bindgen.py /tmp/gwaymaegyi-bindgen
/tmp/gwaymaegyi-bindgen/wasm-bindgen target/wasm32-unknown-unknown/wasm-release/gwaymaegyi_wasm.wasm --target web --out-dir pkg/web
```

The prebuilt binding-tool installer supports Linux. Other platforms can use
`cargo install --locked wasm-bindgen-cli --version 0.2.129`.

```js
import init, { legal_moves, perft } from './pkg/web/gwaymaegyi_wasm.js';
await init();
const fen = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';
console.log(legal_moves(fen, false));
console.log(perft(fen, 3)); // decimal string: '8902'
```

Exports: `normalize_fen`, `legal_moves`, `play_uci`, `perft`. WASM perft accepts
depths zero through six. Use a Web Worker for expensive requests to avoid blocking
the UI. There are no thread, clock, or filesystem requirements in the rules core.

## Development

- [Architecture and boundaries](docs/architecture.md)
- [Development, checks, and CI logs](docs/development.md)
- [Roadmap](docs/roadmap.md)
- [Shared contributor/agent guide](agents.md)

MIT licensed. See [LICENSE](LICENSE).
