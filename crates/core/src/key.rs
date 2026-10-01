use crate::{Color, Piece, PieceKind, Square};

/// Full identity and the independent material subsets used by evaluation history.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PositionKey {
    full: u64,
    pawns: u64,
    non_pawns: [u64; 2],
}

impl PositionKey {
    pub(crate) const fn empty() -> Self {
        Self {
            full: 0,
            pawns: 0,
            non_pawns: [0; 2],
        }
    }

    #[must_use]
    pub const fn full(self) -> u64 {
        self.full
    }

    #[must_use]
    pub const fn pawns(self) -> u64 {
        self.pawns
    }

    #[must_use]
    pub const fn non_pawns(self, color: Color) -> u64 {
        self.non_pawns[color.index()]
    }

    pub(crate) const fn toggle(&mut self, piece: Piece, square: Square) {
        let index = piece.kind.index() * 128 + piece.color.index() * 64 + square.index();
        let value = random(index as u64);
        self.full ^= value;
        if matches!(piece.kind, PieceKind::Pawn) {
            self.pawns ^= value;
        } else {
            self.non_pawns[piece.color.index()] ^= value;
        }
    }

    pub(crate) const fn toggle_state(&mut self, state: u64) {
        self.full ^= state;
    }
}

pub(super) const fn random(index: u64) -> u64 {
    let mut value = 0x9e37_79b9_7f4a_7c15_u64.wrapping_mul(index.wrapping_add(1));
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
