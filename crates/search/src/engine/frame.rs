//! Owned node windows, pending probes, and explicit continuation states.

use super::table::Bound;
use crate::INFINITY;
use gwaymaegyi_core::{Board, Move, Successor};
use gwaymaegyi_eval::Accumulator;

#[derive(Clone, Copy, Debug)]
pub(super) enum Probe {
    Full,
    Scout,
    Reduced,
    Null,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Pending {
    pub index: usize,
    pub depth: i16,
    pub probe: Probe,
}
#[derive(Debug)]
pub(super) struct NodeResult {
    pub score: i32,
    pub pv: Vec<Move>,
}
#[derive(Debug)]
pub(super) enum Stage {
    Enter,
    Moves,
    Waiting(Pending),
    Returned(Pending, NodeResult),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CachePolicy {
    Write,
    Skip,
}

#[derive(Debug)]
pub(super) struct Frame {
    pub board: Board,
    pub accumulator: Accumulator,
    pub depth: i16,
    pub ply: u8,
    pub alpha: i32,
    pub beta: i32,
    pub original_alpha: i32,
    pub pv_node: bool,
    pub synthetic: bool,
    pub key: u64,
    pub context: u64,
    pub in_check: bool,
    pub evaluation: i32,
    pub best: i32,
    pub best_move: Option<Move>,
    pub pv: Vec<Move>,
    pub candidates: Vec<Successor>,
    pub next: usize,
    pub stage: Stage,
    pub cache_policy: CachePolicy,
}

impl Frame {
    #[expect(
        clippy::too_many_arguments,
        reason = "A node explicitly carries its window and portable path state."
    )]
    pub(super) fn new(
        board: Board,
        accumulator: Accumulator,
        depth: i16,
        ply: u8,
        window: [i32; 2],
        pv_node: bool,
        synthetic: bool,
        context: u64,
        scope: u64,
    ) -> Self {
        let key = scope
            ^ board.key().full()
            ^ context.rotate_left(17)
            ^ u64::from(board.halfmove_clock()).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        Self {
            board,
            accumulator,
            depth,
            ply,
            alpha: window[0],
            beta: window[1],
            original_alpha: window[0],
            pv_node,
            synthetic,
            key,
            context,
            in_check: false,
            evaluation: 0,
            best: -INFINITY,
            best_move: None,
            pv: Vec::new(),
            candidates: Vec::new(),
            next: 0,
            stage: Stage::Enter,
            cache_policy: if synthetic {
                CachePolicy::Skip
            } else {
                CachePolicy::Write
            },
        }
    }
    pub(super) const fn bound(&self, score: i32) -> Bound {
        if score >= self.beta {
            Bound::Lower
        } else if score > self.original_alpha {
            Bound::Exact
        } else {
            Bound::Upper
        }
    }
}

#[derive(Debug)]
pub(super) enum Action {
    Keep,
    Descend {
        depth: i16,
        window: [i32; 2],
        null: bool,
    },
    Complete(NodeResult),
}
