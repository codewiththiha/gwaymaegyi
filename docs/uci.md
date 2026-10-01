# Native UCI

Run `gwaymaegyi` without arguments, or `gwaymaegyi uci`. The process uses an
input reader and one worker owning the portable engine. Bounded channels and
small work slices allow `isready`, `stop`, and reset requests during analysis.
No search state is shared mutably between workers.

```text
uci
setoption name Mode value human-like
setoption name UCI_Elo value 1800
setoption name UCI_LimitStrength value true
position startpos moves e2e4 e7e5
go wtime 60000 btime 60000 winc 1000 binc 1000
```

Options: Hash (1–64 MiB), Threads (1–16), MultiPV (1–5), Mode, UCI_Chess960,
UCI_LimitStrength, UCI_Elo (500–3000), Skill_Level (1–21), SyzygyPath (native
local tablebase files), Seed (unsigned 64-bit decimal), Move Overhead (0–5000 ms),
and Ponder. Setting `Threads` above 1 attaches a lock-striped `SharedTable` and
spawns helper workers with diversified quiet move ordering for active `go`
searches. Non-UCI diagnostic commands `bench [depth] [positions]` and
`printparams` are also supported interactively. Turning
limit strength off selects full strength. Numeric Elo
values are **uncalibrated presets**, not measured engine ratings. Analysis mode
ignores the configured strength cap.

`go` supports depth, nodes, mate, clocks/increments, movetime, movestogo,
searchmoves, infinite, and ponder. Clock arithmetic saturates safely; absent
clocks create no artificial deadline. Extremely large budgets are capped at
24 hours. Infinite/ponder searches withhold `bestmove` until stop/ponderhit;
ponder clocks start on `ponderhit`. Enabling Ponder adds a legal predicted reply
when the selected PV has one.
Invalid commands/positions/options emit a protocol-safe diagnostic and leave
valid game/search state intact. Position/option updates cancel stale analysis.

Use Chess960 rook-origin notation with `UCI_Chess960=true`. Terminal games
return `bestmove 0000`; stopping early returns completed analysis or a legal
root fallback. EOF cancels active work and returns that fallback before exit.
Input lines are capped at 16 KiB. These safeguards are not a literal guarantee
that software or its host cannot fail.

`SyzygyPath` optionally loads native Syzygy directories; empty disables them.
`.rtbw` files provide interior WDL probes; those calls skip castling positions and
nonzero halfmove clocks because the WDL wrapper does not receive those draw-rule
inputs. With matching `.rtbz` files, full-strength single-PV searches use a
rule-50-aware root DTZ move; castling positions, limited-strength play and MultiPV
fall back to ordinary search. Disable the current path before selecting another.
`info tbhits` counts successful root and interior probes. Startup only checks directory
readability and `.rtbw` presence, not full file integrity or table coverage; probe
failures fall back to search and do not increment the hit count. Table files are not bundled.

Skill_Level 1–20 selects the documented nominal preset table; level 21 restores full
strength. Setting a valid skill level activates its strength selection immediately
when the command is processed. The preset labels remain uncalibrated. The complete
mapping and shared WASM endpoints are in [controls](controls.md).

## Adaptive time management

Clock games use the reference soft-limit policy. The hard cap is 80% of the
usable side time; the initial soft limit is 60% of one move's share plus the
increment. After each completed iteration the soft limit is rescaled by the
best move's node share, its stability across iterations, and a smoothed
score-drop factor (clamped to 0.90-1.18). Search stops at the soft limit or
hard cap, whichever comes first. The worker checks deadlines between bounded
search slices, so a small slice-level overshoot is possible. `movetime` disables
adaptation; `infinite` and `ponder` searches have no clock deadline until stopped or
answered.
