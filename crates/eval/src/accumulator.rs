use crate::{HIDDEN, Model};
use gwaymaegyi_core::{Board, Color};

/// A snapshot owns its position, so delta updates cannot use a mismatched parent.
#[derive(Clone, Debug)]
pub struct Accumulator {
    board: Board,
    model: Model,
    sides: [[i16; HIDDEN]; 2],
}

impl Accumulator {
    #[must_use]
    pub fn new(board: &Board, model: Model) -> Self {
        let mut result = Self {
            board: *board,
            model,
            sides: [[0; HIDDEN]; 2],
        };
        for side in &mut result.sides {
            for (value, bias) in side.iter_mut().zip(model.bias()) {
                *value = bias;
            }
        }
        for square in board.occupied() {
            if let Some(piece) = board.piece_on(square) {
                result.add(piece, square);
            }
        }
        result
    }

    #[must_use]
    pub const fn model(&self) -> Model {
        self.model
    }

    #[must_use]
    pub fn updated(&self, board: &Board) -> Self {
        let mut child = self.clone();
        // Remove before adding so dense synthetic positions cannot saturate a temporary sum.
        for square in self.board.occupied() {
            if self.board.piece_on(square) != board.piece_on(square) {
                if let Some(piece) = self.board.piece_on(square) {
                    child.subtract(piece, square);
                }
            }
        }
        for square in board.occupied() {
            if self.board.piece_on(square) != board.piece_on(square) {
                if let Some(piece) = board.piece_on(square) {
                    child.add(piece, square);
                }
            }
        }
        child.board = *board;
        child
    }

    #[must_use]
    pub fn score(&self) -> i32 {
        self.score_for(self.board.side_to_move())
    }

    #[must_use]
    pub fn score_for(&self, color: Color) -> i32 {
        let us = &self.sides[color as usize];
        let them = &self.sides[color.opposite() as usize];
        let flatten = |side: &[i16; HIDDEN], ours| {
            side.iter()
                .zip(self.model.output(ours))
                .map(|(&value, weight)| {
                    let activation = i64::from(value.clamp(0, 255));
                    activation * activation * i64::from(weight)
                })
                .sum::<i64>()
        };
        let raw = flatten(us, true) + flatten(them, false);
        let score = (raw + i64::from(self.model.output_bias()) * 255) * 400 / (255 * 255 * 64);
        i32::try_from(score).unwrap_or(if score < 0 { i32::MIN } else { i32::MAX })
    }

    fn add(&mut self, piece: gwaymaegyi_core::Piece, square: gwaymaegyi_core::Square) {
        for color in [Color::White, Color::Black] {
            for (value, weight) in self.sides[color as usize]
                .iter_mut()
                .zip(self.model.feature(piece, square, color))
            {
                *value = value.saturating_add(weight);
            }
        }
    }

    fn subtract(&mut self, piece: gwaymaegyi_core::Piece, square: gwaymaegyi_core::Square) {
        for color in [Color::White, Color::Black] {
            for (value, weight) in self.sides[color as usize]
                .iter_mut()
                .zip(self.model.feature(piece, square, color))
            {
                *value = value.saturating_sub(weight);
            }
        }
    }
}
