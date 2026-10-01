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

Options: Hash (1–64 MiB), MultiPV (1–5), Mode, UCI_Chess960, UCI_LimitStrength,
UCI_Elo (500–3000), Seed (unsigned 64-bit decimal), Move Overhead (0–5000 ms),
and Ponder. Turning limit strength off selects full strength. Numeric Elo
values are **uncalibrated presets**, not measured engine ratings. Analysis mode
ignores the configured strength cap.

`go` supports depth, nodes, mate, clocks/increments, movetime, movestogo,
searchmoves, infinite, and ponder. Clock arithmetic saturates safely; absent
clocks create no artificial deadline. Extremely large budgets are capped at
24 hours. Infinite/ponder searches withhold `bestmove` until stop/ponderhit.
Invalid commands/positions/options emit a protocol-safe diagnostic and leave
valid game/search state intact. Position/option updates cancel stale analysis.

Use Chess960 rook-origin notation with `UCI_Chess960=true`. Terminal games
return `bestmove 0000`; stopping early returns completed analysis or a legal
root fallback. EOF cancels active work and returns that fallback before exit.
Input lines are capped at 16 KiB. These safeguards are not a literal guarantee
that software or its host cannot fail.
