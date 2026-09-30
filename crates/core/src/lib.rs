//! Portable rules with no clock, filesystem, or thread dependencies.

mod bitboard;
mod board;
mod chess_move;
mod error;
mod types;

pub use bitboard::{Bitboard, Squares};
pub use board::{Board, START_FEN, divide, perft};
pub use chess_move::{Move, MoveKind, Promotion};
pub use error::{FenError, MoveError};
pub use types::{Color, Piece, PieceKind, Square};
