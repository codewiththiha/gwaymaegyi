//! Legal game history, draw claims, and automatic outcomes.
//! Rejected moves leave both the board and repetition history intact.

#![expect(
    clippy::missing_errors_doc,
    reason = "Errors are described in plain prose."
)]

use crate::{Board, Color, FenError, MoveError, START_FEN};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DrawReason {
    Stalemate,
    InsufficientMaterial,
    FivefoldRepetition,
    SeventyFiveMoves,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaimableDraw {
    ThreefoldRepetition,
    FiftyMoves,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Ongoing,
    Checkmate { winner: Color },
    Draw(DrawReason),
}

/// A legal game owns its repetition history; search-only transitions never enter it.
#[derive(Clone, Debug)]
pub struct Game {
    board: Board,
    positions: Vec<u64>,
    material_balances: Vec<i32>,
}

impl Game {
    #[must_use]
    pub fn new(board: Board) -> Self {
        let side = board.side_to_move();
        let balance = board.material(side) - board.material(side.opposite());
        Self {
            board,
            positions: vec![board.key().full()],
            material_balances: vec![balance],
        }
    }

    /// A malformed position returns its structural FEN error.
    pub fn start() -> Result<Self, FenError> {
        START_FEN.parse().map(Self::new)
    }

    #[must_use]
    pub const fn board(&self) -> &Board {
        &self.board
    }

    #[must_use]
    pub fn position_history(&self) -> &[u64] {
        &self.positions
    }

    #[must_use]
    pub fn material_history(&self) -> &[i32] {
        &self.material_balances
    }

    /// Rejected moves leave both the position and its history unchanged.
    pub fn play_uci(&mut self, notation: &str, chess960: bool) -> Result<(), MoveError> {
        if self.outcome() != Outcome::Ongoing {
            return Err(MoveError::GameOver);
        }
        let board = self.board.play_uci(notation, chess960)?;
        if board.halfmove_clock() == 0 {
            self.positions.clear();
        }
        self.positions.push(board.key().full());
        let side = board.side_to_move();
        self.material_balances
            .push(board.material(side) - board.material(side.opposite()));
        self.board = board;
        Ok(())
    }

    #[must_use]
    pub fn repetitions(&self) -> usize {
        self.positions
            .iter()
            .filter(|&&key| key == self.board.key().full())
            .count()
    }

    #[must_use]
    pub fn outcome(&self) -> Outcome {
        if self.board.legal_moves().is_empty() {
            return if self.board.in_check(self.board.side_to_move()) {
                Outcome::Checkmate {
                    winner: self.board.side_to_move().opposite(),
                }
            } else {
                Outcome::Draw(DrawReason::Stalemate)
            };
        }
        if self.board.insufficient_material() {
            return Outcome::Draw(DrawReason::InsufficientMaterial);
        }
        if self.board.halfmove_clock() >= 150 {
            return Outcome::Draw(DrawReason::SeventyFiveMoves);
        }
        if self.repetitions() >= 5 {
            return Outcome::Draw(DrawReason::FivefoldRepetition);
        }
        Outcome::Ongoing
    }

    #[must_use]
    pub fn claimable_draws(&self) -> Vec<ClaimableDraw> {
        if self.outcome() != Outcome::Ongoing {
            return Vec::new();
        }
        let mut claims = Vec::new();
        if self.repetitions() >= 3 {
            claims.push(ClaimableDraw::ThreefoldRepetition);
        }
        if self.board.halfmove_clock() >= 100 {
            claims.push(ClaimableDraw::FiftyMoves);
        }
        claims
    }
}
