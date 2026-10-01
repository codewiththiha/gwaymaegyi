//! Material, capture, clock, and dead-position queries.
//! Null positions are synthetic search transitions, never game-history moves.

use crate::{Board, Color, Move, MoveKind, PieceKind};

impl Board {
    #[must_use]
    pub const fn halfmove_clock(&self) -> u32 {
        self.halfmove
    }

    #[must_use]
    pub const fn fullmove_number(&self) -> u32 {
        self.fullmove
    }

    #[must_use]
    pub fn is_capture(&self, chess_move: Move) -> bool {
        chess_move.kind() == MoveKind::EnPassant
            || (chess_move.kind() != MoveKind::Castle && self.piece_on(chess_move.to()).is_some())
    }

    #[must_use]
    pub fn material(&self, color: Color) -> i32 {
        [
            PieceKind::Pawn,
            PieceKind::Knight,
            PieceKind::Bishop,
            PieceKind::Rook,
            PieceKind::Queen,
        ]
        .into_iter()
        .zip([100, 300, 300, 500, 900])
        .map(|(kind, value)| i32::try_from(self.pieces(color, kind).len()).unwrap_or(64) * value)
        .sum()
    }

    #[must_use]
    pub fn insufficient_material(&self) -> bool {
        if [PieceKind::Pawn, PieceKind::Rook, PieceKind::Queen]
            .into_iter()
            .any(|kind| !self.roles[kind.index()].is_empty())
        {
            return false;
        }
        let knights = self.roles[PieceKind::Knight.index()].len();
        let bishops = self.roles[PieceKind::Bishop.index()];
        if knights + bishops.len() <= 1 {
            return true;
        }
        if knights != 0 {
            return false;
        }
        let dark = bishops
            .into_iter()
            .filter(|square| (square.file() + square.rank()) % 2 == 0)
            .count();
        dark == 0 || dark == bishops.len() as usize
    }

    /// A synthetic transition for pruning; it is not a legal game move.
    #[must_use]
    pub fn null_position(&self) -> Option<Self> {
        if self.in_check(self.side) {
            return None;
        }
        let mut child = *self;
        child.side = self.side.opposite();
        child.en_passant = None;
        child
            .identity
            .toggle_state(self.state_key() ^ child.state_key());
        Some(child)
    }
}

impl Board {
    /// True when either side retains at least one rook-origin castling right.
    #[must_use]
    pub fn has_castling_rights(&self) -> bool {
        self.castling.rooks.iter().flatten().any(Option::is_some)
    }

    /// True when the given side retains at least one rook-origin castling right.
    #[must_use]
    pub fn has_castling_rights_for(&self, color: Color) -> bool {
        self.castling.rooks[color.index()]
            .iter()
            .any(Option::is_some)
    }

    /// Returns the en-passant square only when a legal capture is available.
    #[must_use]
    pub fn legal_en_passant_square(&self) -> Option<crate::Square> {
        self.has_legal_en_passant()
            .then_some(self.en_passant)
            .flatten()
    }
}
