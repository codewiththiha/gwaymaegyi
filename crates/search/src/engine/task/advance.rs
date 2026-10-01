//! Node entry, terminal checks, pruning, and child-search scheduling.

use super::Task;
use crate::engine::{
    frame::{Action, CachePolicy, Frame, NodeResult, Pending, Probe, Stage},
    table::{Bound, Cache},
};
use crate::{Behavior, EngineError, INFINITY, MATE, MAX_PLY, Parameter};
use gwaymaegyi_core::{MoveKind, PieceKind};

impl Task {
    pub(super) fn advance(
        &mut self,
        frame: &mut Frame,
        cache: &mut Cache,
    ) -> Result<Action, EngineError> {
        match &frame.stage {
            Stage::Enter => Ok(self.enter(frame, cache)),
            Stage::Moves => Ok(self.next(frame)),
            Stage::Returned(_, _) => self.returned(frame, cache),
            Stage::Waiting(_) => Err(EngineError::InternalState),
        }
    }

    fn enter(&mut self, frame: &mut Frame, cache: &Cache) -> Action {
        self.report.nodes += 1;
        self.report.selective_depth = self.report.selective_depth.max(frame.ply);
        frame.in_check = frame.board.in_check(frame.board.side_to_move());
        frame.candidates = frame.board.legal_successors();
        if frame.ply == 0 {
            if !self.excluded.is_empty() || frame.candidates.len() != self.root_moves.len() {
                frame.cache_policy = CachePolicy::Skip;
            }
            frame.candidates.retain(|child| {
                self.root_moves.contains(&child.chess_move())
                    && !self.excluded.contains(&child.chess_move())
            });
        }
        if frame.candidates.is_empty() {
            let score = if frame.in_check {
                i32::from(frame.ply) - MATE
            } else {
                0
            };
            return Action::Complete(NodeResult {
                score,
                pv: Vec::new(),
            });
        }
        if frame.board.insufficient_material()
            || (!frame.synthetic
                && frame.ply > 0
                && (frame.board.halfmove_clock() >= 100 || self.repetitions(frame) >= 3))
        {
            frame.cache_policy = CachePolicy::Skip;
            return Action::Complete(NodeResult {
                score: 0,
                pv: Vec::new(),
            });
        }
        frame.evaluation = self.evaluate(frame);
        if frame.ply >= MAX_PLY - 1 {
            return Action::Complete(NodeResult {
                score: frame.evaluation,
                pv: Vec::new(),
            });
        }
        frame.alpha = frame.alpha.max(-MATE + i32::from(frame.ply));
        frame.beta = frame.beta.min(MATE - i32::from(frame.ply) - 1);
        if frame.alpha >= frame.beta {
            return Action::Complete(NodeResult {
                score: frame.alpha,
                pv: Vec::new(),
            });
        }
        if let Some(result) = Self::cached(frame, cache) {
            return Action::Complete(result);
        }
        Self::order(frame, cache);
        frame.stage = Stage::Moves;
        if frame.depth <= 0 && !frame.in_check {
            frame.best = frame.evaluation;
            if frame.best >= frame.beta {
                return Action::Complete(NodeResult {
                    score: frame.best,
                    pv: Vec::new(),
                });
            }
            frame.alpha = frame.alpha.max(frame.best);
            frame.candidates.retain(|child| {
                frame.board.is_capture(child.chess_move())
                    || matches!(child.chess_move().kind(), MoveKind::Promotion(_))
            });
        }
        self.prune(frame)
    }

    fn cached(frame: &Frame, cache: &Cache) -> Option<NodeResult> {
        if !frame.pv_node && !frame.synthetic {
            if let Some(entry) = cache
                .probe(frame.key)
                .filter(|entry| entry.depth >= frame.depth.max(0))
            {
                let score = entry.score_at(frame.ply);
                if entry.bound == Bound::Exact
                    || (entry.bound == Bound::Lower && score >= frame.beta)
                    || (entry.bound == Bound::Upper && score <= frame.alpha)
                {
                    return Some(NodeResult {
                        score,
                        pv: Vec::new(),
                    });
                }
            }
        }
        None
    }

    fn prune(&self, frame: &mut Frame) -> Action {
        let tuning = self.options.tuning();
        if frame.depth > 0 && !frame.in_check && !frame.pv_node && frame.ply > 0 {
            if tuning.enabled(Behavior::ReverseFutility)
                && i32::from(frame.depth) <= tuning.get(Parameter::RfpDepth)
                && frame.evaluation - tuning.get(Parameter::RfpMargin) * i32::from(frame.depth)
                    >= frame.beta
            {
                return Action::Complete(NodeResult {
                    score: (frame.evaluation + frame.beta) / 2,
                    pv: Vec::new(),
                });
            }
            let non_pawn = [
                PieceKind::Knight,
                PieceKind::Bishop,
                PieceKind::Rook,
                PieceKind::Queen,
            ]
            .into_iter()
            .any(|kind| {
                !frame
                    .board
                    .pieces(frame.board.side_to_move(), kind)
                    .is_empty()
            });
            if tuning.enabled(Behavior::NullMove)
                && !frame.synthetic
                && non_pawn
                && i32::from(frame.depth) >= tuning.get(Parameter::NullMinDepth)
                && frame.evaluation >= frame.beta
            {
                let reduction = i16::try_from(
                    tuning.get(Parameter::NullBase)
                        + i32::from(frame.depth) / tuning.get(Parameter::NullDepthDiv)
                        + ((frame.evaluation - frame.beta) / tuning.get(Parameter::NullEvalDiv))
                            .min(3),
                )
                .unwrap_or(frame.depth);
                frame.stage = Stage::Waiting(Pending {
                    index: 0,
                    depth: frame.depth - reduction,
                    probe: Probe::Null,
                });
                return Action::Descend {
                    depth: frame.depth - reduction,
                    window: [-frame.beta, 1 - frame.beta],
                    null: true,
                };
            }
        }
        Action::Keep
    }

    fn next(&self, frame: &mut Frame) -> Action {
        let tuning = self.options.tuning();
        while let Some(candidate) = frame.candidates.get(frame.next) {
            let index = frame.next;
            frame.next += 1;
            let chess_move = candidate.chess_move();
            let quiet = !frame.board.is_capture(chess_move)
                && !matches!(chess_move.kind(), MoveKind::Promotion(_));
            let gives_check = candidate.board().in_check(candidate.board().side_to_move());
            let shallow_quiet = tuning.enabled(Behavior::QuietPruning)
                && !frame.pv_node
                && !frame.in_check
                && frame.best > -MATE + 100
                && quiet
                && !gives_check
                && frame.depth >= 1
                && i32::from(frame.depth) <= tuning.get(Parameter::QuietDepth);
            let futile = frame.evaluation
                + tuning.get(Parameter::FutilityBase)
                + tuning.get(Parameter::FutilityMargin) * i32::from(frame.depth)
                <= frame.alpha;
            let late = index
                >= usize::try_from(
                    tuning.get(Parameter::LateBase)
                        + i32::from(frame.depth) * i32::from(frame.depth),
                )
                .unwrap_or(usize::MAX);
            if shallow_quiet && (futile || late) {
                continue;
            }
            let depth = frame.depth - 1;
            let reduction = if tuning.enabled(Behavior::LateReductions)
                && quiet
                && !gives_check
                && !frame.in_check
                && i32::from(frame.depth) >= tuning.get(Parameter::ReductionDepth)
                && index >= 3
            {
                (frame.depth / 3 + i16::try_from(index / 8).unwrap_or(3) - i16::from(frame.pv_node))
                    .clamp(0, (depth - 1).max(0))
            } else {
                0
            };
            let (probe, window) = if index == 0 {
                (Probe::Full, [-frame.beta, -frame.alpha])
            } else if reduction > 0 {
                (Probe::Reduced, [-frame.alpha - 1, -frame.alpha])
            } else {
                (Probe::Scout, [-frame.alpha - 1, -frame.alpha])
            };
            frame.stage = Stage::Waiting(Pending {
                index,
                depth,
                probe,
            });
            return Action::Descend {
                depth: depth - reduction,
                window,
                null: false,
            };
        }
        let score = if frame.best == -INFINITY {
            frame.evaluation
        } else {
            frame.best
        };
        Action::Complete(NodeResult {
            score,
            pv: frame.pv.clone(),
        })
    }
}
