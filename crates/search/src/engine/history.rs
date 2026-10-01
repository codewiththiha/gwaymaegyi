//! Worker-local ordering memory: quiet, capture, and continuation histories.

use crate::MAX_PLY;
use gwaymaegyi_core::{Board, Move, Piece, Square};

const PIECE_KINDS: usize = 6;
const CORR_BUCKETS: usize = 8192;

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
    pawn_correction: Vec<i16>,
    non_pawn_correction: [Vec<i16>; 2],
    continuation_correction: Vec<i16>,
    killers: [Option<Move>; MAX_PLY as usize],
}
impl Default for History {
    fn default() -> Self {
        Self {
            quiet: vec![0; 2 * PIECE_KINDS * 64],
            capture: vec![0; 2 * PIECE_KINDS * 64 * 64],
            continuation: vec![0; 2 * PIECE_KINDS * 64 * 2 * PIECE_KINDS * 64],
            pawn_correction: vec![0; 2 * CORR_BUCKETS],
            non_pawn_correction: std::array::from_fn(|_| vec![0; 2 * CORR_BUCKETS]),
            continuation_correction: vec![0; 2 * PIECE_KINDS * 64 * 2 * PIECE_KINDS * 64],
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

    fn correction_index(key: u64) -> usize {
        usize::try_from((key ^ (key >> 32)) & (CORR_BUCKETS as u64 - 1)).unwrap_or(0)
    }

    const fn correction_pair(previous: PriorMove, current: PriorMove) -> Option<usize> {
        let (Some(piece), Some(to)) = (current.piece, current.to) else {
            return None;
        };
        Self::continuation_index(previous, piece, to)
    }

    /// Pawn, per-side non-pawn, and preceding-move corrections for static evaluation.
    pub(super) fn correction(&self, board: &Board, priors: PriorMoves) -> i32 {
        let side = board.side_to_move().index();
        let key = board.key();
        let mut sum = i32::from(
            self.pawn_correction[side * CORR_BUCKETS + Self::correction_index(key.pawns())],
        );
        for color in [gwaymaegyi_core::Color::White, gwaymaegyi_core::Color::Black] {
            let index = side * CORR_BUCKETS + Self::correction_index(key.non_pawns(color));
            sum += i32::from(self.non_pawn_correction[color.index()][index]);
        }
        for index in [
            Self::correction_pair(priors.our_last, priors.their_last),
            Self::correction_pair(priors.our_earlier, priors.our_last),
        ]
        .into_iter()
        .flatten()
        {
            sum += i32::from(self.continuation_correction[index]);
        }
        sum
    }

    /// Train correction entries from exact quiet-node residuals, not tactical bounds.
    pub(super) fn update_correction(
        &mut self,
        board: &Board,
        priors: PriorMoves,
        best: i32,
        static_eval: i32,
        depth: i16,
    ) {
        let bonus = ((best - static_eval) * i32::from(depth.max(1)) / 8).clamp(-256, 256);
        let side = board.side_to_move().index();
        let key = board.key();
        Self::update_entry(
            &mut self.pawn_correction[side * CORR_BUCKETS + Self::correction_index(key.pawns())],
            bonus,
        );
        for color in [gwaymaegyi_core::Color::White, gwaymaegyi_core::Color::Black] {
            let index = side * CORR_BUCKETS + Self::correction_index(key.non_pawns(color));
            Self::update_entry(&mut self.non_pawn_correction[color.index()][index], bonus);
        }
        for index in [
            Self::correction_pair(priors.our_last, priors.their_last),
            Self::correction_pair(priors.our_earlier, priors.our_last),
        ]
        .into_iter()
        .flatten()
        {
            Self::update_entry(&mut self.continuation_correction[index], bonus);
        }
    }

    fn update_entry(entry: &mut i16, delta: i32) {
        let value = i32::from(*entry);
        *entry = i16::try_from(value + delta - value * delta.abs() / 1024)
            .unwrap_or(if delta < 0 { i16::MIN } else { i16::MAX });
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

#[cfg(test)]
mod tests {
    use super::{History, PriorMove, PriorMoves};
    use gwaymaegyi_core::Board;
    use std::error::Error;

    #[test]
    fn correction_tracks_pawns_sides_and_prior_moves() -> Result<(), Box<dyn Error>> {
        let board: Board = "4k3/8/8/8/8/8/4P3/4K3 w - - 0 1".parse()?;
        let prior = PriorMoves {
            their_last: PriorMove {
                piece: board.piece_on("e2".parse()?),
                to: Some("e4".parse()?),
            },
            our_last: PriorMove {
                piece: board.piece_on("e2".parse()?),
                to: Some("e3".parse()?),
            },
            our_earlier: PriorMove::default(),
        };
        let mut history = History::default();
        assert_eq!(history.correction(&board, prior), 0);
        history.update_correction(&board, prior, 250, 50, 8);
        assert!(history.correction(&board, prior) > 0);
        let no_prior = history.correction(&board, PriorMoves::default());
        assert_eq!(no_prior, history.correction(&board, PriorMoves::default()));
        assert_ne!(no_prior, history.correction(&board, prior));
        let black: Board = "4k3/4p3/8/8/8/8/8/4K3 b - - 0 1".parse()?;
        assert_ne!(
            history.correction(&black, prior),
            history.correction(&board, prior)
        );
        let rotated: Board = "4k3/8/8/8/8/4P3/8/4K3 w - - 0 1".parse()?;
        assert_ne!(
            history.correction(&rotated, prior),
            history.correction(&board, prior)
        );
        Ok(())
    }
}
