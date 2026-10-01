//! Testable native utility commands with injected arguments and output.

#![expect(
    clippy::missing_errors_doc,
    reason = "Failure details use plain prose rather than Markdown sections."
)]

use std::{error::Error, fs, io::Write};

use gwaymaegyi_core::{Board, START_FEN, divide};
use gwaymaegyi_search::{FilterKind, decode_bullet_records, encode_bullet_records, filter_lines};

const HELP: &str = "gwaymaegyi 0.1.0 — portable chess engine\n\
Usage:\n\
  gwaymaegyi [uci]\n\
  gwaymaegyi perft <depth> [FEN]\n\
  gwaymaegyi moves [FEN]\n\
  gwaymaegyi fen <FEN>\n\
  gwaymaegyi convert <decode|encode> <input> <output>\n\
  gwaymaegyi filter <1..11|name> <input> [output]\n\
  gwaymaegyi --version\n\
No arguments starts a UCI session. Elo presets are uncalibrated.\n";

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
        "convert" => run_convert(args, output)?,
        "filter" => run_filter(args, output)?,
        _ => return Err(format!("unknown command: {command}; try --help").into()),
    }
    Ok(())
}

fn run_convert(
    mut args: impl Iterator<Item = String>,
    output: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let mode = args.next().ok_or("convert requires decode or encode")?;
    let input_path = args.next().ok_or("convert requires an input path")?;
    let output_path = args.next().ok_or("convert requires an output path")?;
    if args.next().is_some() {
        return Err("convert takes exactly three arguments".into());
    }
    match mode.as_str() {
        "decode" => {
            let bytes = fs::read(input_path)?;
            let records = decode_bullet_records(&bytes)?;
            let mut body = String::new();
            for record in &records {
                body.push_str(&record.to_string());
                body.push('\n');
            }
            fs::write(output_path, body)?;
            writeln!(output, "decoded {} records", records.len())?;
        }
        "encode" => {
            let text = fs::read_to_string(input_path)?;
            let bytes = encode_bullet_records(&text)?;
            let count = bytes.len() / gwaymaegyi_search::BULLET_RECORD_BYTES;
            fs::write(output_path, bytes)?;
            writeln!(output, "encoded {count} records")?;
        }
        _ => return Err(format!("unknown convert mode: {mode}; expected decode or encode").into()),
    }
    Ok(())
}

fn run_filter(
    mut args: impl Iterator<Item = String>,
    output: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let raw_filter = args.next().ok_or("filter requires a filter id or name")?;
    let filter = FilterKind::parse(&raw_filter)
        .ok_or_else(|| format!("unknown position filter: {raw_filter}"))?;
    let input_path = args.next().ok_or("filter requires an input path")?;
    let output_path = args.next();
    if args.next().is_some() {
        return Err("filter takes at most three arguments".into());
    }
    let text = fs::read_to_string(input_path)?;
    let matched = filter_lines(&text, filter)?;
    if let Some(destination) = output_path {
        let mut body = String::new();
        for record in &matched {
            body.push_str(&record.to_string());
            body.push('\n');
        }
        fs::write(destination, body)?;
        writeln!(
            output,
            "filtered {} records ({})",
            matched.len(),
            filter.name()
        )?;
    } else {
        for record in &matched {
            writeln!(output, "{record}")?;
        }
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
