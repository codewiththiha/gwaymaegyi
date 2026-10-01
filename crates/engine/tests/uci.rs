use gwaymaegyi::run_uci;
use std::{error::Error, io::Cursor};

#[test]
fn handshake_and_invalid_updates_remain_protocol_clean() -> Result<(), Box<dyn Error>> {
    let input=Cursor::new(b"uci\nisready\nsetoption name UCI_Elo value 499\nposition startpos moves e2e5\ngo depth 0\nisready\n".to_vec());
    let mut output = Vec::new();
    run_uci(input, &mut output)?;
    let text = String::from_utf8(output)?;
    assert!(text.contains("uciok\n"));
    assert_eq!(text.matches("readyok\n").count(), 2);
    assert_eq!(text.matches("info string rejected:").count(), 3);
    assert!(text.contains("uncalibrated"));
    Ok(())
}

#[test]
fn stop_and_eof_return_a_legal_root_fallback_once() -> Result<(), Box<dyn Error>> {
    let input = Cursor::new(b"position startpos\ngo infinite\nstop\nstop\n".to_vec());
    let mut output = Vec::new();
    run_uci(input, &mut output)?;
    let text = String::from_utf8(output)?;
    let lines: Vec<_> = text
        .lines()
        .filter_map(|line| line.strip_prefix("bestmove "))
        .collect();
    assert_eq!(lines.len(), 1);
    let board: gwaymaegyi_core::Board = gwaymaegyi_core::START_FEN.parse()?;
    board.resolve_uci(lines[0], false)?;
    Ok(())
}
