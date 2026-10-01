//! Portable controller owning game, options, cache, and an active continuation.
//! Validation precedes state replacement; hosts decide when to step or stop.

#![expect(
    clippy::missing_errors_doc,
    reason = "Errors are documented in plain prose."
)]

use crate::{Completion, EngineError, Options, SearchLimits, SearchReport, SearchStatus, Strength};
mod frame;
mod history;
mod table;
mod task;

use self::{table::Cache, task::Task};
use gwaymaegyi_core::{Game, Outcome};
use gwaymaegyi_eval::Accumulator;

/// Portable orchestration owns its game, configuration, and active continuation.
#[derive(Debug)]
pub struct Engine {
    game: Game,
    options: Options,
    cache: Cache,
    task: Option<Task>,
    style_loss: i32,
}

impl Engine {
    /// Reports an allocation failure or an invalid built-in initial position.
    pub fn new() -> Result<Self, EngineError> {
        let options = Options::default();
        Ok(Self {
            game: Game::start().map_err(|error| EngineError::InvalidPosition(error.to_string()))?,
            options,
            cache: Cache::new(options.hash_mib())?,
            task: None,
            style_loss: 0,
        })
    }
    #[must_use]
    pub const fn game(&self) -> &Game {
        &self.game
    }
    #[must_use]
    pub const fn options(&self) -> Options {
        self.options
    }
    #[must_use]
    pub fn outcome(&self) -> Outcome {
        self.game.outcome()
    }
    #[must_use]
    pub fn report(&self) -> SearchReport {
        self.task
            .as_ref()
            .map_or_else(SearchReport::default, |task| task.report.clone())
    }
    #[must_use]
    pub fn searching(&self) -> bool {
        self.task
            .as_ref()
            .is_some_and(|task| task.report.status == SearchStatus::Running)
    }

    /// Configuration commits atomically; invalid allocations keep the old session intact.
    pub fn configure(&mut self, options: Options) -> Result<(), EngineError> {
        if options.hash_mib() == self.options.hash_mib() {
            self.cache.clear();
        } else {
            self.cache = Cache::new(options.hash_mib())?;
        }
        self.task = None;
        self.options = options;
        self.style_loss = 0;
        Ok(())
    }

    /// The complete position/move list is validated before replacing the current game.
    pub fn set_position(&mut self, fen: &str, moves: &[&str]) -> Result<(), EngineError> {
        let mut game = Game::new(fen.parse().map_err(|error: gwaymaegyi_core::FenError| {
            EngineError::InvalidPosition(error.to_string())
        })?);
        for notation in moves {
            game.play_uci(notation, self.options.chess960())
                .map_err(|error| EngineError::InvalidMove(error.to_string()))?;
        }
        self.game = game;
        self.task = None;
        self.cache.clear();
        self.style_loss = 0;
        Ok(())
    }

    /// Illegal moves leave the game and active continuation unchanged.
    pub fn play_uci(&mut self, notation: &str) -> Result<(), EngineError> {
        self.game
            .play_uci(notation, self.options.chess960())
            .map_err(|error| EngineError::InvalidMove(error.to_string()))?;
        self.apply_style_budget();
        Ok(())
    }

    /// Invalid limits do not interrupt a currently valid search.
    /// Invalid limits do not interrupt a currently valid search.
    pub fn start(&mut self, limits: SearchLimits) -> Result<(), EngineError> {
        self.start_moves(limits, &[])
    }

    /// Root notation is validated completely before replacing an active continuation.
    pub fn start_moves(&mut self, limits: SearchLimits, roots: &[&str]) -> Result<(), EngineError> {
        let limits = limits.validate()?;
        let mut allowed = Vec::new();
        for notation in roots {
            let chess_move = self
                .game
                .board()
                .resolve_uci(notation, self.options.chess960())
                .map_err(|error| EngineError::InvalidMove(error.to_string()))?;
            if !allowed.contains(&chess_move) {
                allowed.push(chess_move);
            }
        }
        self.apply_style_budget();
        let task = Task::new(&self.game, self.options, limits, &allowed, self.style_loss)?;
        self.cache.next_search();
        self.task = Some(task);
        Ok(())
    }

    #[must_use]
    pub fn requested_limits(&self) -> Option<SearchLimits> {
        self.task.as_ref().map(|task| task.requested_limits)
    }

    #[must_use]
    pub fn effective_limits(&self) -> Option<SearchLimits> {
        self.task.as_ref().map(|task| task.limits)
    }

    /// Valid budget changes retain the active continuation; exhausted caps stop it immediately.
    pub fn set_limits(&mut self, limits: SearchLimits) -> Result<(), EngineError> {
        let limits = limits.validate()?;
        let task = self
            .task
            .as_mut()
            .filter(|task| task.report.status == SearchStatus::Running)
            .ok_or(EngineError::NoActiveSearch)?;
        task.update_limits(limits);
        Ok(())
    }

    /// A work unit advances one state transition; the continuation is retained exactly.
    pub fn step(&mut self, work: u32) -> Result<SearchReport, EngineError> {
        if !(1..=65_536).contains(&work) {
            return Err(EngineError::InvalidSlice);
        }
        if let Some(task) = self.task.as_mut() {
            task.step(&mut self.cache, work)?;
        }
        Ok(self.report())
    }
    pub fn stop(&mut self) {
        if let Some(task) = self.task.as_mut() {
            if task.report.status == SearchStatus::Running {
                task.stop(Completion::Stopped);
            }
        }
        self.apply_style_budget();
    }

    /// Carry the finished search's style state forward and update the
    /// accumulated mistake budget once per move, as in the reference policy.
    fn apply_style_budget(&mut self) {
        let Some(task) = self.task.as_mut() else {
            return;
        };
        if task.style_applied {
            return;
        }
        task.style_applied = true;
        self.style_loss = task.style_loss;
        let ply = i32::try_from(self.game.position_history().len()).unwrap_or(0) - 1;
        if matches!(task.options.strength(), Strength::Approximate(_)) {
            if let Some(mistake) = task.last_mistake {
                self.style_loss -= mistake;
            }
            if ply >= 7 {
                self.style_loss += task.options.cp_loss();
            }
        }
    }

    #[must_use]
    pub fn evaluate(&self) -> i32 {
        let board = self.game.board();
        let state = Accumulator::new(board, self.options.model(board));
        self.options.evaluate(board, &state, board)
    }
}
