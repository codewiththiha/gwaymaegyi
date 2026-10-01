//! Validated squares, piece roles, colors, and their display/parsing conventions.

use std::{fmt, str::FromStr};

/// A piece's owner and the side to move.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Color {
    White,
    Black,
}

impl Color {
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }

    pub(crate) const fn index(self) -> usize {
        self as usize
    }

    pub(crate) const fn pawn_step(self) -> i8 {
        match self {
            Self::White => 1,
            Self::Black => -1,
        }
    }

    pub(crate) const fn home_rank(self) -> u8 {
        match self {
            Self::White => 0,
            Self::Black => 7,
        }
    }
}

/// Piece roles, independent of ownership.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PieceKind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl PieceKind {
    pub(crate) const fn index(self) -> usize {
        self as usize
    }
}

/// A colored piece; empty squares are represented by None.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Piece {
    pub color: Color,
    pub kind: PieceKind,
}

impl Piece {
    #[must_use]
    pub const fn new(color: Color, kind: PieceKind) -> Self {
        Self { color, kind }
    }
}

/// A validated square with a1 at index zero.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Square(u8);

impl Square {
    #[must_use]
    pub const fn from_index(index: u8) -> Option<Self> {
        if index < 64 { Some(Self(index)) } else { None }
    }

    #[must_use]
    pub const fn new(file: u8, rank: u8) -> Option<Self> {
        if file < 8 && rank < 8 {
            Some(Self(rank * 8 + file))
        } else {
            None
        }
    }

    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    #[must_use]
    pub const fn file(self) -> u8 {
        self.0 % 8
    }

    #[must_use]
    pub const fn rank(self) -> u8 {
        self.0 / 8
    }

    pub(crate) const fn bit(self) -> u64 {
        1 << self.0
    }

    pub(crate) fn offset(self, file_delta: i8, rank_delta: i8) -> Option<Self> {
        let file = self.file().checked_add_signed(file_delta)?;
        let rank = self.rank().checked_add_signed(rank_delta)?;
        Self::new(file, rank)
    }
}

impl fmt::Display for Square {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}{}",
            char::from(b'a' + self.file()),
            self.rank() + 1
        )
    }
}

impl FromStr for Square {
    type Err = crate::MoveError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let [file, rank] = *value.as_bytes() else {
            return Err(crate::MoveError::InvalidNotation);
        };
        let file = file.checked_sub(b'a');
        let rank = rank.checked_sub(b'1');
        file.zip(rank)
            .and_then(|(file, rank)| Self::new(file, rank))
            .ok_or(crate::MoveError::InvalidNotation)
    }
}
