//! Portable rules with no clock, filesystem, or thread dependencies.

mod attacks;
mod bitboard;
mod board;
mod castling;
mod chess_move;
mod error;
mod fen;
mod movegen;
mod perft;
mod play;
mod types;

pub use bitboard::Bitboard;
pub use board::{Board, START_FEN};
pub use chess_move::{Move, MoveKind, Promotion};
pub use error::{FenError, MoveError};
pub use perft::{divide, perft};
pub use types::{Color, Piece, PieceKind, Square};
