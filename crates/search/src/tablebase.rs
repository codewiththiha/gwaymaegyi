//! Portable WDL probing contract; filesystem discovery stays in native adapters.

use gwaymaegyi_core::{Board, Move};

/// Syzygy win/draw/loss result from the side-to-move perspective.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TablebaseWdl {
    Loss,
    BlessedLoss,
    Draw,
    CursedWin,
    Win,
}

/// Ranked root result with distance-to-zero when supplied by the backend.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TablebaseRoot {
    pub best_move: Move,
    pub wdl: TablebaseWdl,
    pub dtz: u16,
}

/// Optional, thread-safe tablebase source. Unsupported positions return `None`.
pub trait TablebaseProbe: core::fmt::Debug + Send + Sync {
    fn max_pieces(&self) -> u32;
    fn probe_wdl(&self, board: &Board) -> Option<TablebaseWdl>;

    /// Optional rule-50-aware root ranking. Portable providers may leave this unsupported.
    fn probe_root(&self, _board: &Board, _allowed: &[Move]) -> Option<TablebaseRoot> {
        None
    }
}
