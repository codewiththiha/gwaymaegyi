//! Bounded static exchanges on one square with legal least-value recaptures.
//! Swap-list minimax allows either side to decline an unfavorable continuation.

use super::attacks::piece_attacks;
use crate::{Bitboard, Board, Color, Move, MoveKind, Piece, PieceKind, Square};

const fn value(kind: PieceKind) -> i32 {
    match kind {
        PieceKind::Pawn => 100,
        PieceKind::Knight | PieceKind::Bishop => 300,
        PieceKind::Rook => 500,
        PieceKind::Queen => 900,
        PieceKind::King => 20_000,
    }
}

impl Board {
    /// All geometric attackers under the supplied occupancy, including x-rays.
    pub(crate) fn exchange_attackers(&self, square: Square, occupied: Bitboard) -> Bitboard {
        let mut bits = 0;
        for color in [Color::White, Color::Black] {
            bits |= piece_attacks(PieceKind::Pawn, color.opposite(), square, occupied).0
                & self.pieces(color, PieceKind::Pawn).0;
            bits |= piece_attacks(PieceKind::Knight, color, square, occupied).0
                & self.pieces(color, PieceKind::Knight).0;
            bits |= piece_attacks(PieceKind::King, color, square, occupied).0
                & self.pieces(color, PieceKind::King).0;
            bits |= piece_attacks(PieceKind::Bishop, color, square, occupied).0
                & (self.pieces(color, PieceKind::Bishop).0
                    | self.pieces(color, PieceKind::Queen).0);
            bits |= piece_attacks(PieceKind::Rook, color, square, occupied).0
                & (self.pieces(color, PieceKind::Rook).0 | self.pieces(color, PieceKind::Queen).0);
        }
        Bitboard(bits & occupied.0)
    }

    /// Tests whether a generated move's static material gain reaches the threshold.
    /// This is a move-ordering estimate, not proof that a tactical move is sound.
    #[must_use]
    pub fn static_exchange_gain(&self, chess_move: Move, threshold: i32) -> bool {
        if chess_move.kind() == MoveKind::Castle {
            return threshold <= 0;
        }
        let Some(mover) = self.piece_on(chess_move.from()) else {
            return false;
        };
        if mover.color != self.side_to_move() {
            return false;
        }
        let captured = if chess_move.kind() == MoveKind::EnPassant {
            100
        } else {
            self.piece_on(chess_move.to())
                .map_or(0, |piece| value(piece.kind))
        };
        let promotion = match chess_move.kind() {
            MoveKind::Promotion(role) => value(role.piece_kind()) - 100,
            _ => 0,
        };
        let Some(mut position) = self.after_generated_move(chess_move) else {
            return false;
        };
        if position.in_check(mover.color) {
            return false;
        }
        let target = chess_move.to();
        let mut gains = [0_i32; 64];
        gains[0] = captured + promotion;
        let mut length = 1;
        let mut color = mover.color.opposite();
        while length < gains.len() {
            let Some((next, extra)) = position.exchange_recapture(target, color) else {
                break;
            };
            let victim = position
                .piece_on(target)
                .map_or(0, |piece| value(piece.kind));
            gains[length] = victim + extra - gains[length - 1];
            length += 1;
            position = next;
            color = color.opposite();
        }
        for index in (1..length).rev() {
            gains[index - 1] = -(-gains[index - 1]).max(gains[index]);
        }
        gains[0] >= threshold
    }

    fn exchange_recapture(&self, target: Square, color: Color) -> Option<(Self, i32)> {
        let victim = self.piece_on(target)?;
        if victim.color == color || victim.kind == PieceKind::King {
            return None;
        }
        let attackers = self.exchange_attackers(target, self.occupied()) & self.side_pieces(color);
        for kind in [
            PieceKind::Pawn,
            PieceKind::Knight,
            PieceKind::Bishop,
            PieceKind::Rook,
            PieceKind::Queen,
            PieceKind::King,
        ] {
            for from in attackers & self.pieces(color, kind) {
                let promotion = kind == PieceKind::Pawn && matches!(target.rank(), 0 | 7);
                let role = if promotion { PieceKind::Queen } else { kind };
                let mut child = *self;
                child.remove(from);
                child.remove(target);
                child.place(target, Piece::new(color, role));
                if !child.in_check(color) {
                    return Some((child, if promotion { value(role) - 100 } else { 0 }));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::Board;
    use std::error::Error;

    fn see(fen: &str, notation: &str, threshold: i32) -> Result<bool, Box<dyn Error>> {
        let board: Board = fen.parse()?;
        Ok(board.static_exchange_gain(board.resolve_uci(notation, false)?, threshold))
    }

    #[test]
    fn exchanges_allow_declining_bad_recaptures() -> Result<(), Box<dyn Error>> {
        assert!(see("4k3/8/8/3p4/4P3/8/8/4K3 w - - 0 1", "e4d5", 100)?);
        assert!(!see("4k3/8/8/3p4/4P3/8/8/4K3 w - - 0 1", "e4d5", 101)?);
        assert!(see("4k3/3r4/8/3p4/4P3/8/8/4K3 w - - 0 1", "e4d5", 0)?);
        assert!(!see("4k3/3r4/8/3p4/4P3/8/8/4K3 w - - 0 1", "e4d5", 1)?);
        // Black declines a queen-for-pawn recapture answered by the rook.
        assert!(see("4k3/3q4/8/3p4/4P3/8/8/3RK3 w - - 0 1", "e4d5", 100)?);
        assert!(!see("4k3/3q4/8/3p4/4P3/8/8/3RK3 w - - 0 1", "e4d5", 101)?);
        Ok(())
    }

    #[test]
    fn pawn_direction_pins_and_king_safety_are_respected() -> Result<(), Box<dyn Error>> {
        assert!(!see("4k3/8/2p5/3p4/4Q3/8/8/4K3 w - - 0 1", "e4d5", 0)?);
        assert!(see("2k5/8/2p5/3p4/4Q3/8/8/2R1K3 w - - 0 1", "e4d5", 100)?);
        assert!(see("8/8/2k5/3p4/4Q3/8/8/3RK3 w - - 0 1", "e4d5", 100)?);
        Ok(())
    }

    #[test]
    fn special_moves_keep_the_exchange_on_the_destination() -> Result<(), Box<dyn Error>> {
        assert!(see("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1", "e5d6", 100)?);
        assert!(see("4k3/P7/8/8/8/8/8/4K3 w - - 0 1", "a7a8n", 200)?);
        assert!(!see("4k3/P7/8/8/8/8/8/4K3 w - - 0 1", "a7a8n", 201)?);
        Ok(())
    }
}
