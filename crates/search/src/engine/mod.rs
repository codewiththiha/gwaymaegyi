#![expect(
    clippy::missing_errors_doc,
    reason = "Errors are documented in plain prose."
)]

use crate::{Completion, EngineError, Options, SearchLimits, SearchReport, SearchStatus};
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
        Ok(())
    }

    /// Illegal moves leave the game and active continuation unchanged.
    pub fn play_uci(&mut self, notation: &str) -> Result<(), EngineError> {
        self.game
            .play_uci(notation, self.options.chess960())
            .map_err(|error| EngineError::InvalidMove(error.to_string()))?;
        self.task = None;
        Ok(())
    }

    /// Invalid limits do not interrupt a currently valid search.
    pub fn start(&mut self, limits: SearchLimits) -> Result<(), EngineError> {
        let limits = limits.validate()?;
        let task = Task::new(&self.game, self.options, limits)?;
        self.cache.next_search();
        self.task = Some(task);
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
    }

    #[must_use]
    pub fn evaluate(&self) -> i32 {
        Accumulator::new(self.game.board(), self.options.model(self.game.board())).score() * 100
            / 195
    }
}
