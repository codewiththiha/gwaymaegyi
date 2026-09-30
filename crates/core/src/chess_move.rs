use crate::{PieceKind, Square};

/// Only these four roles can result from a promotion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Promotion {
    Knight,
    Bishop,
    Rook,
    Queen,
}

impl Promotion {
    pub(crate) const fn piece_kind(self) -> PieceKind {
        match self {
            Self::Knight => PieceKind::Knight,
            Self::Bishop => PieceKind::Bishop,
            Self::Rook => PieceKind::Rook,
            Self::Queen => PieceKind::Queen,
        }
    }

    pub(crate) const fn suffix(self) -> char {
        match self {
            Self::Knight => 'n',
            Self::Bishop => 'b',
            Self::Rook => 'r',
            Self::Queen => 'q',
        }
    }
}

/// Castling targets the friendly rook internally, including in standard chess.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MoveKind {
    Normal,
    EnPassant,
    Castle,
    Promotion(Promotion),
}

/// A move value; legality is checked against a position before external play.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Move {
    from: Square,
    to: Square,
    kind: MoveKind,
}

impl Move {
    pub(crate) const fn new(from: Square, to: Square, kind: MoveKind) -> Self {
        Self { from, to, kind }
    }

    #[must_use]
    pub const fn from(self) -> Square {
        self.from
    }

    #[must_use]
    pub const fn to(self) -> Square {
        self.to
    }

    #[must_use]
    pub const fn kind(self) -> MoveKind {
        self.kind
    }

    #[must_use]
    pub fn to_uci(self, chess960: bool) -> String {
        let target = if self.kind == MoveKind::Castle && !chess960 {
            Square::new(
                if self.to.file() > self.from.file() {
                    6
                } else {
                    2
                },
                self.from.rank(),
            )
            .unwrap_or(self.to)
        } else {
            self.to
        };
        let mut notation = format!("{}{target}", self.from);
        if let MoveKind::Promotion(promotion) = self.kind {
            notation.push(promotion.suffix());
        }
        notation
    }
}
