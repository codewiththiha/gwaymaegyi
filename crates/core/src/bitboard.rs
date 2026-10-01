//! Square sets and deterministic bit scans for attacks and move generation.

use crate::Square;

/// A set of squares; iteration consumes the least significant square first.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Bitboard(pub u64);

impl Bitboard {
    #[must_use]
    pub const fn contains(self, square: Square) -> bool {
        self.0 & square.bit() != 0
    }

    #[must_use]
    pub const fn len(self) -> u32 {
        self.0.count_ones()
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for Bitboard {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for Bitboard {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

/// Iteration state is separate from the copyable square set.
#[derive(Clone, Debug)]
pub struct Squares {
    remaining: Bitboard,
}

impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = Squares;

    fn into_iter(self) -> Self::IntoIter {
        Squares { remaining: self }
    }
}

impl Iterator for Squares {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining.is_empty() {
            return None;
        }
        let index = u8::try_from(self.remaining.0.trailing_zeros()).ok()?;
        self.remaining.0 &= self.remaining.0 - 1;
        Square::from_index(index)
    }
}
