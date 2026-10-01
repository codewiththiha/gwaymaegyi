//! Exact raw scores and incremental-versus-refresh evaluation regressions.

use gwaymaegyi_core::{Board, Color, START_FEN};
use gwaymaegyi_eval::{Accumulator, Model};
use std::error::Error;

#[test]
fn scalar_scores_match_all_model_fixtures() -> Result<(), Box<dyn Error>> {
    for line in include_str!("scores.txt").lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        let fen = fields[..6].join(" ");
        let board: Board = fen.parse()?;
        for (index, model) in [Model::Balanced, Model::Endgame, Model::Aggressive]
            .into_iter()
            .enumerate()
        {
            let state = Accumulator::new(&board, model);
            for (side, color) in [Color::White, Color::Black].into_iter().enumerate() {
                let expected: i32 = fields[6 + index * 2 + side].parse()?;
                assert_eq!(
                    state.score_for(color),
                    expected,
                    "{fen}, {model:?}, {color:?}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn incremental_scores_match_refresh_for_special_moves() -> Result<(), Box<dyn Error>> {
    for fen in [
        START_FEN,
        "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
        "4k3/P7/8/3pP3/8/8/8/4K3 w - d6 0 1",
        "4k3/8/8/8/8/8/8/R4KR1 w GA - 0 1",
    ] {
        let board: Board = fen.parse()?;
        for model in [Model::Balanced, Model::Endgame, Model::Aggressive] {
            let root = Accumulator::new(&board, model);
            for child in board.legal_successors() {
                let updated = root.updated(child.board());
                let fresh = Accumulator::new(child.board(), model);
                assert_eq!(updated.score(), fresh.score());
                for next in child.board().legal_successors().into_iter().take(3) {
                    assert_eq!(
                        updated.updated(next.board()).score(),
                        Accumulator::new(next.board(), model).score()
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn long_deterministic_play_keeps_accumulators_in_sync() -> Result<(), Box<dyn Error>> {
    let mut board: Board = START_FEN.parse()?;
    let mut state = Accumulator::new(&board, Model::Balanced);
    let mut seed = 19_u64;
    for _ in 0..160 {
        let moves = board.legal_successors();
        if moves.is_empty() {
            break;
        }
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let index = usize::try_from(seed % moves.len() as u64)?;
        board = *moves[index].board();
        state = state.updated(&board);
        assert_eq!(
            state.score(),
            Accumulator::new(&board, Model::Balanced).score()
        );
    }
    Ok(())
}
