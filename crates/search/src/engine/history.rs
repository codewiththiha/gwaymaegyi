use crate::MAX_PLY;
use gwaymaegyi_core::{Board, Move};

#[derive(Debug)]
pub(super) struct History {
    quiet: Vec<i16>,
    killers: [Option<Move>; MAX_PLY as usize],
}
impl Default for History {
    fn default() -> Self {
        Self {
            quiet: vec![0; 2 * 64 * 64],
            killers: [None; MAX_PLY as usize],
        }
    }
}
impl History {
    const fn index(board: &Board, chess_move: Move) -> usize {
        board.side_to_move() as usize * 4096
            + chess_move.from().index() * 64
            + chess_move.to().index()
    }
    pub(super) fn score(&self, board: &Board, chess_move: Move, ply: u8) -> i32 {
        i32::from(self.quiet[Self::index(board, chess_move)])
            + if self.killers[usize::from(ply)] == Some(chess_move) {
                20_000
            } else {
                0
            }
    }
    pub(super) fn record(
        &mut self,
        board: &Board,
        chess_move: Move,
        ply: u8,
        depth: i16,
        good: bool,
    ) {
        if board.is_capture(chess_move) {
            return;
        }
        let bonus = (i32::from(depth.max(1)) * 291).min(2476) * if good { 1 } else { -1 };
        let entry = &mut self.quiet[Self::index(board, chess_move)];
        let value = i32::from(*entry);
        *entry = i16::try_from(value + bonus - value * bonus.abs() / 16_384).unwrap_or(0);
        if good {
            self.killers[usize::from(ply)] = Some(chess_move);
        }
    }
}
