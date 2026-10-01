use crate::engine::{
    frame::{Action, CachePolicy, Frame, Stage},
    table::Cache,
};
mod advance;
mod finish;
mod scoring;

use crate::{
    Completion, EngineError, INFINITY, Options, PrincipalVariation, SearchLimits, SearchReport,
    SearchStatus,
};
use gwaymaegyi_core::{Board, Game, Move, Outcome};
use gwaymaegyi_eval::Accumulator;

#[derive(Debug)]
pub(super) struct Task {
    pub game: Game,
    pub options: Options,
    pub limits: SearchLimits,
    pub frames: Vec<Frame>,
    pub report: SearchReport,
    pub iteration: u8,
    pub lines: Vec<PrincipalVariation>,
    pub excluded: Vec<Move>,
    pub root: Board,
}

impl Task {
    pub(super) fn new(
        game: &Game,
        options: Options,
        limits: SearchLimits,
    ) -> Result<Self, EngineError> {
        let root = *game.board();
        let outcome = game.outcome();
        let best_move = if outcome == Outcome::Ongoing {
            root.legal_moves().first().copied()
        } else {
            None
        };
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
                    let child = Self::child(&frame, depth, window, null)?;
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
        self.frames.push(Frame::new(
            self.root,
            Accumulator::new(&self.root, self.options.model(&self.root)),
            i16::from(self.iteration),
            0,
            [-INFINITY, INFINITY],
            true,
            false,
            context,
        ));
    }

    fn child(
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
        Ok(Frame::new(
            board,
            parent.accumulator.updated(&board),
            depth,
            parent.ply + 1,
            window,
            window[1] - window[0] > 1,
            parent.synthetic || null,
            context,
        ))
    }
}
