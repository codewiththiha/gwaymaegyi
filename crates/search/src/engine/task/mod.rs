//! Iterative search task retaining its exact node stack across work slices.

use crate::engine::{
    frame::{Action, CachePolicy, Frame, Stage},
    table::Cache,
};
mod advance;
mod finish;
mod scoring;
mod window;
use window::Window;

use crate::{
    Completion, EngineError, Options, PrincipalVariation, SearchLimits, SearchReport, SearchStatus,
};
use gwaymaegyi_core::{Board, Game, Move, Outcome};
use gwaymaegyi_eval::Accumulator;

#[derive(Debug)]
pub(super) struct Task {
    pub game: Game,
    pub options: Options,
    pub limits: SearchLimits,
    pub requested_limits: SearchLimits,
    pub frames: Vec<Frame>,
    pub report: SearchReport,
    pub iteration: u8,
    pub lines: Vec<PrincipalVariation>,
    pub excluded: Vec<Move>,
    pub root: Board,
    pub root_moves: Vec<Move>,
    scope: u64,
    window: Option<Window>,
}

impl Task {
    pub(super) fn new(
        game: &Game,
        options: Options,
        limits: SearchLimits,
        allowed: &[Move],
    ) -> Result<Self, EngineError> {
        let root = *game.board();
        let outcome = game.outcome();
        let root_moves = if outcome != Outcome::Ongoing {
            Vec::new()
        } else if allowed.is_empty() {
            root.legal_moves()
        } else {
            allowed.to_vec()
        };
        let best_move = root_moves.first().copied();
        let (status, score_cp) = match outcome {
            Outcome::Ongoing => (SearchStatus::Running, None),
            Outcome::Checkmate { .. } => (
                SearchStatus::Finished(Completion::Terminal),
                Some(-crate::MATE),
            ),
            Outcome::Draw(_) => (SearchStatus::Finished(Completion::Terminal), Some(0)),
        };
        let mut frames = Vec::new();
        frames
            .try_reserve_exact(usize::from(crate::MAX_PLY))
            .map_err(|_| EngineError::Resources)?;
        Ok(Self {
            game: game.clone(),
            options,
            limits: options.limit(limits),
            requested_limits: limits,
            frames,
            report: SearchReport {
                status,
                best_move,
                score_cp,
                ..SearchReport::default()
            },
            iteration: 1,
            lines: Vec::new(),
            excluded: Vec::new(),
            root,
            root_moves,
            window: None,
            scope: root.key().full().rotate_left(33)
                ^ (options.model(&root) as u64 + 1).wrapping_mul(0xd6e8_feb8_6659_fd93),
        })
    }

    pub(super) fn step(&mut self, cache: &mut Cache, work: u32) -> Result<(), EngineError> {
        for _ in 0..work {
            if self.report.status != SearchStatus::Running {
                break;
            }
            if self.report.nodes >= self.limits.nodes {
                self.stop(Completion::Nodes);
                break;
            }
            if self.frames.is_empty() {
                self.start_pass();
            }
            let Some(mut frame) = self.frames.pop() else {
                return Err(EngineError::InternalState);
            };
            let action = self.advance(&mut frame, cache)?;
            match action {
                Action::Keep => self.frames.push(frame),
                Action::Descend {
                    depth,
                    window,
                    null,
                } => {
                    let child = self.child(&frame, depth, window, null)?;
                    self.frames.push(frame);
                    self.frames.push(child);
                }
                Action::Complete(result) => {
                    if frame.cache_policy == CachePolicy::Write {
                        cache.store(
                            frame.key,
                            frame.depth.max(0),
                            frame.ply,
                            result.score,
                            frame.best_move,
                            frame.bound(result.score),
                        );
                    }
                    if let Some(parent) = self.frames.last_mut() {
                        let Stage::Waiting(pending) = parent.stage else {
                            return Err(EngineError::InternalState);
                        };
                        parent.stage = Stage::Returned(pending, result);
                    } else {
                        self.finish_pass(result);
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn update_limits(&mut self, limits: SearchLimits) {
        self.requested_limits = limits;
        self.limits = self.options.limit(limits);
        if self.report.nodes >= self.limits.nodes {
            self.stop(Completion::Nodes);
        } else if self.report.depth >= self.limits.depth {
            self.stop(Completion::Depth);
        }
    }

    pub(super) fn stop(&mut self, reason: Completion) {
        self.frames.clear();
        self.report.status = SearchStatus::Finished(reason);
    }

    fn start_pass(&mut self) {
        let context = self
            .game
            .position_history()
            .iter()
            .fold(0_u64, |sum, key| sum.wrapping_add(key.rotate_left(7)));
        let tuning = self.options.tuning();
        let guess = if tuning.enabled(crate::Behavior::Aspiration)
            && i32::from(self.iteration) >= tuning.get(crate::Parameter::AspirationDepth)
        {
            self.report
                .variations
                .get(self.lines.len())
                .map(|line| line.score_cp)
        } else {
            None
        };
        let window = self
            .window
            .get_or_insert_with(|| {
                Window::new(guess, tuning.get(crate::Parameter::AspirationWindow))
            })
            .bounds;
        self.frames.push(Frame::new(
            self.root,
            Accumulator::new(&self.root, self.options.model(&self.root)),
            i16::from(self.iteration),
            0,
            window,
            true,
            false,
            context,
            self.scope,
        ));
    }

    fn child(
        &self,
        parent: &Frame,
        depth: i16,
        window: [i32; 2],
        null: bool,
    ) -> Result<Frame, EngineError> {
        let Stage::Waiting(pending) = parent.stage else {
            return Err(EngineError::InternalState);
        };
        let board = if null {
            parent
                .board
                .null_position()
                .ok_or(EngineError::InternalState)?
        } else {
            *parent
                .candidates
                .get(pending.index)
                .ok_or(EngineError::InternalState)?
                .board()
        };
        let context = if board.halfmove_clock() == 0 {
            board.key().full().rotate_left(7)
        } else {
            parent
                .context
                .wrapping_add(board.key().full().rotate_left(7))
        };
        let model = self.options.model(&board);
        let accumulator = if model == parent.accumulator.model() {
            parent.accumulator.updated(&board)
        } else {
            Accumulator::new(&board, model)
        };
        Ok(Frame::new(
            board,
            accumulator,
            depth,
            parent.ply + 1,
            window,
            window[1] - window[0] > 1,
            parent.synthetic || null,
            context,
            self.scope,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::Task;
    use crate::{
        EngineError, Options, SearchLimits,
        engine::{frame::CachePolicy, table::Cache},
    };
    use gwaymaegyi_core::Game;
    use std::error::Error;

    #[test]
    fn restricted_root_bounds_never_enter_the_full_position_cache() -> Result<(), Box<dyn Error>> {
        let game = Game::start()?;
        let mut task = Task::new(&game, Options::default(), SearchLimits::default(), &[])?;
        let mut cache = Cache::new(1)?;
        task.start_pass();
        let mut frame = task.frames.pop().ok_or(EngineError::InternalState)?;
        task.advance(&mut frame, &mut cache)?;
        assert_eq!(frame.cache_policy, CachePolicy::Write);
        task.excluded.push(game.board().resolve_uci("e2e4", false)?);
        task.start_pass();
        let mut excluded = task.frames.pop().ok_or(EngineError::InternalState)?;
        task.advance(&mut excluded, &mut cache)?;
        assert_eq!(excluded.cache_policy, CachePolicy::Skip);
        Ok(())
    }
    #[test]
    fn material_phase_changes_refresh_the_selected_model() -> Result<(), Box<dyn Error>> {
        let game = Game::new("r3k3/4q3/8/8/8/8/q7/R2Q2KR w - - 0 1".parse()?);
        let root_move = game.board().resolve_uci("a1a2", false)?;
        let mut task = Task::new(
            &game,
            Options::default(),
            SearchLimits::default(),
            &[root_move],
        )?;
        task.start_pass();
        let mut parent = task.frames.pop().ok_or(EngineError::InternalState)?;
        let mut cache = Cache::new(1)?;
        task.advance(&mut parent, &mut cache)?;
        task.advance(&mut parent, &mut cache)?;
        let child = task.child(&parent, 0, [-crate::INFINITY, crate::INFINITY], false)?;
        assert_eq!(parent.accumulator.model(), gwaymaegyi_eval::Model::Balanced);
        assert_eq!(child.accumulator.model(), gwaymaegyi_eval::Model::Endgame);
        assert_eq!(
            child.accumulator.score(),
            gwaymaegyi_eval::Accumulator::new(&child.board, gwaymaegyi_eval::Model::Endgame)
                .score()
        );
        Ok(())
    }
}
