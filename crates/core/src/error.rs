//! Typed FEN, square, and move failures shared by all adapters.

use std::{error::Error, fmt};

use crate::Color;

/// Structural FEN failures are distinct from whether a position is reachable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FenError {
    FieldCount,
    Placement,
    Kings(Color),
    SideToMove,
    Castling,
    EnPassant,
    HalfmoveClock,
    FullmoveNumber,
}

impl fmt::Display for FenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FieldCount => formatter.write_str("FEN must contain six fields"),
            Self::Placement => formatter.write_str("invalid FEN piece placement"),
            Self::Kings(color) => write!(formatter, "FEN must contain one {color:?} king"),
            Self::SideToMove => formatter.write_str("invalid FEN side to move"),
            Self::Castling => formatter.write_str("invalid FEN castling rights"),
            Self::EnPassant => formatter.write_str("invalid FEN en passant target"),
            Self::HalfmoveClock => formatter.write_str("invalid FEN halfmove clock"),
            Self::FullmoveNumber => formatter.write_str("invalid FEN fullmove number"),
        }
    }
}

impl Error for FenError {}

/// External moves cannot bypass legality or notation checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MoveError {
    GameOver,
    InvalidNotation,
    IllegalMove,
}

impl fmt::Display for MoveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::GameOver => "game is already over",
            Self::InvalidNotation => "invalid UCI move notation",
            Self::IllegalMove => "move is not legal in this position",
        })
    }
}

impl Error for MoveError {}
