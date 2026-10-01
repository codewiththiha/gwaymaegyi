use gwaymaegyi_core::Move;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Completion {
    Depth,
    Nodes,
    Stopped,
    Terminal,
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
}
