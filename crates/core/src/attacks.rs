use crate::{Bitboard, Board, Color, PieceKind, Square};

const KNIGHT: [(i8, i8); 8] = [(-2, -1), (-2, 1), (-1, -2), (-1, 2), (1, -2), (1, 2), (2, -1), (2, 1)];
const KING: [(i8, i8); 8] = [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)];
const DIAGONAL: [(i8, i8); 4] = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
const ORTHOGONAL: [(i8, i8); 4] = [(-1, 0), (0, -1), (0, 1), (1, 0)];

pub(crate) fn piece_attacks(kind: PieceKind, color: Color, square: Square, occupied: Bitboard) -> Bitboard {
    match kind {
        PieceKind::Pawn => steps(square, &[(-1, color.pawn_step()), (1, color.pawn_step())]),
        PieceKind::Knight => steps(square, &KNIGHT),
        PieceKind::King => steps(square, &KING),
        PieceKind::Bishop => rays(square, occupied, &DIAGONAL),
        PieceKind::Rook => rays(square, occupied, &ORTHOGONAL),
        PieceKind::Queen => Bitboard(rays(square, occupied, &DIAGONAL).0 | rays(square, occupied, &ORTHOGONAL).0),
    }
}

fn steps(square: Square, offsets: &[(i8, i8)]) -> Bitboard {
    Bitboard(offsets.iter().filter_map(|&(file, rank)| square.offset(file, rank)).fold(0, |bits, square| bits | square.bit()))
}

fn rays(square: Square, occupied: Bitboard, directions: &[(i8, i8)]) -> Bitboard {
    let mut bits = 0;
    for &(file, rank) in directions {
        let mut current = square;
        while let Some(next) = current.offset(file, rank) {
            bits |= next.bit();
            if occupied.contains(next) {
                break;
            }
            current = next;
        }
    }
    Bitboard(bits)
}

impl Board {
    pub(crate) fn is_attacked(&self, square: Square, by: Color, occupied: Bitboard) -> bool {
        let intersects = |kind, attack_color| {
            piece_attacks(kind, attack_color, square, occupied).0 & self.pieces(by, kind).0 != 0
        };
        intersects(PieceKind::Pawn, by.opposite())
            || intersects(PieceKind::Knight, by)
            || intersects(PieceKind::King, by)
            || piece_attacks(PieceKind::Bishop, by, square, occupied).0
                & (self.pieces(by, PieceKind::Bishop).0 | self.pieces(by, PieceKind::Queen).0) != 0
            || piece_attacks(PieceKind::Rook, by, square, occupied).0
                & (self.pieces(by, PieceKind::Rook).0 | self.pieces(by, PieceKind::Queen).0) != 0
    }
}
