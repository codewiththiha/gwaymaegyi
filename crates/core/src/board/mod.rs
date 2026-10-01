mod attacks;
mod castling;
mod fen;
mod identity;
mod movegen;
mod perft;
mod play;
mod query;

use crate::{Bitboard, Color, Move, Piece, PieceKind, Square};
use castling::CastlingRights;

pub use perft::{divide, perft};

pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

/// Mailbox and bitboard views are changed together through place and remove.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Board {
    mailbox: [Option<Piece>; 64],
    roles: [Bitboard; 6],
    colors: [Bitboard; 2],
    side: Color,
    castling: CastlingRights,
    en_passant: Option<Square>,
    halfmove: u32,
    fullmove: u32,
    identity: crate::PositionKey,
}

impl Board {
    const fn empty() -> Self {
        Self {
            mailbox: [None; 64],
            roles: [Bitboard(0); 6],
            colors: [Bitboard(0); 2],
            side: Color::White,
            castling: CastlingRights::empty(),
            en_passant: None,
            halfmove: 0,
            fullmove: 1,
            identity: crate::PositionKey::empty(),
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
        self.legal_successors()
            .into_iter()
            .map(Successor::chess_move)
            .collect()
    }

    fn king(&self, color: Color) -> Option<Square> {
        self.pieces(color, PieceKind::King).into_iter().next()
    }

    const fn place(&mut self, square: Square, piece: Piece) {
        self.remove(square);
        self.mailbox[square.index()] = Some(piece);
        self.identity.toggle(piece, square);
        self.roles[piece.kind.index()].0 |= square.bit();
        self.colors[piece.color.index()].0 |= square.bit();
    }

    const fn remove(&mut self, square: Square) {
        if let Some(piece) = self.mailbox[square.index()].take() {
            self.identity.toggle(piece, square);
            self.roles[piece.kind.index()].0 &= !square.bit();
            self.colors[piece.color.index()].0 &= !square.bit();
        }
    }
}

/// A legal transition produced by the position that owns it.
#[derive(Clone, Copy, Debug)]
pub struct Successor {
    chess_move: Move,
    board: Board,
}

impl Successor {
    #[must_use]
    pub const fn chess_move(self) -> Move {
        self.chess_move
    }

    #[must_use]
    pub const fn board(&self) -> &Board {
        &self.board
    }
}

impl Board {
    #[must_use]
    pub fn legal_successors(&self) -> Vec<Successor> {
        movegen::legal_successors(self)
    }
}
