//! Bounded native CPU parallelism for batch and single-position SMP analysis.
//! Each worker owns isolated search state while SMP workers share a lock-striped table.

#![expect(clippy::missing_errors_doc, reason = "Batch failures use plain prose.")]

use gwaymaegyi_search::{
    Engine, EngineError, Options, SearchLimits, SearchReport, SharedTable, TablebaseProbe,
};
use std::{
    error::Error,
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

#[derive(Clone, Debug)]
pub struct AnalysisRequest {
    pub fen: String,
    pub moves: Vec<String>,
    pub roots: Vec<String>,
    pub options: Options,
    pub limits: SearchLimits,
    pub tablebase: Option<Arc<dyn TablebaseProbe>>,
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

fn validate_request(request: &AnalysisRequest) -> Result<(), BatchError> {
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
    Ok(())
}

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
        validate_request(request)?;
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
                    let report = run_single_worker(&requests[index], 0, None, None, cancel)?;
                    reports.push((index, report));
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

/// Runs a single-position parallel search across `threads` workers sharing a transposition table.
pub fn analyze_parallel(
    request: &AnalysisRequest,
    threads: u8,
    cancel: &AtomicBool,
) -> Result<SearchReport, BatchError> {
    if !(1..=16).contains(&threads) {
        return Err(BatchError::InvalidWorkers);
    }
    validate_request(request)?;
    let total_hash =
        usize::from(request.options.hash_mib()) + usize::from(threads.saturating_sub(1));
    if total_hash > 256 {
        return Err(BatchError::MemoryBudget);
    }
    if threads == 1 {
        return run_single_worker(request, 0, None, None, cancel);
    }
    let table = SharedTable::new(request.options.hash_mib()).map_err(BatchError::Engine)?;
    let shared = Arc::new(table);
    let done = AtomicBool::new(false);
    thread::scope(|scope| {
        let mut handles = Vec::with_capacity(usize::from(threads));
        for worker_id in 0..threads {
            let shared_ref = Arc::clone(&shared);
            let done_ref = &done;
            handles.push(scope.spawn(move || {
                let report = run_single_worker(
                    request,
                    worker_id,
                    Some(shared_ref),
                    Some(done_ref),
                    cancel,
                )?;
                if worker_id == 0 {
                    done_ref.store(true, Ordering::Relaxed);
                }
                Ok::<_, BatchError>((worker_id, report))
            }));
        }
        let mut primary: Option<SearchReport> = None;
        let mut total_nodes = 0_u64;
        let mut total_tb_hits = 0_u64;
        let mut failure = None;
        for handle in handles {
            match handle.join() {
                Ok(Ok((worker_id, report))) => {
                    total_nodes = total_nodes.saturating_add(report.nodes);
                    total_tb_hits = total_tb_hits.saturating_add(report.tablebase_hits);
                    if worker_id == 0 {
                        primary = Some(report);
                    }
                }
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
        let mut report = primary.ok_or(BatchError::WorkerFailure)?;
        report.nodes = total_nodes;
        report.tablebase_hits = total_tb_hits;
        Ok(report)
    })
}

fn run_single_worker(
    request: &AnalysisRequest,
    worker_id: u8,
    shared: Option<Arc<SharedTable>>,
    done: Option<&AtomicBool>,
    cancel: &AtomicBool,
) -> Result<SearchReport, BatchError> {
    let mut engine = Engine::new().map_err(BatchError::Engine)?;
    engine.set_worker_id(worker_id);
    engine.set_tablebase(request.tablebase.clone());
    let mut worker_options = request.options;
    if worker_id > 0 {
        worker_options.set_hash_mib(1).map_err(BatchError::Engine)?;
    }
    engine
        .configure(worker_options)
        .map_err(BatchError::Engine)?;
    engine.set_shared_table(shared);
    let moves: Vec<_> = request.moves.iter().map(String::as_str).collect();
    let roots: Vec<_> = request.roots.iter().map(String::as_str).collect();
    engine
        .set_position(&request.fen, &moves)
        .map_err(BatchError::Engine)?;
    engine
        .start_moves(request.limits, &roots)
        .map_err(BatchError::Engine)?;
    while engine.searching() {
        let primary_done = worker_id > 0 && done.is_some_and(|flag| flag.load(Ordering::Relaxed));
        if cancel.load(Ordering::Relaxed) || primary_done {
            engine.stop();
            break;
        }
        engine.step(256).map_err(BatchError::Engine)?;
    }
    Ok(engine.report())
}
