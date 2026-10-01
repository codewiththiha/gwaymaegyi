//! Worker-local ordering memory: quiet, capture, and continuation histories.

use crate::MAX_PLY;
use gwaymaegyi_core::{Board, Move, Piece, Square};

const PIECE_KINDS: usize = 6;

const fn piece_index(piece: Piece) -> usize {
    piece.color.index() * PIECE_KINDS + piece.kind.index()
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct PriorMove {
    pub piece: Option<Piece>,
    pub to: Option<Square>,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct PriorMoves {
    pub their_last: PriorMove,
    pub our_last: PriorMove,
    pub our_earlier: PriorMove,
}

#[derive(Debug)]
pub(super) struct History {
    quiet: Vec<i16>,
    capture: Vec<i16>,
    continuation: Vec<i16>,
    killers: [Option<Move>; MAX_PLY as usize],
}
impl Default for History {
    fn default() -> Self {
        Self {
            quiet: vec![0; 2 * PIECE_KINDS * 64],
            capture: vec![0; 2 * PIECE_KINDS * 64 * 64],
            continuation: vec![0; 2 * PIECE_KINDS * 64 * 2 * PIECE_KINDS * 64],
            killers: [None; MAX_PLY as usize],
        }
    }
}

impl History {
    const fn quiet_index(piece: Piece, to: Square) -> usize {
        piece_index(piece) * 64 + to.index()
    }

    const fn capture_index(piece: Piece, from: Square, to: Square) -> usize {
        piece_index(piece) * 4096 + from.index() * 64 + to.index()
    }

    const fn continuation_index(previous: PriorMove, piece: Piece, to: Square) -> Option<usize> {
        let (Some(previous_piece), Some(previous_to)) = (previous.piece, previous.to) else {
            return None;
        };
        Some(
            (piece_index(previous_piece) * 64 + previous_to.index()) * (2 * PIECE_KINDS * 64)
                + piece_index(piece) * 64
                + to.index(),
        )
    }

    pub(super) fn quiet_only(&self, board: &Board, chess_move: Move) -> i32 {
        let from = chess_move.from();
        let to = chess_move.to();
        let Some(piece) = board.piece_on(from) else {
            return 0;
        };
        i32::from(self.quiet[Self::quiet_index(piece, to)])
    }

    /// Score a quiet move with up to three continuation-history contributions.
    pub(super) fn quiet_score(
        &self,
        board: &Board,
        chess_move: Move,
        ply: u8,
        priors: PriorMoves,
    ) -> i32 {
        let from = chess_move.from();
        let to = chess_move.to();
        let Some(piece) = board.piece_on(from) else {
            return 0;
        };
        if self.killers[usize::from(ply)] == Some(chess_move) {
            return 100_000;
        }
        let mut score = i32::from(self.quiet[Self::quiet_index(piece, to)]);
        if let Some(index) = Self::continuation_index(priors.their_last, piece, to) {
            score += 2 * i32::from(self.continuation[index]);
        }
        if let Some(index) = Self::continuation_index(priors.our_last, piece, to) {
            score += i32::from(self.continuation[index]);
        }
        if let Some(index) = Self::continuation_index(priors.our_earlier, piece, to) {
            score += i32::from(self.continuation[index]);
        }
        score
    }

    pub(super) fn capture_adjustment(&self, piece: Piece, from: Square, to: Square) -> i32 {
        i32::from(self.capture[Self::capture_index(piece, from, to)])
    }

    fn update_quiet(&mut self, piece: Piece, to: Square, delta: i32) {
        let index = Self::quiet_index(piece, to);
        let value = i32::from(self.quiet[index]);
        let next = value + delta - value * delta.abs() / 16_384;
        self.quiet[index] =
            i16::try_from(next).unwrap_or(if next < 0 { i16::MIN } else { i16::MAX });
    }

    fn update_capture(&mut self, piece: Piece, from: Square, to: Square, delta: i32) {
        let index = Self::capture_index(piece, from, to);
        let value = i32::from(self.capture[index]);
        let next = value + delta - value * delta.abs() / 16_384;
        self.capture[index] =
            i16::try_from(next).unwrap_or(if next < 0 { i16::MIN } else { i16::MAX });
    }

    fn update_continuation(&mut self, previous: PriorMove, piece: Piece, to: Square, delta: i32) {
        if let Some(index) = Self::continuation_index(previous, piece, to) {
            let value = i32::from(self.continuation[index]);
            let next = value + delta - value * delta.abs() / 16_384;
            self.continuation[index] =
                i16::try_from(next).unwrap_or(if next < 0 { i16::MIN } else { i16::MAX });
        }
    }

    /// Record a quiet move that produced a beta cutoff, including continuations.
    pub(super) fn record_cutoff(
        &mut self,
        board: &Board,
        chess_move: Move,
        ply: u8,
        bonus: i32,
        priors: PriorMoves,
    ) {
        let from = chess_move.from();
        let to = chess_move.to();
        let Some(piece) = board.piece_on(from) else {
            return;
        };
        if board.is_capture(chess_move) {
            self.update_capture(piece, from, to, bonus);
        } else {
            self.update_quiet(piece, to, bonus);
            self.update_continuation(priors.their_last, piece, to, 2 * bonus);
            self.update_continuation(priors.our_last, piece, to, bonus);
            self.update_continuation(priors.our_earlier, piece, to, bonus);
            if usize::from(ply) < MAX_PLY as usize {
                self.killers[usize::from(ply)] = Some(chess_move);
            }
        }
    }

    pub(super) fn record_malus(&mut self, board: &Board, chess_move: Move, malus: i32) {
        let from = chess_move.from();
        let to = chess_move.to();
        let Some(piece) = board.piece_on(from) else {
            return;
        };
        if board.is_capture(chess_move) {
            self.update_capture(piece, from, to, malus);
        } else {
            self.update_quiet(piece, to, malus);
        }
    }
}
