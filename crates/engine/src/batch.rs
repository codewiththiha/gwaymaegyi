//! Bounded native CPU parallelism for independent analysis requests.
//! Each request owns fresh search resources; results retain input ordering.

#![expect(clippy::missing_errors_doc, reason = "Batch failures use plain prose.")]

use gwaymaegyi_search::{Engine, EngineError, Options, SearchLimits, SearchReport};
use std::{
    error::Error,
    fmt,
    sync::atomic::{AtomicBool, Ordering},
    thread,
};

#[derive(Clone, Debug)]
pub struct AnalysisRequest {
    pub fen: String,
    pub moves: Vec<String>,
    pub roots: Vec<String>,
    pub options: Options,
    pub limits: SearchLimits,
}

#[derive(Clone, Debug)]
pub enum BatchError {
    InvalidWorkers,
    MemoryBudget,
    Engine(EngineError),
    WorkerFailure,
}
impl fmt::Display for BatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkers => f.write_str("worker count must be 1 through 16"),
            Self::MemoryBudget => f.write_str("combined per-worker hash budget exceeds 256 MiB"),
            Self::Engine(error) => error.fmt(f),
            Self::WorkerFailure => f.write_str("native analysis worker failed"),
        }
    }
}
impl Error for BatchError {}

/// Cancellation is checked between bounded slices, not through async-task abortion.
pub fn analyze_batch(
    requests: &[AnalysisRequest],
    workers: u8,
    cancel: &AtomicBool,
) -> Result<Vec<SearchReport>, BatchError> {
    if !(1..=16).contains(&workers) {
        return Err(BatchError::InvalidWorkers);
    }
    for request in requests {
        request.limits.validate().map_err(BatchError::Engine)?;
        let mut game = gwaymaegyi_core::Game::new(request.fen.parse().map_err(
            |error: gwaymaegyi_core::FenError| {
                BatchError::Engine(EngineError::InvalidPosition(error.to_string()))
            },
        )?);
        for notation in &request.moves {
            game.play_uci(notation, request.options.chess960())
                .map_err(|error| BatchError::Engine(EngineError::InvalidMove(error.to_string())))?;
        }
        for notation in &request.roots {
            game.board()
                .resolve_uci(notation, request.options.chess960())
                .map_err(|error| BatchError::Engine(EngineError::InvalidMove(error.to_string())))?;
        }
    }
    let count = usize::from(workers).min(requests.len());
    let hash = requests
        .iter()
        .map(|request| request.options.hash_mib())
        .max()
        .unwrap_or(0);
    if usize::from(hash) * count > 256 {
        return Err(BatchError::MemoryBudget);
    }
    if requests.is_empty() {
        return Ok(Vec::new());
    }
    thread::scope(|scope| {
        let mut handles = Vec::new();
        for worker in 0..count {
            handles.push(scope.spawn(move || {
                let mut reports = Vec::new();
                for index in (worker..requests.len()).step_by(count) {
                    let request = &requests[index];
                    let mut engine = Engine::new().map_err(BatchError::Engine)?;
                    engine
                        .configure(request.options)
                        .map_err(BatchError::Engine)?;
                    let moves: Vec<_> = request.moves.iter().map(String::as_str).collect();
                    let roots: Vec<_> = request.roots.iter().map(String::as_str).collect();
                    engine
                        .set_position(&request.fen, &moves)
                        .map_err(BatchError::Engine)?;
                    engine
                        .start_moves(request.limits, &roots)
                        .map_err(BatchError::Engine)?;
                    while engine.searching() {
                        if cancel.load(Ordering::Relaxed) {
                            engine.stop();
                            break;
                        }
                        engine.step(256).map_err(BatchError::Engine)?;
                    }
                    reports.push((index, engine.report()));
                }
                Ok::<_, BatchError>(reports)
            }));
        }
        let mut indexed = Vec::new();
        let mut failure = None;
        for handle in handles {
            match handle.join() {
                Ok(Ok(reports)) => indexed.extend(reports),
                Ok(Err(error)) => {
                    if failure.is_none() {
                        failure = Some(error);
                    }
                }
                Err(_) => {
                    if failure.is_none() {
                        failure = Some(BatchError::WorkerFailure);
                    }
                }
            }
        }
        if let Some(error) = failure {
            return Err(error);
        }
        indexed.sort_by_key(|(index, _)| *index);
        Ok(indexed.into_iter().map(|(_, report)| report).collect())
    })
}
