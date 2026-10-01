//! Emit deterministic native search snapshots for cross-platform/WASM verification.

use gwaymaegyi_core::START_FEN;
use gwaymaegyi_search::{Completion, Engine, Mode, SearchLimits, SearchStatus};
use std::{
    error::Error,
    io::{self, Write},
};

struct Case {
    fen: &'static str,
    mode: Mode,
    elo: u16,
    chess960: bool,
    roots: &'static [&'static str],
    depth: u8,
}

const CASES: [Case; 8] = [
    Case {
        fen: START_FEN,
        mode: Mode::Balanced,
        elo: 0,
        chess960: false,
        roots: &[],
        depth: 3,
    },
    Case {
        fen: START_FEN,
        mode: Mode::Aggressive,
        elo: 0,
        chess960: false,
        roots: &[],
        depth: 2,
    },
    Case {
        fen: START_FEN,
        mode: Mode::Human,
        elo: 500,
        chess960: false,
        roots: &[],
        depth: 3,
    },
    Case {
        fen: START_FEN,
        mode: Mode::Analysis,
        elo: 500,
        chess960: false,
        roots: &[],
        depth: 2,
    },
    Case {
        fen: "7k/8/6KQ/8/8/8/8/8 w - - 149 1",
        mode: Mode::Balanced,
        elo: 0,
        chess960: false,
        roots: &[],
        depth: 2,
    },
    Case {
        fen: "4k3/P7/8/8/8/8/8/4K3 w - - 0 1",
        mode: Mode::Balanced,
        elo: 0,
        chess960: false,
        roots: &[],
        depth: 2,
    },
    Case {
        fen: "4k3/8/8/8/8/8/8/R4KR1 w GA - 0 1",
        mode: Mode::Balanced,
        elo: 0,
        chess960: true,
        roots: &["f1g1"],
        depth: 2,
    },
    Case {
        fen: "r3k3/4q3/8/8/8/8/q7/R2Q2KR w - - 0 1",
        mode: Mode::Balanced,
        elo: 0,
        chess960: false,
        roots: &["a1a2"],
        depth: 2,
    },
];

fn main() -> Result<(), Box<dyn Error>> {
    let mut output = io::stdout().lock();
    writeln!(output, "[")?;
    for (index, case) in CASES.iter().enumerate() {
        let mut engine = Engine::new()?;
        let mut options = engine.options();
        options.set_mode(case.mode);
        options.set_elo(case.elo)?;
        options.set_chess960(case.chess960);
        engine.configure(options)?;
        engine.set_position(case.fen, &[])?;
        engine.start_moves(
            SearchLimits {
                depth: case.depth,
                nodes: 20_000,
            },
            case.roots,
        )?;
        while engine.searching() {
            engine.step(256)?;
        }
        let report = engine.report();
        let status = match report.status {
            SearchStatus::Idle => "idle",
            SearchStatus::Running => "running",
            SearchStatus::Finished(reason) => match reason {
                Completion::Depth => "depth",
                Completion::Nodes => "nodes",
                Completion::Stopped => "stopped",
                Completion::Terminal => "terminal",
            },
        };
        let best = report.best_move.map_or_else(
            || "null".into(),
            |mv| format!("\"{}\"", mv.to_uci(case.chess960)),
        );
        let score = report
            .score_cp
            .map_or_else(|| "null".into(), |value| value.to_string());
        let mate = report
            .mate_in()
            .map_or_else(|| "null".into(), |value| value.to_string());
        let variations = report
            .variations
            .iter()
            .map(|line| {
                format!(
                    "{{\"scoreCp\":{},\"pv\":[{}]}}",
                    line.score_cp,
                    quoted(line.moves.iter().map(|mv| mv.to_uci(case.chess960)))
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let roots = quoted(case.roots.iter().map(ToString::to_string));
        let selected = report
            .variations
            .iter()
            .find(|line| line.moves.first().copied() == report.best_move);
        let pv = selected.map_or_else(String::new, |line| {
            quoted(line.moves.iter().map(|mv| mv.to_uci(case.chess960)))
        });
        let separator = if index + 1 == CASES.len() { "" } else { "," };
        writeln!(
            output,
            "{{\"fen\":\"{}\",\"mode\":\"{}\",\"elo\":{},\"chess960\":{},\"roots\":[{roots}],\"depth\":{},\"nodes\":\"20000\",\"report\":{{\"status\":\"{status}\",\"finished\":true,\"depth\":{},\"selectiveDepth\":{},\"nodes\":\"{}\",\"bestMove\":{best},\"scoreCp\":{score},\"mate\":{mate},\"pv\":[{pv}],\"variations\":[{variations}]}}}}{separator}",
            case.fen,
            case.mode.as_str(),
            case.elo,
            case.chess960,
            case.depth,
            report.depth,
            report.selective_depth,
            report.nodes
        )?;
    }
    writeln!(output, "]")?;
    Ok(())
}
fn quoted(items: impl Iterator<Item = String>) -> String {
    items
        .map(|item| format!("\"{item}\""))
        .collect::<Vec<_>>()
        .join(",")
}
