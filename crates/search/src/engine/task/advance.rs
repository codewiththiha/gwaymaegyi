//! Node entry, terminal checks, pruning, and child-search scheduling.

use super::Task;
use crate::engine::{
    frame::{Action, CachePolicy, Frame, NodeResult, Pending, Probe, ScoredMove, Stage},
    table::{Bound, Cache},
};
use crate::{Behavior, EngineError, INFINITY, MATE, MAX_PLY, Mode, Parameter, Strength};
use gwaymaegyi_core::PieceKind;

impl Task {
    pub(super) fn advance(
        &mut self,
        frame: &mut Frame,
        cache: &mut Cache,
    ) -> Result<Action, EngineError> {
        match &frame.stage {
            Stage::Enter => Ok(self.enter(frame, cache)),
            Stage::Moves => Ok(self.next(frame, cache)),
            Stage::Returned(_, _) => self.returned(frame, cache),
            Stage::Waiting(_) => Err(EngineError::InternalState),
        }
    }

    fn enter(&mut self, frame: &mut Frame, cache: &Cache) -> Action {
        self.report.nodes += 1;
        self.report.selective_depth = self.report.selective_depth.max(frame.ply);
        let tuning = self.options.tuning();
        frame
            .flags
            .set_in_check(frame.board.in_check(frame.board.side_to_move()));
        let successors = self.filtered_successors(frame);
        if let Some(action) = self.terminal_or_draw(frame, successors.len()) {
            return action;
        }
        frame.evaluation = self.evaluate(frame);
        let corr = cache.history.correction(&frame.board, self.priors());
        frame.evaluation = (frame.evaluation + tuning.get(Parameter::CorrWeight) * corr / 512)
            .clamp(-28_000, 28_000);
        let improving = frame.ply > 1
            && !frame.flags.in_check()
            && self
                .frames
                .get(self.frames.len().saturating_sub(2))
                .is_some_and(|grandparent| frame.evaluation > grandparent.evaluation);
        frame.flags.set_improving(improving);
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
        if let Some(entry) = cache.probe(frame.key) {
            if !frame.flags.synthetic() {
                frame.tt_move = entry.best;
                frame.tt_depth = entry.depth;
                frame.tt_score = entry.score_at(frame.ply);
                frame.tt_bound = Some(entry.bound);
            }
        }
        if let Some(result) = Self::cached(frame) {
            return Action::Complete(result);
        }
        if let Some(result) = self.tablebase(frame) {
            return result;
        }
        if tuning.enabled(Behavior::InternalReductions)
            && (frame.flags.pv_node() || frame.flags.cutnode())
            && frame.tt_move.is_none()
            && !frame.flags.in_check()
            && frame.excluded.is_none()
            && i32::from(frame.depth) > tuning.get(Parameter::IirDepth)
        {
            frame.depth -= 1;
        }
        if tuning.enabled(Behavior::Razoring)
            && !frame.flags.pv_node()
            && !frame.flags.in_check()
            && frame.depth > 0
            && i32::from(frame.depth) < tuning.get(Parameter::RazoringDepth)
            && frame.evaluation + tuning.get(Parameter::RazoringMargin) * i32::from(frame.depth)
                <= frame.alpha
        {
            frame.depth = 0;
        }
        Self::order(frame, &cache.history, successors, self.priors());
        frame.stage = Stage::Moves;
        if frame.depth <= 0 && !frame.flags.in_check() {
            frame.best = frame.evaluation;
            if frame.best >= frame.beta {
                return Action::Complete(NodeResult {
                    score: frame.best,
                    pv: Vec::new(),
                });
            }
            frame.alpha = frame.alpha.max(frame.best);
            frame
                .candidates
                .retain(|item| item.is_capture || !item.is_quiet);
            return Action::Keep;
        }
        if let Some(action) = Self::reverse_futility(frame, tuning) {
            return action;
        }
        if let Some(action) = Self::null_move(frame, tuning) {
            return action;
        }
        Action::Keep
    }

    fn tablebase(&mut self, frame: &Frame) -> Option<Action> {
        if frame.depth <= 0
            || frame.ply == 0
            || frame.flags.synthetic()
            || matches!(self.options.strength(), Strength::Approximate(_))
            || self.options.mode() == Mode::Human
        {
            return None;
        }
        let wdl = self.tablebase.as_ref()?.probe_wdl(&frame.board)?;
        self.report.tablebase_hits = self.report.tablebase_hits.saturating_add(1);
        let score = match wdl {
            crate::TablebaseWdl::Win => 29_000,
            crate::TablebaseWdl::Loss => -29_000,
            crate::TablebaseWdl::Draw
            | crate::TablebaseWdl::CursedWin
            | crate::TablebaseWdl::BlessedLoss => 0,
        };
        Some(Action::Complete(NodeResult {
            score,
            pv: Vec::new(),
        }))
    }

    fn filtered_successors(&self, frame: &mut Frame) -> Vec<gwaymaegyi_core::Successor> {
        let mut successors = frame.board.legal_successors();
        if frame.ply == 0 {
            if !self.excluded.is_empty() || successors.len() != self.root_moves.len() {
                frame.cache_policy = CachePolicy::Skip;
            }
            successors.retain(|child| {
                self.root_moves.contains(&child.chess_move())
                    && !self.excluded.contains(&child.chess_move())
            });
        } else if let Some(excluded) = frame.excluded {
            successors.retain(|child| child.chess_move() != excluded);
        }
        successors
    }

    fn terminal_or_draw(&self, frame: &mut Frame, count: usize) -> Option<Action> {
        if count == 0 {
            let score = if frame.excluded.is_some() {
                frame.cache_policy = CachePolicy::Skip;
                frame.alpha
            } else if frame.flags.in_check() {
                i32::from(frame.ply) - MATE
            } else {
                0
            };
            return Some(Action::Complete(NodeResult {
                score,
                pv: Vec::new(),
            }));
        }
        if frame.board.insufficient_material()
            || (!frame.flags.synthetic()
                && frame.ply > 0
                && (frame.board.halfmove_clock() >= 100 || self.repetitions(frame) >= 3))
        {
            frame.cache_policy = CachePolicy::Skip;
            let score = if !frame.board.insufficient_material()
                && frame.ply > 0
                && self.options.mode() == Mode::Aggressive
            {
                let side = frame.board.side_to_move();
                let diff = frame.board.material(side) - frame.board.material(side.opposite());
                if diff < -100 {
                    50
                } else if diff > 100 {
                    -50
                } else {
                    0
                }
            } else {
                0
            };
            return Some(Action::Complete(NodeResult {
                score,
                pv: Vec::new(),
            }));
        }
        None
    }

    fn reverse_futility(frame: &Frame, tuning: crate::SearchTuning) -> Option<Action> {
        if frame.depth <= 0 || frame.flags.in_check() || frame.flags.pv_node() || frame.ply == 0 {
            return None;
        }
        if tuning.enabled(Behavior::ReverseFutility)
            && i32::from(frame.depth) <= tuning.get(Parameter::RfpMaxDepth)
            && frame.evaluation
                - tuning.get(Parameter::RfpMargin)
                    * i32::from(frame.depth - i16::from(frame.flags.improving()))
                >= frame.beta
        {
            return Some(Action::Complete(NodeResult {
                score: (frame.evaluation + frame.beta) / 2,
                pv: Vec::new(),
            }));
        }
        None
    }

    fn null_move(frame: &mut Frame, tuning: crate::SearchTuning) -> Option<Action> {
        if frame.depth <= 0 || frame.flags.in_check() || frame.flags.pv_node() || frame.ply == 0 {
            return None;
        }
        if !tuning.enabled(Behavior::NullMove)
            || frame.flags.synthetic()
            || i32::from(frame.depth) < tuning.get(Parameter::NullMinDepth)
            || frame.evaluation < frame.beta
            || !Self::has_non_pawn(frame)
        {
            return None;
        }
        let reduction = i16::try_from(
            tuning.get(Parameter::NullBase)
                + i32::from(frame.depth) / tuning.get(Parameter::NullDepthDiv)
                + ((frame.evaluation - frame.beta) / tuning.get(Parameter::NullEvalDiv)).min(3),
        )
        .unwrap_or(frame.depth)
        .min(frame.depth - 1);
        frame.stage = Stage::Waiting(Pending {
            index: 0,
            depth: frame.depth - reduction,
            probe: Probe::Null,
            beta: 0,
            window: [-frame.beta, 1 - frame.beta],
        });
        Some(Action::Descend)
    }

    fn cached(frame: &Frame) -> Option<NodeResult> {
        if !frame.flags.pv_node()
            && !frame.flags.synthetic()
            && frame.tt_depth >= frame.depth.max(0)
        {
            let bound = frame.tt_bound?;
            let score = frame.tt_score;
            if bound == Bound::Exact
                || (bound == Bound::Lower && score >= frame.beta)
                || (bound == Bound::Upper && score <= frame.alpha)
            {
                return Some(NodeResult {
                    score,
                    pv: Vec::new(),
                });
            }
        }
        None
    }

    fn has_non_pawn(frame: &Frame) -> bool {
        [
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
        })
    }

    fn lmr_depth(&self, frame: &Frame, index: usize) -> i32 {
        let depth = usize::try_from(frame.depth.max(1)).unwrap_or(64).min(64);
        (i32::from(frame.depth.max(1)) - i32::from(self.lmr_table[depth * 65 + index.min(64)]))
            .max(1)
    }

    /// Schedule singular verification of the TT move before searching it.
    fn singular(&self, frame: &Frame, item: &ScoredMove, index: usize) -> bool {
        let tuning = self.options.tuning();
        let Some(tt) = frame
            .tt_move
            .filter(|chess_move| *chess_move == item.chess_move())
        else {
            return false;
        };
        let _ = tt;
        tuning.enabled(Behavior::SingularExtensions)
            && index == 0
            && frame.ply > 0
            && frame.ply < self.iteration.saturating_mul(2)
            && frame.depth > 0
            && i32::from(frame.depth) >= tuning.get(Parameter::SingularDepth)
            && !frame.flags.synthetic()
            && frame.excluded.is_none()
            && frame.tt_score.abs() < 31_000
            && frame.tt_depth >= frame.depth - 3
            && frame.tt_bound != Some(Bound::Upper)
    }

    fn next(&self, frame: &mut Frame, cache: &Cache) -> Action {
        let tuning = self.options.tuning();
        while let Some(item) = frame.candidates.get(frame.next).copied() {
            let index = frame.next;
            frame.next += 1;
            let chess_move = item.chess_move();
            if self.late_prune(frame, &item, index, tuning, cache) {
                continue;
            }
            if self.singular(frame, &item, index) {
                let s_beta = frame.tt_score - i32::from(frame.depth);
                frame.stage = Stage::Waiting(Pending {
                    index,
                    depth: (frame.depth - 1) / 2,
                    probe: Probe::Singular,
                    beta: s_beta,
                    window: [s_beta - 1, s_beta],
                });
                return Action::Descend;
            }
            if let Some(p_beta) = Self::probcut(frame, &item, tuning) {
                if frame
                    .board
                    .static_exchange_gain(chess_move, p_beta - frame.evaluation)
                {
                    frame.stage = Stage::Waiting(Pending {
                        index,
                        depth: frame.depth - 4,
                        probe: Probe::Probcut,
                        beta: p_beta,
                        window: [-p_beta, -p_beta + 1],
                    });
                    return Action::Descend;
                }
                continue;
            }
            let gives_check = item.board().in_check(item.board().side_to_move());
            let (probe, descend_depth, window) =
                self.schedule(frame, &item, index, gives_check, tuning, cache);
            frame.stage = Stage::Waiting(Pending {
                index,
                depth: descend_depth,
                probe,
                beta: 0,
                window,
            });
            return Action::Descend;
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

    /// Late-move, futility, history, and SEE pruning. True skips this move.
    fn late_prune(
        &self,
        frame: &mut Frame,
        item: &ScoredMove,
        index: usize,
        tuning: crate::SearchTuning,
        cache: &Cache,
    ) -> bool {
        let chess_move = item.chess_move();
        // Exchange analysis models the moving piece as capturable, which is unsound
        // for checks; the reference omits this guard and can miss mates here.
        let gives_check = item.board().in_check(item.board().side_to_move());
        let quiet_prune = item.is_quiet
            && !frame.flags.in_check()
            && frame.ply > 0
            && frame.depth > 0
            && !frame.flags.pv_node()
            && frame.best > -INFINITY;
        if quiet_prune {
            let lmr_depth = self.lmr_depth(frame, index);
            let divisor = if frame.flags.improving() { 1 } else { 2 };
            let late = i32::from(frame.depth) < tuning.get(Parameter::LmpDepth)
                && i32::try_from(index).unwrap_or(i32::MAX)
                    >= tuning.get(Parameter::LmpBase)
                        + i32::from(frame.depth) * i32::from(frame.depth) / divisor;
            let futile = i32::from(frame.depth) < tuning.get(Parameter::FutilityDepth)
                && frame.evaluation
                    + tuning.get(Parameter::FutilityMargin1)
                    + tuning.get(Parameter::FutilityMargin2) * lmr_depth
                    < frame.alpha;
            if late || futile {
                frame.flags.set_skip_quiet(true);
            } else if tuning.enabled(Behavior::HistoryPruning)
                && lmr_depth < tuning.get(Parameter::HistPruneDepth)
                && cache.history.quiet_only(&frame.board, chess_move) < -4096 * lmr_depth
            {
                return true;
            }
        }
        if frame.flags.skip_quiet() && (item.is_quiet || item.score < 0) {
            return true;
        }
        if tuning.enabled(Behavior::ExchangePruning)
            && !gives_check
            && frame.ply > 0
            && frame.depth > 0
            && frame.best > -INFINITY
            && i32::from(frame.depth) < tuning.get(Parameter::SeePruneDepth)
        {
            let margin = if item.is_capture {
                tuning.get(Parameter::SeePruneNoisy)
            } else {
                tuning.get(Parameter::SeePruneQuiet)
            };
            if !frame
                .board
                .static_exchange_gain(chess_move, i32::from(frame.depth) * margin)
            {
                return true;
            }
        }
        false
    }

    /// Probcut beta for a SEE-passing capture on a cutnode, if eligible.
    fn probcut(frame: &Frame, item: &ScoredMove, tuning: crate::SearchTuning) -> Option<i32> {
        if !(tuning.enabled(Behavior::Probcut)
            && frame.flags.cutnode()
            && frame.depth > 0
            && item.is_capture
            && !frame.flags.in_check()
            && frame.beta.abs() < 31_000
            && i32::from(frame.depth) >= tuning.get(Parameter::ProbcutDepth))
        {
            return None;
        }
        let p_beta = frame.beta + tuning.get(Parameter::ProbcutMargin);
        if frame.tt_score >= p_beta || frame.evaluation >= p_beta {
            Some(p_beta)
        } else {
            None
        }
    }

    /// Choose probe kind, depth (with LMR), and window for a move descent.
    fn schedule(
        &self,
        frame: &Frame,
        item: &ScoredMove,
        index: usize,
        gives_check: bool,
        tuning: crate::SearchTuning,
        cache: &Cache,
    ) -> (Probe, i16, [i32; 2]) {
        let newdepth = frame.depth - 1;
        if index == 0 {
            return (Probe::Full, newdepth, [-frame.beta, -frame.alpha]);
        }
        if !(tuning.enabled(Behavior::LateReductions)
            && frame.depth > 0
            && i32::from(frame.depth) >= tuning.get(Parameter::LmrMinDepth))
        {
            return (Probe::Scout, newdepth, [-frame.alpha - 1, -frame.alpha]);
        }
        let mut r =
            self.lmr_table[usize::try_from(frame.depth).unwrap_or(64).min(64) * 65 + index.min(64)];
        if item.is_capture {
            r /= 2;
        } else {
            let hist = cache.history.quiet_only(&frame.board, item.chess_move())
                / tuning.get(Parameter::HistDiv);
            r -= i16::try_from(hist).unwrap_or(-200);
        }
        r -= i16::from(frame.flags.pv_node());
        if frame.tt_bound.is_some() && frame.tt_depth >= frame.depth {
            r -= 1;
        }
        if !frame.flags.improving() {
            r += 1;
        }
        if frame.flags.cutnode() {
            r += 1;
        }
        if gives_check {
            r -= 1;
        }
        let r = r.clamp(0, newdepth - 1);
        (
            Probe::Reduced,
            (newdepth - r).max(1),
            [-frame.alpha - 1, -frame.alpha],
        )
    }
}
