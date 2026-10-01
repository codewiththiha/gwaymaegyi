//! Owned node windows, pending probes, and explicit continuation states.

use super::table::Bound;
use crate::INFINITY;
use gwaymaegyi_core::{Board, Move, Successor};
use gwaymaegyi_eval::Accumulator;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Probe {
    Full,
    Scout,
    Reduced,
    Null,
    Singular,
    Probcut,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Pending {
    pub index: usize,
    pub depth: i16,
    pub probe: Probe,
    /// Singular-verification beta or probcut beta for the matching probe.
    pub beta: i32,
    pub window: [i32; 2],
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

const PV: u8 = 1;
const CUT: u8 = 2;
const SYNTHETIC: u8 = 4;
const IN_CHECK: u8 = 8;
const IMPROVING: u8 = 16;
const SKIP_QUIET: u8 = 32;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct Flags(u8);

impl Flags {
    pub(super) const fn new(pv_node: bool, cutnode: bool, synthetic: bool) -> Self {
        let mut bits = 0u8;
        if pv_node {
            bits |= PV;
        }
        if cutnode {
            bits |= CUT;
        }
        if synthetic {
            bits |= SYNTHETIC;
        }
        Self(bits)
    }
    #[must_use]
    pub(super) const fn pv_node(self) -> bool {
        self.0 & PV != 0
    }
    #[must_use]
    pub(super) const fn cutnode(self) -> bool {
        self.0 & CUT != 0
    }
    #[must_use]
    pub(super) const fn synthetic(self) -> bool {
        self.0 & SYNTHETIC != 0
    }
    #[must_use]
    pub(super) const fn in_check(self) -> bool {
        self.0 & IN_CHECK != 0
    }
    #[must_use]
    pub(super) const fn improving(self) -> bool {
        self.0 & IMPROVING != 0
    }
    #[must_use]
    pub(super) const fn skip_quiet(self) -> bool {
        self.0 & SKIP_QUIET != 0
    }
    pub(super) const fn set_in_check(&mut self, value: bool) {
        if value {
            self.0 |= IN_CHECK;
        } else {
            self.0 &= !IN_CHECK;
        }
    }
    pub(super) const fn set_improving(&mut self, value: bool) {
        if value {
            self.0 |= IMPROVING;
        } else {
            self.0 &= !IMPROVING;
        }
    }
    pub(super) const fn set_skip_quiet(&mut self, value: bool) {
        if value {
            self.0 |= SKIP_QUIET;
        } else {
            self.0 &= !SKIP_QUIET;
        }
    }
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
    pub flags: Flags,
    pub key: u64,
    pub context: u64,
    pub evaluation: i32,
    pub best: i32,
    pub best_move: Option<Move>,
    pub pv: Vec<Move>,
    pub candidates: Vec<ScoredMove>,
    pub next: usize,
    pub excluded: Option<Move>,
    pub tt_move: Option<Move>,
    pub tt_depth: i16,
    pub tt_score: i32,
    pub tt_bound: Option<Bound>,
    pub stage: Stage,
    pub cache_policy: CachePolicy,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ScoredMove {
    pub successor: Successor,
    pub score: i32,
    pub is_capture: bool,
    pub is_quiet: bool,
}

impl ScoredMove {
    pub(super) const fn chess_move(&self) -> Move {
        self.successor.chess_move()
    }

    pub(super) const fn board(&self) -> &Board {
        self.successor.board()
    }
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
            flags: Flags::new(pv_node, window[1] - window[0] == 1 && !pv_node, synthetic),
            key,
            context,
            evaluation: 0,
            best: -INFINITY,
            best_move: None,
            pv: Vec::new(),
            candidates: Vec::new(),
            next: 0,
            excluded: None,
            tt_move: None,
            tt_depth: 0,
            tt_score: 0,
            tt_bound: None,
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
    Descend,
    Complete(NodeResult),
}
