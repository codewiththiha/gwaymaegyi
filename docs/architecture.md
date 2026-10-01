# Architecture

## Current boundaries

| Crate | Owns | Must not own |
| --- | --- | --- |
| `crates/core` | Domain types, position state, FEN, attacks, legal moves, perft | Clocks, threads, filesystem, platform dispatch |
| `crates/eval` | Immutable model bytes, incremental accumulators | Search, clocks, platform intrinsics |
| `crates/search` | Game orchestration, continuations, cache, controls | OS clocks, I/O, threads, protocol text |
| `crates/engine` | Native UCI actor, clocks, process arguments/I/O | Chess rules or a separate search implementation |
| `crates/wasm` | Browser bindings, owned result views, conversion/errors | A separate board/search implementation, native assumptions |

Within the core, `board/mod.rs` owns storage and maintains mailbox/bitboard agreement.
Its child `fen.rs` parses structural invariants, `play.rs` owns state transitions,
`attacks.rs` owns attack calculation, and `movegen/` owns candidate generation and
king-safety filtering. `castling.rs` handles rook-origin rights and overlapping
Chess960 destinations. Castling moves target the rook internally; adapters choose
the external UCI convention.

The native command library receives arguments and an output writer explicitly.
UCI additionally receives an input stream and owns a bounded-channel actor; only
the binary entry point reads process globals.

Positions are copyable values. Externally supplied moves cannot call the internal
state-transition helper. All state is owned; there are no mutable global registries.
The initial safe, straightforward attack backend is the correctness baseline,
not a claim of optimized engine throughput.

## Extension boundaries

The evaluation crate owns immutable weights and per-worker accumulators. Its model
bytes are borrowed, decoded safely, and never duplicated into a heap-sized weight
table. Search owns history, its typed continuation stack, transposition access,
and target-appropriate validated limits. Native builds expose full resource
ranges; WASM builds retain browser ceilings. See [resource ranges](resources.md).
Evaluation context is part of cache identity. The
rules core must not prefetch or access search state.

Native orchestration owns timing and workers; browser orchestration uses
worker messages and cooperative stop requests. A portable search entry point must
not depend on native threads. Shared tables and worker-local state stay distinct.
Do not add browser threading or architecture-specific intrinsics as unconditional
requirements.

Changes that affect decisions require deterministic tests before performance
optimization. Measure safe alternatives before requesting any unsafe exception.

## Browser scheduling

The worker SDK translates bounded messages into the same portable controller.
MessageChannel tasks schedule slices without a microtask loop or nested-timer
throttling. A job identity guards queued callbacks; successful mutations replace
that identity only after validation, so invalid requests do not cancel valid work.
The SDK copies/freezes no search internals and frees owned binding reports after
copying their primitive data. Compiler SIMD128 is optional, not an unsafe backend
or an unconditional browser requirement.
