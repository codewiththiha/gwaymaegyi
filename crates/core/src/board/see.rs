//! Static exchange evaluation with cheapest-recapturer simulation.
//! The exchange ends when a side cannot recapture; the surviving piece stands.

use super::attacks::piece_attacks;
use crate::{Bitboard, Board, Color, Move, MoveKind, PieceKind, Square};

const fn see_value(kind: PieceKind) -> i32 {
    match kind {
        PieceKind::Pawn => 100,
        PieceKind::Knight | PieceKind::Bishop => 300,
        PieceKind::Rook => 500,
        PieceKind::Queen => 900,
        PieceKind::King => 20_000,
    }
}

fn lowest(attackers: Bitboard, candidates: Bitboard) -> Option<Square> {
    (attackers & candidates).into_iter().next()
}

impl Board {
    /// All pieces that attack the square, including kings for exchange purposes.
    pub(crate) fn exchange_attackers(&self, square: Square, occupied: Bitboard) -> Bitboard {
        let mut bits = 0;
        for color in [Color::White, Color::Black] {
            let opposite = color.opposite();
            let kind = |kind| piece_attacks(kind, color, square, occupied).0;
            bits |= kind(PieceKind::Knight) & self.pieces(color, PieceKind::Knight).0;
            // A pawn of `color` attacks `square` from where the opposite color's pawn attacks back.
            bits |= kind(PieceKind::Pawn) & self.pieces(color, PieceKind::Pawn).0;
            bits |= kind(PieceKind::King) & self.pieces(color, PieceKind::King).0;
            let diagonals = piece_attacks(PieceKind::Bishop, opposite, square, occupied).0;
            bits |= diagonals
                & (self.pieces(color, PieceKind::Bishop).0
                    | self.pieces(color, PieceKind::Queen).0);
            let lines = piece_attacks(PieceKind::Rook, opposite, square, occupied).0;
            bits |= lines
                & (self.pieces(color, PieceKind::Rook).0 | self.pieces(color, PieceKind::Queen).0);
        }
        Bitboard(bits)
    }

    /// True when the move's capture sequence wins more material than the threshold.
    #[must_use]
    pub fn static_exchange_gain(&self, chess_move: Move, threshold: i32) -> bool {
        let side = self.side_to_move();
        let to = chess_move.to();
        let (victim_value, target) = match self.piece_on(to) {
            Some(victim) if chess_move.kind() != MoveKind::Castle => (see_value(victim.kind), to),
            _ if matches!(chess_move.kind(), MoveKind::EnPassant) => {
                let pawn = to.offset(0, -side.pawn_step()).unwrap_or(to);
                let value = self
                    .piece_on(pawn)
                    .map_or(see_value(PieceKind::Pawn), |piece| see_value(piece.kind));
                (value, pawn)
            }
            _ => (0, to),
        };
        let attacker_kind = if matches!(chess_move.kind(), MoveKind::Promotion(_)) {
            PieceKind::Queen
        } else {
            self.piece_on(chess_move.from())
                .map_or(PieceKind::King, |piece| piece.kind)
        };
        let mut net = victim_value;
        let mut on_square_value = see_value(attacker_kind);
        let mut on_square_owner = side;
        let mut occupied = self.occupied().0 & !chess_move.from().bit();
        let mut attackers = self.exchange_attackers(target, Bitboard(occupied));
        let bishops = self.pieces(Color::White, PieceKind::Bishop)
            | self.pieces(Color::White, PieceKind::Queen)
            | self.pieces(Color::Black, PieceKind::Bishop)
            | self.pieces(Color::Black, PieceKind::Queen);
        let rooks = self.pieces(Color::White, PieceKind::Rook)
            | self.pieces(Color::White, PieceKind::Queen)
            | self.pieces(Color::Black, PieceKind::Rook)
            | self.pieces(Color::Black, PieceKind::Queen);
        let mut side_to_move = side.opposite();
        for _ in 0..8 {
            let ours = attackers & self.side_pieces(side_to_move) & Bitboard(occupied);
            let Some(capturer) = lowest(ours, self.pieces(side_to_move, PieceKind::Pawn))
                .or_else(|| lowest(ours, self.pieces(side_to_move, PieceKind::Knight)))
                .or_else(|| lowest(ours, self.pieces(side_to_move, PieceKind::Bishop)))
                .or_else(|| lowest(ours, self.pieces(side_to_move, PieceKind::Rook)))
                .or_else(|| lowest(ours, self.pieces(side_to_move, PieceKind::Queen)))
                .or_else(|| lowest(ours, self.pieces(side_to_move, PieceKind::King)))
            else {
                return net > threshold;
            };
            let kind = self
                .piece_on(capturer)
                .map_or(PieceKind::King, |piece| piece.kind);
            net = if on_square_owner == side {
                net - on_square_value
            } else {
                net + on_square_value
            };
            on_square_value = see_value(kind);
            on_square_owner = side_to_move;
            occupied &= !capturer.bit();
            if matches!(kind, PieceKind::Pawn | PieceKind::Bishop | PieceKind::Queen) {
                attackers.0 |=
                    piece_attacks(PieceKind::Bishop, Color::White, target, Bitboard(occupied)).0
                        & bishops.0;
            }
            if matches!(kind, PieceKind::Rook | PieceKind::Queen) {
                attackers.0 |=
                    piece_attacks(PieceKind::Rook, Color::White, target, Bitboard(occupied)).0
                        & rooks.0;
            }
            side_to_move = side_to_move.opposite();
        }
        net > threshold
    }
}

#[cfg(test)]
mod tests {
    use super::Board;
    use std::error::Error;

    fn gain(fen: &str, notation: &str, threshold: i32) -> Result<bool, Box<dyn Error>> {
        let board: Board = fen.parse()?;
        Ok(board.static_exchange_gain(board.resolve_uci(notation, false)?, threshold))
    }

    #[test]
    fn captures_are_weighted_by_material() -> Result<(), Box<dyn Error>> {
        assert!(gain("4k3/8/8/3p4/4P3/8/8/4K3 w - - 0 1", "e4d5", 99)?);
        assert!(!gain("4k3/8/8/3p4/4P3/8/8/4K3 w - - 0 1", "e4d5", 101)?);
        // A bare rook recapture makes the exchange even for white.
        assert!(!gain("4k3/3r4/8/3p4/4P3/8/8/4K3 w - - 0 1", "e4d5", 99)?);
        assert!(gain("4k3/3r4/8/3p4/4P3/8/8/4K3 w - - 0 1", "e4d5", -1)?);
        // A queen recapture answered by a rook leaves the rook surviving on d5.
        assert!(gain("4k3/3q4/8/3p4/4P3/8/3R4/4K3 w - - 0 1", "e4d5", 899)?);
        assert!(!gain("4k3/3q4/8/3p4/4P3/8/3R4/4K3 w - - 0 1", "e4d5", 901)?);
        // A knight recapture after the rook leaves the knight surviving.
        assert!(gain("4k3/3r4/8/3p4/4P3/2N5/8/4K3 w - - 0 1", "e4d5", 499)?);
        assert!(!gain("4k3/3r4/8/3p4/4P3/2N5/8/4K3 w - - 0 1", "e4d5", 501)?);
        assert!(gain("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1", "e5d6", 99)?);
        Ok(())
    }

    #[test]
    fn quiet_moves_hang_the_moving_piece() -> Result<(), Box<dyn Error>> {
        // A rook moving into a defended square hangs exactly its value.
        assert!(!gain("4k3/r7/8/8/8/8/8/R3K3 w - - 0 1", "a1a5", -499)?);
        assert!(gain("4k3/r7/8/8/8/8/8/R3K3 w - - 0 1", "a1a5", -501)?);
        Ok(())
    }
}
