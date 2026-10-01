//! Child-result delivery, re-search decisions, and completed root variation selection.

use super::Task;
use crate::engine::{
    frame::{Action, Frame, NodeResult, Probe, Stage},
    table::Cache,
};
use crate::{Completion, EngineError, Mode, PrincipalVariation};

impl Task {
    pub(super) fn returned(frame: &mut Frame, cache: &mut Cache) -> Result<Action, EngineError> {
        let Stage::Returned(mut pending, result) =
            std::mem::replace(&mut frame.stage, Stage::Moves)
        else {
            return Err(EngineError::InternalState);
        };
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
            Probe::Reduced if score > frame.alpha => {
                pending.probe = Probe::Scout;
                frame.stage = Stage::Waiting(pending);
                return Ok(Action::Descend {
                    depth: pending.depth,
                    window: [-frame.alpha - 1, -frame.alpha],
                    null: false,
                });
            }
            Probe::Scout if score > frame.alpha && frame.pv_node => {
                pending.probe = Probe::Full;
                frame.stage = Stage::Waiting(pending);
                return Ok(Action::Descend {
                    depth: pending.depth,
                    window: [-frame.beta, -frame.alpha],
                    null: false,
                });
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
            for candidate in frame.candidates.iter().take(pending.index) {
                cache.history.record(
                    &frame.board,
                    candidate.chess_move(),
                    frame.ply,
                    frame.depth,
                    false,
                );
            }
            cache
                .history
                .record(&frame.board, chess_move, frame.ply, frame.depth, true);
            return Ok(Action::Complete(NodeResult {
                score,
                pv: frame.pv.clone(),
            }));
        }
        Ok(Action::Keep)
    }

    pub(super) fn finish_pass(&mut self, result: NodeResult) {
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
            let chosen = self.choose();
            if let Some(line) = self.report.variations.get(chosen) {
                self.report.best_move = line.moves.first().copied();
                self.report.score_cp = Some(line.score_cp);
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
        }
    }

    fn choose(&self) -> usize {
        if self.options.mode() == Mode::Analysis {
            return 0;
        }
        let Some(best) = self.report.variations.first() else {
            return 0;
        };
        let tolerance = self.options.loss_tolerance();
        let eligible = self
            .report
            .variations
            .iter()
            .take_while(|line| best.score_cp - line.score_cp <= tolerance)
            .count()
            .max(1);
        let sample = self
            .root
            .key()
            .full()
            .wrapping_add(self.options.seed())
            .rotate_left(19)
            .wrapping_add(u64::from(self.iteration));
        usize::try_from(sample % eligible as u64).unwrap_or(0)
    }
}
