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

impl Iterator for Bitboard {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_empty() {
            return None;
        }
        let index = u8::try_from(self.0.trailing_zeros()).ok()?;
        self.0 &= self.0 - 1;
        Square::from_index(index)
    }
}
