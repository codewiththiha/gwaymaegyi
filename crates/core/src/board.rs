use crate::{Bitboard, Color, Move, Piece, PieceKind, Square, castling::CastlingRights};

pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

/// Mailbox and bitboard views are changed together through place and remove.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Board {
    pub(crate) mailbox: [Option<Piece>; 64],
    pub(crate) roles: [Bitboard; 6],
    pub(crate) colors: [Bitboard; 2],
    pub(crate) side: Color,
    pub(crate) castling: CastlingRights,
    pub(crate) en_passant: Option<Square>,
    pub(crate) halfmove: u32,
    pub(crate) fullmove: u32,
}

impl Board {
    pub(crate) const fn empty() -> Self {
        Self {
            mailbox: [None; 64],
            roles: [Bitboard(0); 6],
            colors: [Bitboard(0); 2],
            side: Color::White,
            castling: CastlingRights::empty(),
            en_passant: None,
            halfmove: 0,
            fullmove: 1,
        }
    }

    #[must_use]
    pub const fn side_to_move(&self) -> Color {
        self.side
    }

    #[must_use]
    pub const fn piece_on(&self, square: Square) -> Option<Piece> {
        self.mailbox[square.index()]
    }

    #[must_use]
    pub const fn pieces(&self, color: Color, kind: PieceKind) -> Bitboard {
        Bitboard(self.colors[color.index()].0 & self.roles[kind.index()].0)
    }

    #[must_use]
    pub const fn occupied(&self) -> Bitboard {
        Bitboard(self.colors[0].0 | self.colors[1].0)
    }

    #[must_use]
    pub fn in_check(&self, color: Color) -> bool {
        self.king(color)
            .is_none_or(|king| self.is_attacked(king, color.opposite(), self.occupied()))
    }

    #[must_use]
    pub fn legal_moves(&self) -> Vec<Move> {
        crate::movegen::legal_moves(self)
    }

    pub(crate) fn king(&self, color: Color) -> Option<Square> {
        self.pieces(color, PieceKind::King).into_iter().next()
    }

    pub(crate) const fn place(&mut self, square: Square, piece: Piece) {
        self.remove(square);
        self.mailbox[square.index()] = Some(piece);
        self.roles[piece.kind.index()].0 |= square.bit();
        self.colors[piece.color.index()].0 |= square.bit();
    }

    pub(crate) const fn remove(&mut self, square: Square) {
        if let Some(piece) = self.mailbox[square.index()].take() {
            self.roles[piece.kind.index()].0 &= !square.bit();
            self.colors[piece.color.index()].0 &= !square.bit();
        }
    }
}
