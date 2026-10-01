#![expect(
    clippy::missing_errors_doc,
    reason = "Validation failures are documented in plain prose."
)]

use crate::{EngineError, MAX_DEPTH};
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Mode {
    #[default]
    Balanced,
    Aggressive,
    Human,
    Analysis,
}

impl Mode {
    pub const ALL: [Self; 4] = [
        Self::Balanced,
        Self::Aggressive,
        Self::Human,
        Self::Analysis,
    ];
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Balanced => "balanced",
            Self::Aggressive => "aggressive",
            Self::Human => "human-like",
            Self::Analysis => "analysis",
        }
    }
}
impl FromStr for Mode {
    type Err = EngineError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.as_str() == value)
            .ok_or(EngineError::InvalidMode)
    }
}

/// These are resource/error-tolerance presets, not calibrated playing ratings.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Strength {
    #[default]
    Full,
    Approximate(u16),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Options {
    mode: Mode,
    strength: Strength,
    hash_mib: u16,
    multi_pv: u8,
    chess960: bool,
    seed: u64,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            mode: Mode::Balanced,
            strength: Strength::Full,
            hash_mib: 8,
            multi_pv: 1,
            chess960: false,
            seed: 19,
        }
    }
}
impl Options {
    #[must_use]
    pub const fn mode(self) -> Mode {
        self.mode
    }
    #[must_use]
    pub const fn strength(self) -> Strength {
        self.strength
    }
    #[must_use]
    pub const fn hash_mib(self) -> u16 {
        self.hash_mib
    }
    #[must_use]
    pub const fn multi_pv(self) -> u8 {
        self.multi_pv
    }
    #[must_use]
    pub const fn chess960(self) -> bool {
        self.chess960
    }
    #[must_use]
    pub const fn seed(self) -> u64 {
        self.seed
    }
    pub const fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }
    pub const fn set_chess960(&mut self, enabled: bool) {
        self.chess960 = enabled;
    }
    pub const fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
    }

    /// Zero selects full strength; other supported values select uncalibrated presets.
    pub const fn set_elo(&mut self, elo: u16) -> Result<(), EngineError> {
        self.strength = match elo {
            0 => Strength::Full,
            500..=3000 => Strength::Approximate(elo),
            _ => return Err(EngineError::InvalidElo),
        };
        Ok(())
    }
    /// An invalid size leaves the existing value unchanged.
    pub fn set_hash_mib(&mut self, size: u16) -> Result<(), EngineError> {
        if !(1..=64).contains(&size) {
            return Err(EngineError::InvalidHash);
        }
        self.hash_mib = size;
        Ok(())
    }
    /// An invalid count leaves the existing value unchanged.
    pub fn set_multi_pv(&mut self, count: u8) -> Result<(), EngineError> {
        if !(1..=5).contains(&count) {
            return Err(EngineError::InvalidMultiPv);
        }
        self.multi_pv = count;
        Ok(())
    }

    pub(super) fn model(self, board: &gwaymaegyi_core::Board) -> gwaymaegyi_eval::Model {
        let total = board.material(gwaymaegyi_core::Color::White)
            + board.material(gwaymaegyi_core::Color::Black);
        if total < 3500 {
            gwaymaegyi_eval::Model::Endgame
        } else if self.mode == Mode::Aggressive {
            gwaymaegyi_eval::Model::Aggressive
        } else {
            gwaymaegyi_eval::Model::Balanced
        }
    }

    pub(super) const fn effective_pv(self) -> u8 {
        if !matches!(self.strength, Strength::Full) && !matches!(self.mode, Mode::Analysis) {
            5
        } else {
            self.multi_pv
        }
    }
    pub(super) fn limit(self, mut limits: SearchLimits) -> SearchLimits {
        if self.mode != Mode::Analysis {
            if let Strength::Approximate(elo) = self.strength {
                let bucket = u32::from((elo - 500) / 125);
                limits.nodes = limits.nodes.min(256_u64 << bucket.min(13));
                limits.depth = limits
                    .depth
                    .min(u8::try_from(1 + bucket / 2).unwrap_or(MAX_DEPTH));
            }
        }
        limits
    }
    pub(super) fn loss_tolerance(self) -> i32 {
        match self.strength {
            Strength::Full => 0,
            Strength::Approximate(elo) => {
                i32::from(3000 - elo) / 15 + if self.mode == Mode::Human { 20 } else { 0 }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SearchLimits {
    pub depth: u8,
    pub nodes: u64,
}
impl Default for SearchLimits {
    fn default() -> Self {
        Self {
            depth: 8,
            nodes: 100_000,
        }
    }
}
impl SearchLimits {
    /// Invalid limits are rejected before replacing an active search.
    pub const fn validate(self) -> Result<Self, EngineError> {
        if self.depth == 0 || self.depth > MAX_DEPTH || self.nodes == 0 {
            Err(EngineError::InvalidLimits)
        } else {
            Ok(self)
        }
    }
}
