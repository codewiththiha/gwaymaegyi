#![expect(
    clippy::missing_errors_doc,
    reason = "Failure details use plain prose rather than Markdown sections."
)]

use super::castling::destinations;
use crate::{Board, Move, MoveError, MoveKind, Piece, PieceKind};

impl Board {
    /// Invalid notation and moves outside the legal list return distinct errors.
    /// Castling notation follows the requested standard or Chess960 convention.
    pub fn play_uci(&self, notation: &str, chess960: bool) -> Result<Self, MoveError> {
        if !matches!(notation.len(), 4 | 5) || !notation.is_ascii() {
            return Err(MoveError::InvalidNotation);
        }
        let chess_move = self
            .legal_moves()
            .into_iter()
            .find(|chess_move| chess_move.to_uci(chess960) == notation)
            .ok_or(MoveError::IllegalMove)?;
        self.after_generated_move(chess_move)
            .ok_or(MoveError::IllegalMove)
    }

    pub(super) fn after_generated_move(&self, chess_move: Move) -> Option<Self> {
        let from = chess_move.from();
        let to = chess_move.to();
        let piece = self.piece_on(from)?;
        let mut child = *self;
        let old_state = self.state_key();
        let is_capture = self.piece_on(to).is_some() && chess_move.kind() != MoveKind::Castle;
        child.en_passant = None;
        child.castling.remove_at(from);
        child.castling.remove_at(to);
        if piece.kind == PieceKind::King {
            child.castling.rooks[piece.color.index()] = [None; 2];
        }
        child.remove(from);
        match chess_move.kind() {
            MoveKind::Castle => {
                let (king_to, rook_to) = destinations(from, to)?;
                child.remove(to);
                child.place(king_to, piece);
                child.place(rook_to, Piece::new(piece.color, PieceKind::Rook));
            }
            MoveKind::EnPassant => {
                child.remove(to.offset(0, -piece.color.pawn_step())?);
                child.place(to, piece);
            }
            MoveKind::Promotion(promotion) => {
                child.place(to, Piece::new(piece.color, promotion.piece_kind()));
            }
            MoveKind::Normal => {
                child.place(to, piece);
                if piece.kind == PieceKind::Pawn && from.rank().abs_diff(to.rank()) == 2 {
                    child.en_passant = from.offset(0, piece.color.pawn_step());
                }
            }
        }
        child.halfmove = if piece.kind == PieceKind::Pawn || is_capture {
            0
        } else {
            self.halfmove.saturating_add(1)
        };
        if self.side == crate::Color::Black {
            child.fullmove = child.fullmove.saturating_add(1);
        }
        child.side = self.side.opposite();
        child.identity.toggle_state(old_state ^ child.state_key());
        Some(child)
    }
}
