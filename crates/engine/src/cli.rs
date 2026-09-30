#![expect(
    clippy::missing_errors_doc,
    reason = "Failure details use plain prose rather than Markdown sections."
)]

use std::{error::Error, io::Write};

use gwaymaegyi_core::{Board, START_FEN, divide};

const HELP: &str = "gwaymaegyi 0.1.0 — portable chess-rules foundation\n\
Usage:\n\
  gwaymaegyi perft <depth> [FEN]\n\
  gwaymaegyi moves [FEN]\n\
  gwaymaegyi fen <FEN>\n\
  gwaymaegyi --version\n\
Castling uses standard UCI notation; search/UCI sessions are not implemented yet.\n";

/// Reports invalid commands, position errors, or output failures to the caller.
pub fn run(
    mut args: impl Iterator<Item = String>,
    output: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let Some(command) = args.next() else {
        output.write_all(HELP.as_bytes())?;
        return Ok(());
    };
    match command.as_str() {
        "--help" | "-h" => output.write_all(HELP.as_bytes())?,
        "--version" | "-V" => writeln!(output, "gwaymaegyi {}", env!("CARGO_PKG_VERSION"))?,
        "perft" => {
            let depth: u8 = args.next().ok_or("perft requires a depth")?.parse()?;
            if depth > 8 {
                return Err("perft depth must be between zero and eight".into());
            }
            let board = parse_board(args, false)?;
            let counts = divide(&board, depth);
            let total = if depth == 0 {
                1
            } else {
                counts.iter().map(|(_, nodes)| nodes).sum()
            };
            for (chess_move, nodes) in counts {
                writeln!(output, "{}: {nodes}", chess_move.to_uci(false))?;
            }
            writeln!(output, "{total} nodes")?;
        }
        "moves" => {
            let board = parse_board(args, false)?;
            let moves: Vec<_> = board
                .legal_moves()
                .into_iter()
                .map(|chess_move| chess_move.to_uci(false))
                .collect();
            writeln!(output, "{}", moves.join(" "))?;
        }
        "fen" => writeln!(output, "{}", parse_board(args, true)?)?,
        _ => return Err(format!("unknown command: {command}; try --help").into()),
    }
    Ok(())
}

fn parse_board(
    args: impl Iterator<Item = String>,
    required: bool,
) -> Result<Board, Box<dyn Error>> {
    let fen = args.collect::<Vec<_>>().join(" ");
    if fen.is_empty() {
        if required {
            return Err("fen requires a position".into());
        }
        Ok(START_FEN.parse()?)
    } else {
        Ok(fen.parse()?)
    }
}
