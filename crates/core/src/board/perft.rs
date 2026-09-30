use crate::{Board, Move};

/// Counts legal leaf positions; depth zero includes the current position once.
#[must_use]
pub fn perft(board: &Board, depth: u8) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = board.legal_moves();
    if depth == 1 {
        return moves.len() as u64;
    }
    moves
        .into_iter()
        .filter_map(|chess_move| board.after_generated_move(chess_move))
        .map(|child| perft(&child, depth - 1))
        .sum()
}

/// Root-level counts make move-generation discrepancies easier to locate.
#[must_use]
pub fn divide(board: &Board, depth: u8) -> Vec<(Move, u64)> {
    if depth == 0 {
        return Vec::new();
    }
    board
        .legal_moves()
        .into_iter()
        .filter_map(|chess_move| {
            board
                .after_generated_move(chess_move)
                .map(|child| (chess_move, perft(&child, depth - 1)))
        })
        .collect()
}
