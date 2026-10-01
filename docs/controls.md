# Playing strength and compute controls

Rust, UCI, direct WASM bindings, and the browser worker expose the same playing
policies. Compute budgets are independent from deliberate strength reduction.

## Playing policies

Canonical names: `balanced`, `aggressive`, `human-like`, `analysis`.
`set_mode` and configuration changes take effect when processed; no model reload or
rebuild is required. Analysis selects the best score and ignores approximate-strength
caps. Full strength does not imply a measured Elo rating or feature/strength parity
with another implementation.

## Skill presets

Skill levels 1–20 select the following nominal Elo controls. Level 21 selects full
strength. Direct Elo settings also accept 500–3000, or zero for full strength.
These labels are uncalibrated for this engine; the preset table is not evidence of
achieved match ratings. Calibration remains a separate acceptance gate.

| Level | Nominal Elo | Level | Nominal Elo |
| --- | ---: | --- | ---: |
| 1 | 500 | 11 | 1900 |
| 2 | 800 | 12 | 2000 |
| 3 | 1000 | 13 | 2100 |
| 4 | 1200 | 14 | 2200 |
| 5 | 1300 | 15 | 2300 |
| 6 | 1400 | 16 | 2400 |
| 7 | 1500 | 17 | 2500 |
| 8 | 1600 | 18 | 2650 |
| 9 | 1700 | 19 | 2800 |
| 10 | 1800 | 20 | 3000 |

A direct non-preset target such as 1350 has no exact skill-level getter value.
The worker rejects configurations containing both `elo` and `skillLevel` rather
than silently choosing one. The most recent valid UCI strength option determines
the active setting; `Skill_Level=21` or disabling UCI_LimitStrength restores full strength.

## API access

| Control | Rust | Native UCI | Direct WASM | Worker |
| --- | --- | --- | --- | --- |
| Playing mode | Options::set_mode | Mode | set_mode/configure | configure.options.mode |
| Nominal Elo | Options::set_elo | UCI_Elo + UCI_LimitStrength | set_elo/configure | configure.options.elo |
| Skill preset | SkillLevel + Options::set_skill_level | Skill_Level | set_skill_level/configure_skill | configure.options.skillLevel |
| Hash memory | Options::set_hash_mib | Hash | set_hash_mib/configure | configure.options.hashMiB |
| MultiPV | Options::set_multi_pv | MultiPV | set_multi_pv/configure | configure.options.multiPv |
| Search caps | SearchLimits, Engine::set_limits | go depth/nodes | start/set_limits/start_full | start/performance |
| Slice work | Engine::step | Adapter-owned | step | quantum |
| Search algorithm tuning | Options::tuning / SearchTuning | Behavior + 38 numeric options | set_behavior / set_parameter | behavior / parameter requests |
| Adaptive time factors | SearchTuning | NodeTmFactor*, BmFactor1, ScoreDrop* | set_parameter / soft_time_limit_ms | set_parameter, applied by worker |
| Time allowance | Host-owned | go clocks/movetime | Host-owned | timeMs deadline, checked between slices |
| Reporting overhead | Host-owned | Adapter-owned | Host-owned | reportIntervalMs |

`capabilities_json()` exposes modes, all 21 skill presets, supported limits,
backend identity, live-limit support, and the false Elo-calibration flag.
`search_controls_json()` lists the eleven behavior switches and 38 validated numeric
parameters. `limits_json()` reports requested/effective depth-node caps. Approximate
strength may lower effective caps; a full compute profile does not override the
selected strength policy.

## Native resource ranges

Native Rust and UCI expose Hash through 131,072 MiB, MultiPV through 255, Threads
through 1,024, and depth 127. Native default search budgets request full depth/nodes;
there is no 256 MiB combined parallel hash ceiling. Defaults allocate only 32 MiB
hash and use one PV/thread. Maximum ranges require suitable host memory and CPU
resources; see [resource ranges](resources.md). Browser limits do not apply to native.

## Full and adjustable WASM compute

The default worker compute profile is `full`: depth 64, maximum unsigned 64-bit
node budget, 1024 work units per slice, and a 100 ms reporting interval. It uses
all currently implemented single-worker search capability until a limit or stop.
It is not a native multi-thread/ISA performance guarantee. Portable and optional
SIMD128 modules use level-3 optimization and the same safe engine.

`balanced` uses depth 8, 100,000 nodes, slices of 256, and 50 ms reporting.
`responsive` uses depth 6, 50,000 nodes, slices of 64, and 100 ms reporting.
Any profile field can be overridden. Choose a finite time/node budget for games;
full analysis deliberately keeps consuming compute until stopped or completed.

```js
worker.postMessage({id: 'policy', type: 'configure', options: {
  mode: 'aggressive', skillLevel: 21
}});
worker.postMessage({id: 'search', type: 'start', profile: 'full', timeMs: 1000});
worker.postMessage({id: 'tune', type: 'performance', target: 'search', options: {
  quantum: 128, reportIntervalMs: 200, nodes: '500000', timeMs: 500
}});
```

A performance update preserves the live continuation and already counted nodes.
Node caps are absolute for that search, not additional nodes. Raising caps allows
more work; lowering an exhausted cap stops immediately and retains completed
analysis. A new timeMs deadline starts when its update is processed; null removes it. The
worker checks deadlines between bounded synchronous slices, so one slice can
overshoot. Adaptive factors may stop earlier but never extend the deadline.
Finished/stopped searches require a new start request.

## What instant changes mean

Commands are handled between bounded work slices, not in the middle of a Rust
function. No fixed millisecond response is guaranteed on arbitrary devices.
Smaller quantum values improve control latency; larger values reduce scheduling
overhead. Report intervals affect notifications, not search decisions.

Depth/node/time/scheduling changes do not restart valid running work. Playing-mode,
Elo/skill, hash, and MultiPV configuration changes invalidate old analysis because
its policy/resources may no longer be compatible. Those operations cancel the old
job; issue a new start request with the new settings. Invalid input preserves the
current game, configuration, and active search. Old queued callbacks cannot publish
results into the replacement job.
