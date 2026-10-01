//! Portable controller owning game, options, cache, and an active continuation.
//! Validation precedes state replacement; hosts decide when to step or stop.

#![expect(
    clippy::missing_errors_doc,
    reason = "Errors are documented in plain prose."
)]

use crate::{
    Completion, EngineError, Options, SearchLimits, SearchReport, SearchStatus, Strength,
    TablebaseProbe,
};
mod frame;
mod history;
mod table;
mod task;

use self::{history::PriorMoves, table::Cache, task::Task};
use gwaymaegyi_core::{Game, Outcome};
use gwaymaegyi_eval::Accumulator;
use std::sync::Arc;

pub use self::table::SharedTable;

/// Portable orchestration owns its game, configuration, and active continuation.
#[derive(Debug)]
pub struct Engine {
    game: Game,
    options: Options,
    cache: Cache,
    task: Option<Task>,
    style_loss: i32,
    tablebase: Option<Arc<dyn TablebaseProbe>>,
    worker_id: u8,
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
            tablebase: None,
            worker_id: 0,
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

    /// Attach an optional thread-safe WDL provider and invalidate dependent search state.
    pub fn set_tablebase(&mut self, tablebase: Option<Arc<dyn TablebaseProbe>>) {
        self.task = None;
        self.cache.clear();
        self.tablebase = tablebase;
    }

    /// Attach a lock-striped shared transposition table for single-position parallel search.
    pub fn set_shared_table(&mut self, shared: Option<Arc<SharedTable>>) {
        self.task = None;
        self.cache.set_shared(shared);
    }

    /// Sets the worker identifier (`0` for primary, `1..=15` for helper diversification).
    pub const fn set_worker_id(&mut self, worker_id: u8) {
        self.worker_id = worker_id;
    }

    /// Begin a distinct game and release game-specific search history.
    pub fn new_game(&mut self) -> Result<(), EngineError> {
        let game =
            Game::start().map_err(|error| EngineError::InvalidPosition(error.to_string()))?;
        self.game = game;
        self.task = None;
        self.cache.clear();
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
        self.cache.clear_entries();
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
        let mut task = Task::new(
            &self.game,
            self.options,
            limits,
            &allowed,
            self.style_loss,
            self.tablebase.clone(),
        )?;
        task.worker_id = self.worker_id;
        if task.report.status == SearchStatus::Running
            && matches!(self.options.strength(), Strength::Full)
            && self.options.mode() != crate::Mode::Human
            && self.options.multi_pv() == 1
        {
            if let Some(root) = self
                .tablebase
                .as_ref()
                .and_then(|table| table.probe_root(self.game.board(), &allowed))
                .filter(|root| {
                    (allowed.is_empty() || allowed.contains(&root.best_move))
                        && self.game.board().legal_moves().contains(&root.best_move)
                })
            {
                let score = match root.wdl {
                    crate::TablebaseWdl::Win => 29_000,
                    crate::TablebaseWdl::Loss => -29_000,
                    crate::TablebaseWdl::Draw
                    | crate::TablebaseWdl::CursedWin
                    | crate::TablebaseWdl::BlessedLoss => 0,
                };
                task.report.status = SearchStatus::Finished(Completion::Tablebase);
                task.report.best_move = Some(root.best_move);
                task.report.score_cp = Some(score);
                task.report.tablebase_hits = 1;
                task.report.variations = vec![crate::PrincipalVariation {
                    score_cp: score,
                    moves: vec![root.best_move],
                }];
            }
        }
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
    /// accumulated mistake budget once per move, under the selected playing policy.
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
        let side = board.side_to_move();
        let balance = board.material(side) - board.material(side.opposite());
        let sacrifice = Options::detect_sacrifice(self.game.material_history(), balance);
        let static_score = self
            .options
            .evaluate_with_sacrifice(board, &state, board, sacrifice);
        let corrected = static_score
            + self.options.tuning().get(crate::Parameter::CorrWeight)
                * self.cache.history.correction(board, PriorMoves::default())
                / 512;
        corrected.clamp(-28_000, 28_000)
    }
}

#[cfg(test)]
mod tests {
    use super::Engine;
    use crate::engine::history::PriorMoves;
    use gwaymaegyi_core::START_FEN;
    use std::error::Error;

    #[test]
    fn correction_history_survives_position_updates_and_resets_on_new_game()
    -> Result<(), Box<dyn Error>> {
        let mut engine = Engine::new()?;
        let fen = START_FEN;
        let before = engine.evaluate();
        let board = *engine.game().board();
        engine
            .cache
            .history
            .update_correction(&board, PriorMoves::default(), 400, before, 8);
        engine.cache.store(
            board.key().full(),
            1,
            0,
            0,
            None,
            super::table::Bound::Exact,
        );
        let corrected = engine.evaluate();
        assert!(corrected > before);
        engine.set_position(fen, &[])?;
        assert!(engine.cache.probe(board.key().full()).is_none());
        assert_eq!(engine.evaluate(), corrected);
        engine.new_game()?;
        assert_eq!(engine.evaluate(), before);
        Ok(())
    }
}
