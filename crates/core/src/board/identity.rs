use crate::{Board, Color, Piece, PieceKind, PositionKey};

impl Board {
    #[must_use]
    pub const fn key(&self) -> PositionKey {
        self.identity
    }

    #[must_use]
    pub fn recomputed_key(&self) -> PositionKey {
        let mut key = PositionKey::empty();
        for square in self.occupied() {
            if let Some(piece) = self.piece_on(square) {
                key.toggle(piece, square);
            }
        }
        key.toggle_state(self.state_key());
        key
    }

    pub(super) fn state_key(&self) -> u64 {
        let mut state = if self.side == Color::Black {
            PositionKey::state_component(768)
        } else {
            0
        };
        for (color, rights) in self.castling.rooks.iter().enumerate() {
            for (side, rook) in rights.iter().enumerate() {
                if let Some(rook) = rook {
                    state ^= PositionKey::state_component(
                        (769 + color * 16 + side * 8 + usize::from(rook.file())) as u64,
                    );
                }
            }
        }
        if self.has_legal_en_passant() {
            if let Some(square) = self.en_passant {
                state ^= PositionKey::state_component(801 + u64::from(square.file()));
            }
        }
        state
    }

    fn has_legal_en_passant(&self) -> bool {
        let Some(target) = self.en_passant else {
            return false;
        };
        let Some(captured) = target.offset(0, -self.side.pawn_step()) else {
            return false;
        };
        if self.piece_on(captured) != Some(Piece::new(self.side.opposite(), PieceKind::Pawn)) {
            return false;
        }
        [-1, 1]
            .into_iter()
            .filter_map(|file| target.offset(file, -self.side.pawn_step()))
            .any(|from| {
                if self.piece_on(from) != Some(Piece::new(self.side, PieceKind::Pawn)) {
                    return false;
                }
                let mut child = *self;
                child.remove(from);
                child.remove(captured);
                child.place(target, Piece::new(self.side, PieceKind::Pawn));
                !child.in_check(self.side)
            })
    }
}
