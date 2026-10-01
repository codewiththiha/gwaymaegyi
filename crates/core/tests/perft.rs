//! Standard legal-tree fixtures that detect move-generation regressions.

use std::error::Error;

use gwaymaegyi_core::{Board, START_FEN, perft};

#[test]
fn standard_perft_suite() -> Result<(), Box<dyn Error>> {
    let cases = [
        (START_FEN, 4, 197_281),
        (
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            3,
            97_862,
        ),
        ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 4, 43_238),
        (
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            3,
            9_467,
        ),
        (
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            3,
            62_379,
        ),
        (
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
            3,
            89_890,
        ),
    ];
    for (fen, depth, expected) in cases {
        let board: Board = fen.parse()?;
        assert_eq!(perft(&board, depth), expected, "{fen}, depth {depth}");
    }
    Ok(())
}

#[test]
fn zero_depth_includes_terminal_positions() -> Result<(), Box<dyn Error>> {
    let mate: Board = "7k/6Q1/6K1/8/8/8/8/8 b - - 0 1".parse()?;
    assert_eq!(mate.legal_moves(), Vec::new());
    assert_eq!(perft(&mate, 0), 1);
    assert_eq!(perft(&mate, 1), 0);
    Ok(())
}
