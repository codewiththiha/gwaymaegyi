# Architecture

## Current boundaries

| Crate | Owns | Must not own |
| --- | --- | --- |
| `crates/core` | Domain types, position state, FEN, attacks, legal moves, perft | Clocks, threads, filesystem, platform dispatch |
| `crates/engine` | Process arguments, stdout/stderr, native exit codes | Chess rules |
| `crates/wasm` | Browser-facing conversion and errors | A second board implementation, native assumptions |

Within the core, `board/mod.rs` owns storage and maintains mailbox/bitboard agreement.
Its child `fen.rs` parses structural invariants, `play.rs` owns state transitions,
`attacks.rs` owns attack calculation, and `movegen/` owns candidate generation and
king-safety filtering. `castling.rs` handles rook-origin rights and overlapping
Chess960 destinations. Castling moves target the rook internally; adapters choose
the external UCI convention.

The native command library receives arguments and an output writer explicitly;
only its binary entry point reads process globals.

Positions are copyable values. Externally supplied moves cannot call the internal
state-transition helper. All state is owned; there are no mutable global registries.
The initial safe, straightforward attack backend is the correctness baseline,
not a claim of optimized engine throughput.

## Extension boundaries

Introduce evaluation and search crates only when their implementation is needed.
Evaluation will own immutable weights and per-worker accumulators. Search will own
history, its indexed stack, transposition access, and typed search limits. The
rules core must not prefetch or access search state.

Native orchestration will own timing and workers; browser orchestration will use
worker messages and cooperative stop requests. A portable search entry point must
not depend on native threads. Shared tables and worker-local state stay distinct.
Do not add browser threading or architecture-specific intrinsics as unconditional
requirements.

Changes that affect decisions require deterministic tests before performance
optimization. Measure safe alternatives before requesting any unsafe exception.
