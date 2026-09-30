use std::error::Error;

use gwaymaegyi_core::{Bitboard, Board, Color, START_FEN, Square};

#[test]
fn start_position_round_trips() -> Result<(), Box<dyn Error>> {
    let board: Board = START_FEN.parse()?;
    assert_eq!(board.to_string(), START_FEN);
    assert_eq!(board.occupied().len(), 32);
    assert_eq!(board.legal_moves().len(), 20);
    assert_eq!(board.side_to_move(), Color::White);
    Ok(())
}

#[test]
fn squares_reject_off_board_and_unicode_input() -> Result<(), Box<dyn Error>> {
    assert!(Square::from_index(64).is_none());
    assert!(Square::new(8, 0).is_none());
    for text in ["a0", "i2", "a11", "é", "a"] {
        assert!(text.parse::<Square>().is_err());
    }
    for index in 0..64 {
        let square = Square::from_index(index).ok_or("square construction failed")?;
        assert_eq!(square.to_string().parse::<Square>()?, square);
    }
    assert_eq!(Bitboard(0).into_iter().next(), None);
    assert_eq!(Bitboard(u64::MAX).into_iter().count(), 64);
    Ok(())
}

#[test]
fn malformed_fens_are_errors_not_panics() {
    for fen in [
        "",
        "8/8/8/8/8/8/8/8 w - - 0 1",
        "9/8/8/8/8/8/4k3/4K3 w - - 0 1",
        "8/8/8/8/8/8/4k3/4K3 w - - 0 0",
        "8/8/8/8/8/8/4k3/4K3 w K - 0 1",
        "8/8/8/8/8/8/4k3/4K3 w - e6 0 1",
    ] {
        assert!(fen.parse::<Board>().is_err(), "{fen}");
    }
}

#[test]
fn play_updates_counters_and_en_passant() -> Result<(), Box<dyn Error>> {
    let board: Board = START_FEN.parse()?;
    let board = board.play_uci("e2e4", false)?;
    assert_eq!(
        board.to_string(),
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"
    );
    let board = board.play_uci("g8f6", false)?;
    assert!(board.to_string().ends_with("w KQkq - 1 2"));
    assert!(board.play_uci("e1e8", false).is_err());
    assert!(board.play_uci("😊", false).is_err());
    Ok(())
}

#[test]
fn en_passant_cannot_expose_the_king() -> Result<(), Box<dyn Error>> {
    let board: Board = "4k3/8/8/r4pPK/8/8/8/8 w - f6 0 1".parse()?;
    assert!(
        !board
            .legal_moves()
            .iter()
            .any(|chess_move| chess_move.to_uci(false) == "g5f6")
    );
    Ok(())
}

#[test]
fn promotion_has_exactly_four_choices() -> Result<(), Box<dyn Error>> {
    let board: Board = "4k3/P7/8/8/8/8/8/4K3 w - - 0 1".parse()?;
    let count = board
        .legal_moves()
        .iter()
        .filter(|chess_move| chess_move.to_uci(false).starts_with("a7a8"))
        .count();
    assert_eq!(count, 4);
    assert!(board.play_uci("a7a8k", false).is_err());
    assert!(
        board
            .play_uci("a7a8q", false)?
            .to_string()
            .starts_with("Q3k3/")
    );
    Ok(())
}

#[test]
fn standard_castles_use_the_expected_notation() -> Result<(), Box<dyn Error>> {
    let board: Board = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1".parse()?;
    let moves: Vec<_> = board
        .legal_moves()
        .iter()
        .map(|chess_move| chess_move.to_uci(false))
        .collect();
    assert!(moves.iter().any(|value| value == "e1g1"));
    assert!(moves.iter().any(|value| value == "e1c1"));
    assert_eq!(
        board.play_uci("e1g1", false)?.to_string(),
        "r3k2r/8/8/8/8/8/8/R4RK1 b kq - 1 1"
    );
    Ok(())
}

#[test]
fn chess960_handles_stationary_and_swapping_pieces() -> Result<(), Box<dyn Error>> {
    for (fen, notation, expected) in [
        (
            "4k3/8/8/8/8/8/8/R5KR w HA - 0 1",
            "g1h1",
            "4k3/8/8/8/8/8/8/R4RK1 b - - 1 1",
        ),
        (
            "4k3/8/8/8/8/8/8/R4KR1 w GA - 0 1",
            "f1g1",
            "4k3/8/8/8/8/8/8/R4RK1 b - - 1 1",
        ),
        (
            "4k3/8/8/8/8/8/8/R3KR2 w FA - 0 1",
            "e1f1",
            "4k3/8/8/8/8/8/8/R4RK1 b - - 1 1",
        ),
    ] {
        let board: Board = fen.parse()?;
        assert_eq!(board.play_uci(notation, true)?.to_string(), expected);
    }
    Ok(())
}

#[test]
fn chess960_rook_cannot_uncover_a_stationary_king() -> Result<(), Box<dyn Error>> {
    let board: Board = "4k3/8/8/8/8/8/8/rRK5 w B - 0 1".parse()?;
    assert!(board.play_uci("c1b1", true).is_err());
    Ok(())
}
