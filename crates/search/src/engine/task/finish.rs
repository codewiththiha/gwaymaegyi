//! Child-result delivery, re-search decisions, and completed root selection.

use super::Task;
use crate::engine::{
    frame::{Action, Frame, NodeResult, Pending, Probe, Stage},
    table::Cache,
};
use crate::{Completion, EngineError, Mode, Parameter, PrincipalVariation, Strength};

impl Task {
    pub(super) fn returned(
        &self,
        frame: &mut Frame,
        cache: &mut Cache,
    ) -> Result<Action, EngineError> {
        let Stage::Returned(pending, result) = std::mem::replace(&mut frame.stage, Stage::Moves)
        else {
            return Err(EngineError::InternalState);
        };
        let tuning = self.options.tuning();
        let score = -result.score;
        match pending.probe {
            Probe::Null => {
                if score >= frame.beta {
                    return Ok(Action::Complete(NodeResult {
                        score: frame.beta,
                        pv: Vec::new(),
                    }));
                }
                return Ok(Action::Keep);
            }
            Probe::Probcut => {
                let p_beta = pending.beta;
                if score >= p_beta {
                    return Ok(Action::Complete(NodeResult {
                        score,
                        pv: Vec::new(),
                    }));
                }
                return Ok(Action::Keep);
            }
            Probe::Singular => return Ok(self.apply_singular(frame, pending, score, tuning)),
            Probe::Reduced if score > frame.alpha => {
                Self::schedule_retry(
                    frame,
                    pending.index,
                    pending.depth,
                    Probe::Scout,
                    [-frame.alpha - 1, -frame.alpha],
                );
                return Ok(Action::Descend);
            }
            Probe::Scout if score > frame.alpha && frame.flags.pv_node() => {
                Self::schedule_retry(
                    frame,
                    pending.index,
                    pending.depth,
                    Probe::Full,
                    [-frame.beta, -frame.alpha],
                );
                return Ok(Action::Descend);
            }
            Probe::Full | Probe::Scout | Probe::Reduced => {}
        }
        let chess_move = frame
            .candidates
            .get(pending.index)
            .ok_or(EngineError::InternalState)?
            .chess_move();
        if score > frame.best {
            frame.best = score;
            frame.best_move = Some(chess_move);
            frame.pv = vec![chess_move];
            frame.pv.extend(result.pv);
        }
        frame.alpha = frame.alpha.max(score);
        if score >= frame.beta {
            let priors = self.priors();
            let bonus = (i32::from(frame.depth.max(1)) * tuning.get(Parameter::HistBonus))
                .min(tuning.get(Parameter::HistMax));
            for preceding in frame
                .candidates
                .iter()
                .take(pending.index)
                .filter(|item| item.is_quiet)
            {
                cache
                    .history
                    .record_malus(&frame.board, preceding.chess_move(), -bonus);
            }
            cache
                .history
                .record_cutoff(&frame.board, chess_move, frame.ply, bonus, priors);
            return Ok(Action::Complete(NodeResult {
                score,
                pv: frame.pv.clone(),
            }));
        }
        Ok(Action::Keep)
    }

    fn apply_singular(
        &self,
        frame: &mut Frame,
        pending: Pending,
        score: i32,
        tuning: crate::SearchTuning,
    ) -> Action {
        let s_beta = pending.beta;
        let extension = if score < s_beta {
            if !frame.flags.pv_node()
                && score + tuning.get(Parameter::SingularDoubleMargin) < s_beta
                && i32::from(frame.ply) < i32::from(self.iteration)
            {
                let tt_quiet = frame
                    .tt_move
                    .is_some_and(|chess_move| !frame.board.is_capture(chess_move));
                2 + i32::from(
                    tt_quiet && score < s_beta - tuning.get(Parameter::SingularTripleMargin),
                )
            } else {
                1
            }
        } else if s_beta >= frame.beta {
            // Multicut: another move beat beta, so this one probably will too.
            return Action::Complete(NodeResult {
                score: s_beta,
                pv: Vec::new(),
            });
        } else if frame.flags.cutnode() {
            -1
        } else {
            0
        };
        let depth = (frame.depth - 1 + i16::try_from(extension).unwrap_or(0)).max(1);
        let (probe, window) = if frame.flags.pv_node() {
            (Probe::Full, [-frame.beta, -frame.alpha])
        } else {
            (Probe::Scout, [-frame.alpha - 1, -frame.alpha])
        };
        Self::schedule_retry(frame, 0, depth, probe, window);
        Action::Descend
    }

    fn schedule_retry(frame: &mut Frame, index: usize, depth: i16, probe: Probe, window: [i32; 2]) {
        frame.stage = Stage::Waiting(Pending {
            index,
            depth,
            probe,
            beta: 0,
            window,
        });
    }

    pub(super) fn finish_pass(&mut self, result: NodeResult) {
        if self
            .window
            .as_mut()
            .is_some_and(|window| window.retry(result.score))
        {
            return;
        }
        self.window = None;
        if let Some(&chess_move) = result.pv.first() {
            self.excluded.push(chess_move);
            self.lines.push(PrincipalVariation {
                score_cp: result.score,
                moves: result.pv,
            });
            let count = usize::from(self.options.effective_pv()).min(self.root_moves.len());
            if self.lines.len() < count {
                return;
            }
            self.report.depth = self.iteration;
            self.report.variations = std::mem::take(&mut self.lines);
            self.report.best_move_nodes = self
                .report
                .variations
                .first()
                .and_then(|line| line.moves.first())
                .and_then(|move_| {
                    self.root_moves
                        .iter()
                        .position(|root| *root == *move_)
                        .map(|index| self.root_nodes[index])
                })
                .unwrap_or(0);
            // Lines complete in search order, not score order; publish them sorted.
            self.report
                .variations
                .sort_by_key(|line| std::cmp::Reverse(line.score_cp));
            let chosen = self.choose();
            if let Some(line) = self.report.variations.get(chosen) {
                self.report.best_move = line.moves.first().copied();
                self.report.score_cp = Some(line.score_cp);
            }
            if self.iteration >= 6
                && matches!(self.options.mode(), Mode::Aggressive | Mode::Human)
                && let Some(best) = self.report.variations.first()
            {
                self.update_phase_model(best.score_cp);
            }
        } else {
            self.report.score_cp = Some(result.score);
            if self.root.legal_moves().is_empty() || self.root.insufficient_material() {
                self.report.best_move = None;
            }
            self.stop(Completion::Terminal);
            return;
        }
        self.excluded.clear();
        if self.iteration >= self.limits.depth {
            self.stop(Completion::Depth);
        } else {
            self.iteration += 1;
            self.root_nodes.fill(0);
        }
    }

    pub(super) fn update_phase_model(&mut self, root_score: i32) {
        let total = self.root.material(gwaymaegyi_core::Color::White)
            + self.root.material(gwaymaegyi_core::Color::Black);
        self.active_model = if total < 3_500 || root_score < -20 {
            gwaymaegyi_eval::Model::Endgame
        } else if root_score > 400 {
            gwaymaegyi_eval::Model::Aggressive
        } else {
            gwaymaegyi_eval::Model::Balanced
        };
    }

    /// Human-style selection: opening diversity, an accumulated mistake
    /// budget, and sacrifice preference, all deterministic under the seed.
    fn choose(&mut self) -> usize {
        if self.options.mode() == Mode::Analysis
            || !matches!(self.options.strength(), Strength::Approximate(_))
        {
            self.last_mistake = None;
            return 0;
        }
        let Some(best) = self.report.variations.first() else {
            self.last_mistake = None;
            return 0;
        };
        let our_color = self.root.side_to_move();
        let starting_mat = self.root.material(our_color);
        let ply = i32::try_from(self.game.position_history().len()).unwrap_or(0) - 1;
        let mut best_index = 0;
        let mut mistake: Option<i32> = None;
        let mut sacrifice: i32 = 0;
        for (index, line) in self.report.variations.iter().enumerate().take(5) {
            let eval_diff = best.score_cp - line.score_cp;
            let m_diff = starting_mat - self.pv_material(&line.moves, our_color);
            if ply < 7 && eval_diff < 25 && line.score_cp > -100 && index < 4 {
                let draw =
                    (self.options.seed() ^ self.root.key().full() ^ u64::from(self.iteration)) % 5;
                if draw == index as u64 {
                    best_index = index;
                    mistake = Some(eval_diff);
                }
            }
            if ply >= 7 && eval_diff > 0 && eval_diff < self.style_loss + 10 && best.score_cp < 5000
            {
                best_index = index;
                mistake = Some(eval_diff);
            }
            if line.score_cp > 0
                && m_diff < sacrifice
                && ((m_diff < 0 && eval_diff < 50)
                    || (m_diff <= -200 && eval_diff < 100)
                    || (m_diff <= -400 && eval_diff < 200))
            {
                best_index = index;
                mistake = Some(eval_diff);
                sacrifice = m_diff;
            }
        }
        self.last_mistake = mistake;
        best_index
    }

    /// Material for our perspective at the end of a principal variation.
    fn pv_material(
        &self,
        moves: &[gwaymaegyi_core::Move],
        our_color: gwaymaegyi_core::Color,
    ) -> i32 {
        let mut board = self.root;
        for chess_move in moves {
            let notation = chess_move.to_uci(self.options.chess960());
            board = match board.play_uci(&notation, self.options.chess960()) {
                Ok(board) => board,
                Err(_) => break,
            };
        }
        board.material(our_color)
    }
}
