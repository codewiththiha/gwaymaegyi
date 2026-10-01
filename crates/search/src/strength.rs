//! Validated skill presets and full/approximate strength values.
//! Nominal Elo labels are controls, not measured engine ratings.

#![expect(
    clippy::missing_errors_doc,
    reason = "Validation failures are documented in plain prose."
)]

use crate::EngineError;

/// Resource/error-tolerance selection; numeric targets remain uncalibrated.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Strength {
    #[default]
    Full,
    Approximate(u16),
}

/// Levels 1 through 20 select presets; level 21 selects unrestricted strength.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SkillLevel(u8);

impl Default for SkillLevel {
    fn default() -> Self {
        Self::FULL
    }
}

impl SkillLevel {
    pub const FULL: Self = Self(21);
    pub const NOMINAL_ELO: [u16; 20] = [
        500, 800, 1000, 1200, 1300, 1400, 1500, 1600, 1700, 1800, 1900, 2000, 2100, 2200, 2300,
        2400, 2500, 2650, 2800, 3000,
    ];

    /// Invalid levels are rejected before they can alter configuration.
    pub const fn new(level: u8) -> Result<Self, EngineError> {
        if level == 0 || level > 21 {
            Err(EngineError::InvalidSkill)
        } else {
            Ok(Self(level))
        }
    }

    #[must_use]
    pub const fn value(self) -> u8 {
        self.0
    }

    #[must_use]
    pub const fn strength(self) -> Strength {
        if self.0 == 21 {
            Strength::Full
        } else {
            Strength::Approximate(Self::NOMINAL_ELO[self.0 as usize - 1])
        }
    }

    #[must_use]
    pub const fn nominal_elo(self) -> Option<u16> {
        match self.strength() {
            Strength::Full => None,
            Strength::Approximate(elo) => Some(elo),
        }
    }

    pub(super) fn from_strength(strength: Strength) -> Option<Self> {
        match strength {
            Strength::Full => Some(Self::FULL),
            Strength::Approximate(elo) => Self::NOMINAL_ELO
                .iter()
                .position(|&value| value == elo)
                .and_then(|index| u8::try_from(index + 1).ok())
                .map(Self),
        }
    }
}
