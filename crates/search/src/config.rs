//! Validated playing policies and search budgets, separate from host scheduling.

#![expect(
    clippy::missing_errors_doc,
    reason = "Validation failures are documented in plain prose."
)]

use crate::{EngineError, MAX_DEPTH, SearchTuning, SkillLevel, Strength};
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Options {
    mode: Mode,
    strength: Strength,
    hash_mib: u16,
    multi_pv: u8,
    chess960: bool,
    seed: u64,
    tuning: SearchTuning,
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
            tuning: SearchTuning::default(),
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

    #[must_use]
    pub fn skill_level(self) -> Option<SkillLevel> {
        SkillLevel::from_strength(self.strength)
    }

    pub const fn set_skill_level(&mut self, level: SkillLevel) {
        self.strength = level.strength();
    }

    #[must_use]
    pub const fn tuning(self) -> SearchTuning {
        self.tuning
    }
    pub const fn set_tuning(&mut self, tuning: SearchTuning) {
        self.tuning = tuning;
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

    pub(super) fn evaluate(
        self,
        root: &gwaymaegyi_core::Board,
        state: &gwaymaegyi_eval::Accumulator,
        board: &gwaymaegyi_core::Board,
    ) -> i32 {
        self.evaluate_with_sacrifice(root, state, board, 0)
    }

    pub(super) fn evaluate_with_sacrifice(
        self,
        root: &gwaymaegyi_core::Board,
        state: &gwaymaegyi_eval::Accumulator,
        board: &gwaymaegyi_core::Board,
        sacrifice: i32,
    ) -> i32 {
        let raw = state.score().clamp(-40_000, 40_000);
        let mut score = raw * 100 / 195;
        if matches!(self.mode(), Mode::Aggressive | Mode::Human) {
            let color = root.side_to_move();
            let our_side = board.side_to_move() == color;
            let total = board.material(color) + board.material(color.opposite());
            let lost = root.material(color) - board.material(color);
            let enemy_lost = root.material(color.opposite()) - board.material(color.opposite());
            let mut bonus = 0;
            if sacrifice < 0 && total > 4500 {
                let divisor = if sacrifice < -300 {
                    5
                } else if sacrifice < -100 {
                    10
                } else {
                    20
                };
                let tier = if our_side {
                    if score > 500 {
                        2
                    } else {
                        i32::from(score > 0)
                    }
                } else if score < -500 {
                    -2
                } else {
                    -i32::from(score < 0)
                };
                bonus = 50 * tier * 10 / divisor;
            }
            let root_ahead = (our_side && score > 0) || (!our_side && score < 0);
            let root_queenless = board
                .pieces(color, gwaymaegyi_core::PieceKind::Queen)
                .is_empty();
            let mut scale = 750 + total / 25;
            if root_queenless || (total < 4000 && root_ahead) {
                scale -= 102;
            }
            score = score * scale / 1024 + bonus;
            if sacrifice == 0 && total > 4500 && lost > enemy_lost + 100 {
                let favorable = if our_side { score > 0 } else { score < 0 };
                if favorable {
                    score += if our_side { 30 } else { -30 };
                }
            }
        }
        score = score * (200 - i32::try_from(board.halfmove_clock().min(100)).unwrap_or(100)) / 200;
        score.clamp(-28_000, 28_000)
    }

    pub(super) fn detect_sacrifice(material_history: &[i32], current_balance: i32) -> i32 {
        if material_history.len() < 6 {
            return 0;
        }
        let last = material_history.len() - 1;
        let mut index = if (last & 1) == 0 { 2 } else { 1 };
        while index + 4 < material_history.len() {
            if material_history[index] < 0
                && material_history[index + 1] > 0
                && material_history[index + 2] < 0
                && material_history[index + 3] > 0
                && material_history[index + 4] < 0
            {
                return material_history[index + 4];
            }
            if material_history[index] < 0 && material_history[index] == current_balance {
                return material_history[index];
            }
            index += 2;
        }
        0
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
    /// Reference mistake budget per move: 120 minus a quarter centipawn per Elo.
    #[must_use]
    pub(super) fn cp_loss(self) -> i32 {
        match self.strength {
            Strength::Approximate(elo) => (120 - i32::from(elo) / 25).max(0),
            Strength::Full => 0,
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
    /// Maximum supported depth and node budget, still subject to the selected strength policy.
    #[must_use]
    pub const fn full() -> Self {
        Self {
            depth: MAX_DEPTH,
            nodes: u64::MAX,
        }
    }

    /// Invalid limits are rejected before replacing an active search.
    pub const fn validate(self) -> Result<Self, EngineError> {
        if self.depth == 0 || self.depth > MAX_DEPTH || self.nodes == 0 {
            Err(EngineError::InvalidLimits)
        } else {
            Ok(self)
        }
    }
}
