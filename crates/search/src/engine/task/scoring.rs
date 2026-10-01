use super::Task;
use crate::Mode;
use crate::engine::{frame::Frame, table::Cache};
use gwaymaegyi_core::{MoveKind, PieceKind};

impl Task {
    pub(super) fn evaluate(&self, frame: &Frame) -> i32 {
        let raw = frame.accumulator.score().clamp(-40_000, 40_000);
        let mut score = raw * 100 / 195;
        if matches!(self.options.mode(), Mode::Aggressive | Mode::Human) {
            let color = self.root.side_to_move();
            let total = frame.board.material(color) + frame.board.material(color.opposite());
            score = score * (750 + total / 25) / 1024;
            let lost = self.root.material(color) - frame.board.material(color);
            let enemy_lost =
                self.root.material(color.opposite()) - frame.board.material(color.opposite());
            if total > 4500 && lost > enemy_lost + 100 {
                let favorable = if frame.board.side_to_move() == color {
                    score > 0
                } else {
                    score < 0
                };
                if favorable {
                    score += if frame.board.side_to_move() == color {
                        30
                    } else {
                        -30
                    };
                }
            }
        }
        score = score * (200 - i32::try_from(frame.board.halfmove_clock().min(100)).unwrap_or(100))
            / 200;
        score.clamp(-28_000, 28_000)
    }

    pub(super) fn order(frame: &mut Frame, cache: &Cache) {
        let tt = cache.probe(frame.key).and_then(|entry| entry.best);
        frame.candidates.sort_by_cached_key(|candidate| {
            let chess_move = candidate.chess_move();
            if Some(chess_move) == tt {
                return -1_000_000;
            }
            let victim = frame
                .board
                .piece_on(chess_move.to())
                .map_or(0, |piece| Self::piece_value(piece.kind));
            let attacker = frame
                .board
                .piece_on(chess_move.from())
                .map_or(0, |piece| Self::piece_value(piece.kind));
            let promotion = if matches!(chess_move.kind(), MoveKind::Promotion(_)) {
                100_000
            } else {
                0
            };
            let capture = if frame.board.is_capture(chess_move) {
                200_000 + victim * 16 - attacker
            } else {
                0
            };
            -(promotion + capture + cache.history.score(&frame.board, chess_move, frame.ply))
        });
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
            .skip(1)
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
}
