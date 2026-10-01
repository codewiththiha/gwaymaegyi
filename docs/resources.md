# Native and browser resource ranges

Native Rust/UCI builds expose the full resource ranges below. Browser restrictions
are selected for WASM targets only; they do not weaken native builds.

| Control | Native | WASM |
| --- | --- | --- |
| Hash | 1–131,072 MiB (128 GiB maximum), default 32 MiB | 1–64 MiB, default 8 MiB |
| MultiPV | 1–255, default 1 | 1–32, default 1 |
| Search depth | 1–127 | 1–64 |
| Nodes per search | Positive unsigned 64-bit count | Positive unsigned 64-bit count |
| Work per `step` | 1–4,294,967,295 | 1–65,536 |
| Native UCI/SMP/batch/datagen threads | 1–1,024, default 1 | Native threading is not exported |
| Native UCI clock allowance | Unsigned 64-bit milliseconds; no one-day cap | Worker `timeMs` remains capped at one day |

`MAX_HASH_MIB`, `MAX_MULTI_PV`, `MAX_DEPTH`, and `MAX_WORK` report the compiled
search target's limits. The native adapter additionally exports `MAX_THREADS`.
Hash setters/getters use `u32`; native worker counts use `u16` so the complete
ranges are representable. Invalid requests remain transactional.

Full strength is the default (`Skill_Level=21`, `UCI_LimitStrength=false`). Native
`SearchLimits::default()` equals `SearchLimits::full()`: depth 127 and `u64::MAX`
nodes. Explicit depth/node/time limits and deliberately selected approximate
strength still apply. WASM retains bounded Rust default limits; the browser
worker's default full profile requests depth 64 and `u64::MAX` nodes.

## Maximums are not default allocations

A supported maximum does not mean allocating 128 GiB or launching 1,024 workers
on startup. Choose hash and thread settings that fit the host. Large native hash
requests use checked byte arithmetic and fallible reservation; worker creation
is fallible and already-started helpers are stopped/joined if startup fails.
An operating system can still terminate a process under memory pressure.

Native parallel APIs no longer impose a separate 256 MiB aggregate hash budget.
Batch requests own their hash tables. SMP allocates the requested shared table
and retains worker-local caches/history, so total memory includes more than the
shared hash alone. `Engine::with_options` creates the chosen private cache
without first allocating the default cache. UCI helpers are stopped and joined
when their search/session is replaced, not left detached.

MultiPV returns at most the requested count and the number of legal permitted
root moves. Requesting 255 in the starting position therefore produces 20 lines,
not invented or duplicate moves. Higher MultiPV spends more search work on
alternatives and can reduce reached depth at a fixed time budget. Default 1
remains appropriate for selecting the strongest move within a finite clock.

Native late-move-reduction tables cover depths through 127 and move indices
through 255. Native `bench` and `perft` accept depths through 127 instead of the
previous small CLI ceilings. These diagnostic commands can take a very long
time at high depths and are not asynchronous UCI analysis jobs.

`DatagenConfig` exposes per-worker `hash_mib` and threads through 1,024. Any
positive representable position count is accepted; there is no 100,000-record
ceiling. The current return value owns collected records in memory, so callers
must budget storage for large runs. Cancellation is honored before allocating
workers/output for a validated already-cancelled request.

The WASM bindings and worker validate the smaller browser ranges independently
of native settings. `capabilities_json()` reports their compiled limits. Both
portable and SIMD128 packages use the same limits and safe search implementation.
Resource-range parity is not a measured playing-strength or scaling claim.
