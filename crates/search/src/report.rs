//! Completed analysis snapshots, stop reasons, and checked mate-distance formatting.

use gwaymaegyi_core::Move;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Completion {
    Depth,
    Nodes,
    Stopped,
    Terminal,
    Tablebase,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SearchStatus {
    #[default]
    Idle,
    Running,
    Finished(Completion),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrincipalVariation {
    pub score_cp: i32,
    pub moves: Vec<Move>,
}

/// Interrupted iterations never replace the last complete set of principal variations.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SearchReport {
    pub status: SearchStatus,
    pub depth: u8,
    pub selective_depth: u8,
    pub nodes: u64,
    pub best_move: Option<Move>,
    pub score_cp: Option<i32>,
    pub variations: Vec<PrincipalVariation>,
    /// Nodes spent on the best line's root move in the last completed iteration.
    pub best_move_nodes: u64,
    pub tablebase_hits: u64,
}

impl PrincipalVariation {
    #[must_use]
    pub fn mate_in(&self) -> Option<i32> {
        mate_in(self.score_cp)
    }
}
impl SearchReport {
    #[must_use]
    pub fn mate_in(&self) -> Option<i32> {
        self.score_cp.and_then(mate_in)
    }
}
fn mate_in(score: i32) -> Option<i32> {
    let magnitude = score.checked_abs()?;
    if !(crate::MATE_THRESHOLD..=crate::MATE).contains(&magnitude) {
        return None;
    }
    Some((crate::MATE - magnitude + 1) / 2 * score.signum())
}
