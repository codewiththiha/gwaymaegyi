use gwaymaegyi_core::{Color, Piece, Square};

use crate::HIDDEN;
const FEATURE_BYTES: usize = 768 * HIDDEN * 2;
const BIAS_BYTES: usize = HIDDEN * 2;
const OUTPUT_BYTES: usize = HIDDEN * 4;
const NET_BYTES: usize = FEATURE_BYTES + BIAS_BYTES + OUTPUT_BYTES + 2;

static BALANCED: &[u8; NET_BYTES] = include_bytes!("../../../assets/models/balanced.nnue");
static ENDGAME: &[u8; NET_BYTES] = include_bytes!("../../../assets/models/endgame.nnue");
static AGGRESSIVE: &[u8; NET_BYTES] = include_bytes!("../../../assets/models/aggressive.nnue");

/// Each model uses the same quantization and perspective convention.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Model {
    Balanced,
    Endgame,
    Aggressive,
}

impl Model {
    const fn data(self) -> &'static [u8; NET_BYTES] {
        match self {
            Self::Balanced => BALANCED,
            Self::Endgame => ENDGAME,
            Self::Aggressive => AGGRESSIVE,
        }
    }

    pub(super) fn feature(
        self,
        piece: Piece,
        square: Square,
        perspective: Color,
    ) -> impl ExactSizeIterator<Item = i16> {
        let black = usize::from(perspective == Color::Black);
        let color = (piece.color as usize) ^ black;
        let square = square.index() ^ (black * 56);
        let index = color * 384 + (piece.kind as usize) * 64 + square;
        let offset = index * HIDDEN * 2;
        weights(&self.data()[offset..offset + HIDDEN * 2])
    }

    pub(super) fn bias(self) -> impl ExactSizeIterator<Item = i16> {
        weights(&self.data()[FEATURE_BYTES..FEATURE_BYTES + BIAS_BYTES])
    }

    pub(super) fn output(self, ours: bool) -> impl ExactSizeIterator<Item = i16> {
        let offset = FEATURE_BYTES + BIAS_BYTES + usize::from(!ours) * HIDDEN * 2;
        weights(&self.data()[offset..offset + HIDDEN * 2])
    }

    pub(super) fn output_bias(self) -> i16 {
        let data = self.data();
        i16::from_le_bytes([data[NET_BYTES - 2], data[NET_BYTES - 1]])
    }
}

fn weights(data: &[u8]) -> impl ExactSizeIterator<Item = i16> {
    data.chunks_exact(2)
        .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
}
