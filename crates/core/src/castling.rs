use crate::{Bitboard, Board, Color, Move, MoveKind, Piece, PieceKind, Square};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CastlingRights {
    pub(super) rooks: [[Option<Square>; 2]; 2],
}

impl CastlingRights {
    pub(super) const fn empty() -> Self {
        Self {
            rooks: [[None; 2]; 2],
        }
    }

    pub(super) fn remove_at(&mut self, square: Square) {
        for rights in &mut self.rooks {
            for rook in rights {
                if *rook == Some(square) {
                    *rook = None;
                }
            }
        }
    }
}

pub(super) fn destinations(king: Square, rook: Square) -> Option<(Square, Square)> {
    let kingside = rook.file() > king.file();
    let king_to = Square::new(if kingside { 6 } else { 2 }, king.rank())?;
    let rook_to = Square::new(if kingside { 5 } else { 3 }, king.rank())?;
    Some((king_to, rook_to))
}

fn rank_path(from: Square, to: Square) -> Bitboard {
    let mut bits = 0;
    for file in from.file().min(to.file())..=from.file().max(to.file()) {
        if let Some(square) = Square::new(file, from.rank()) {
            bits |= square.bit();
        }
    }
    Bitboard(bits)
}

pub(super) fn generate(board: &Board, moves: &mut Vec<Move>) {
    let color = board.side;
    let Some(king) = board.king(color) else {
        return;
    };
    if king.rank() != color.home_rank() || board.in_check(color) {
        return;
    }
    for rook in board.castling.rooks[color.index()].into_iter().flatten() {
        if board.piece_on(rook) != Some(Piece::new(color, PieceKind::Rook)) {
            continue;
        }
        let Some((king_to, rook_to)) = destinations(king, rook) else {
            continue;
        };
        let cleared = Bitboard(board.occupied().0 & !king.bit() & !rook.bit());
        let paths = rank_path(king, king_to).0 | rank_path(rook, rook_to).0;
        if cleared.0 & paths != 0 {
            continue;
        }
        // Intermediate attacks retain the rook; final legality uses the completed position.
        let intermediate = Bitboard(rank_path(king, king_to).0 & !king_to.bit());
        let transit_occupied = Bitboard(board.occupied().0 & !king.bit());
        if intermediate
            .into_iter()
            .any(|square| board.is_attacked(square, color.opposite(), transit_occupied))
        {
            continue;
        }
        moves.push(Move::new(king, rook, MoveKind::Castle));
    }
}

pub(super) fn rook_for_symbol(board: &Board, color: Color, kingside: bool) -> Option<Square> {
    let king = board.king(color)?;
    let rooks = board
        .pieces(color, PieceKind::Rook)
        .filter(|rook| rook.rank() == color.home_rank() && (rook.file() > king.file()) == kingside);
    if kingside { rooks.max() } else { rooks.min() }
}
