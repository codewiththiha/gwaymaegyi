//! Static-score policy delegation, staged move ordering, and repetition checks.

use super::Task;
use crate::engine::{
    frame::{Frame, ScoredMove},
    history::History,
};
use gwaymaegyi_core::{MoveKind, PieceKind, Successor};

impl Task {
    pub(super) fn evaluate(&self, frame: &Frame) -> i32 {
        let sacrifice = if matches!(
            self.options.mode(),
            crate::Mode::Aggressive | crate::Mode::Human
        ) && self.game.material_history().len() + usize::from(frame.ply) >= 6
        {
            self.path_sacrifice(frame)
        } else {
            0
        };
        self.options.evaluate_with_sacrifice(
            &self.root,
            &frame.accumulator,
            &frame.board,
            sacrifice,
        )
    }

    fn path_sacrifice(&self, frame: &Frame) -> i32 {
        let base = self.game.material_history();
        let mut balances = Vec::with_capacity(base.len() + usize::from(frame.ply));
        balances.extend_from_slice(base);
        for parent in &self.frames {
            if parent.ply > 0 && parent.ply < frame.ply && parent.excluded.is_none() {
                let side = parent.board.side_to_move();
                let diff = parent.board.material(side) - parent.board.material(side.opposite());
                balances.push(diff);
            }
        }
        let side = frame.board.side_to_move();
        let current = frame.board.material(side) - frame.board.material(side.opposite());
        if frame.ply > 0 {
            balances.push(current);
        }
        crate::Options::detect_sacrifice(&balances, current)
    }

    /// TT move, queen promotions, SEE-verified captures, quiet moves, then
    /// SEE-failing captures; capture scores scale victim against attacker.
    pub(super) fn order(
        frame: &mut Frame,
        history: &History,
        successors: Vec<Successor>,
        priors: crate::engine::history::PriorMoves,
        worker_id: u8,
    ) {
        let tt = frame.tt_move;
        let mut scored = Vec::with_capacity(successors.len());
        for candidate in successors {
            let chess_move = candidate.chess_move();
            let is_capture = frame.board.is_capture(chess_move);
            let is_promo = matches!(chess_move.kind(), MoveKind::Promotion(_));
            let score = if Some(chess_move) == tt {
                10_000_000
            } else if is_promo && !is_capture {
                5_000_000
            } else if is_capture {
                let victim = frame
                    .board
                    .piece_on(chess_move.to())
                    .map_or(100, |piece| Self::piece_value(piece.kind));
                let attacker = frame
                    .board
                    .piece_on(chess_move.from())
                    .map_or(0, |piece| Self::piece_value(piece.kind));
                let piece = frame.board.piece_on(chess_move.from());
                let see_adjustment = piece.map_or(0, |piece| {
                    (history.capture_adjustment(piece, chess_move.from(), chess_move.to()) / 128)
                        .clamp(-128, 128)
                });
                let see_ok = frame
                    .board
                    .static_exchange_gain(chess_move, -107 - see_adjustment);
                2_000_000 + victim * 100 - attacker / 100
                    + (piece.map_or(0, |piece| {
                        history.capture_adjustment(piece, chess_move.from(), chess_move.to())
                    }))
                    - if see_ok { 0 } else { 10_000_000 }
            } else {
                let base = history.quiet_score(&frame.board, chess_move, frame.ply, priors);
                if worker_id == 0 {
                    base
                } else {
                    let salt = i32::from(worker_id) * 17;
                    let idx = chess_move.from().index() * 64 + chess_move.to().index();
                    let hash = i32::try_from(idx).unwrap_or(0);
                    base + (((hash ^ salt) & 0x1F) - 16)
                }
            };
            scored.push(ScoredMove {
                successor: candidate,
                score,
                is_capture,
                is_quiet: !is_capture && !is_promo,
            });
        }
        scored.sort_by_key(|item| std::cmp::Reverse(item.score));
        frame.candidates = scored;
    }

    const fn piece_value(kind: PieceKind) -> i32 {
        match kind {
            PieceKind::Pawn => 100,
            PieceKind::Knight | PieceKind::Bishop => 300,
            PieceKind::Rook => 500,
            PieceKind::Queen => 900,
            PieceKind::King => 20_000,
        }
    }

    pub(super) fn repetitions(&self, frame: &Frame) -> usize {
        let path = self
            .frames
            .iter()
            .filter(|parent| parent.ply > 0 && parent.ply < frame.ply && parent.excluded.is_none())
            .rev()
            .map(|parent| parent.board.key().full());
        let mut base = self.game.position_history();
        if frame.ply == 0 {
            base = &base[..base.len().saturating_sub(1)];
        }
        let window = usize::try_from(frame.board.halfmove_clock()).unwrap_or(usize::MAX);
        1 + path
            .chain(base.iter().rev().copied())
            .take(window)
            .filter(|&key| key == frame.board.key().full())
            .count()
    }

    /// Actual path moves, never a parent's earlier best candidate.
    pub(super) fn priors(&self) -> crate::engine::history::PriorMoves {
        use crate::engine::{
            frame::{Probe, Stage},
            history::{PriorMove, PriorMoves},
        };
        let path: Vec<_> = self
            .frames
            .iter()
            .filter_map(|parent| {
                let (Stage::Waiting(pending) | Stage::Returned(pending, _)) = &parent.stage else {
                    return None;
                };
                if pending.probe == Probe::Singular {
                    return None;
                }
                if pending.probe == Probe::Null {
                    return Some(PriorMove::default());
                }
                let chess_move = parent.candidates.get(pending.index)?.chess_move();
                Some(PriorMove {
                    piece: parent.board.piece_on(chess_move.from()),
                    to: Some(chess_move.to()),
                })
            })
            .collect();
        let prior = |offset: usize| {
            path.len()
                .checked_sub(offset)
                .and_then(|index| path.get(index))
                .copied()
                .unwrap_or_default()
        };
        PriorMoves {
            their_last: prior(1),
            our_last: prior(2),
            our_earlier: prior(4),
        }
    }
}
