//! Portable chess types, rules, game history, and perft APIs.
//! This crate owns no clocks, files, threads, or search resources.

mod bitboard;
mod board;
mod chess_move;
mod error;
mod game;
mod key;
mod types;

pub use bitboard::{Bitboard, Squares};
pub use board::{Board, START_FEN, Successor, divide, perft};
pub use chess_move::{Move, MoveKind, Promotion};
pub use error::{FenError, MoveError};
pub use types::{Color, Piece, PieceKind, Square};

pub use game::{ClaimableDraw, DrawReason, Game, Outcome};
pub use key::PositionKey;
