//! Native batch and single-position SMP analysis with caller-selected CPU resources.
//! Each worker owns isolated search state while SMP workers share a lock-striped table.

#![expect(clippy::missing_errors_doc, reason = "Batch failures use plain prose.")]

use crate::MAX_THREADS;
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
    Engine(EngineError),
    WorkerFailure,
}

impl fmt::Display for BatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkers => write!(f, "worker count must be 1 through {MAX_THREADS}"),
            Self::Engine(error) => error.fmt(f),
            Self::WorkerFailure => f.write_str("native analysis worker could not start or failed"),
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
    workers: u16,
    cancel: &AtomicBool,
) -> Result<Vec<SearchReport>, BatchError> {
    if !(1..=MAX_THREADS).contains(&workers) {
        return Err(BatchError::InvalidWorkers);
    }
    for request in requests {
        validate_request(request)?;
    }
    if requests.is_empty() {
        return Ok(Vec::new());
    }
    let count = usize::from(workers).min(requests.len());
    let failed = AtomicBool::new(false);
    thread::scope(|scope| {
        let mut handles = Vec::with_capacity(count);
        let mut failure = None;
        for worker in 0..count {
            let failed_ref = &failed;
            let started = thread::Builder::new().spawn_scoped(scope, move || {
                let result = (|| {
                    let mut reports = Vec::new();
                    for index in (worker..requests.len()).step_by(count) {
                        let report =
                            run_single_worker(&requests[index], 0, None, Some(failed_ref), cancel)?;
                        reports.push((index, report));
                    }
                    Ok::<_, BatchError>(reports)
                })();
                if result.is_err() {
                    failed_ref.store(true, Ordering::Relaxed);
                }
                result
            });
            let Ok(handle) = started else {
                failed.store(true, Ordering::Relaxed);
                failure = Some(BatchError::WorkerFailure);
                break;
            };
            handles.push(handle);
        }
        let mut indexed = Vec::new();
        for handle in handles {
            match handle.join() {
                Ok(Ok(reports)) => indexed.extend(reports),
                Ok(Err(error)) => {
                    failure.get_or_insert(error);
                }
                Err(_) => {
                    failed.store(true, Ordering::Relaxed);
                    failure.get_or_insert(BatchError::WorkerFailure);
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
    threads: u16,
    cancel: &AtomicBool,
) -> Result<SearchReport, BatchError> {
    if !(1..=MAX_THREADS).contains(&threads) {
        return Err(BatchError::InvalidWorkers);
    }
    validate_request(request)?;
    if threads == 1 {
        return run_single_worker(request, 0, None, None, cancel);
    }
    let table = SharedTable::new(request.options.hash_mib()).map_err(BatchError::Engine)?;
    let shared = Arc::new(table);
    let done = AtomicBool::new(false);
    thread::scope(|scope| {
        let mut handles = Vec::with_capacity(usize::from(threads));
        let mut failure = None;
        for worker_id in 0..threads {
            let shared_ref = Arc::clone(&shared);
            let done_ref = &done;
            let started = thread::Builder::new().spawn_scoped(scope, move || {
                let result =
                    run_single_worker(request, worker_id, Some(shared_ref), Some(done_ref), cancel);
                if worker_id == 0 || result.is_err() {
                    done_ref.store(true, Ordering::Relaxed);
                }
                result.map(|report| (worker_id, report))
            });
            let Ok(handle) = started else {
                done.store(true, Ordering::Relaxed);
                failure = Some(BatchError::WorkerFailure);
                break;
            };
            handles.push(handle);
        }
        let mut primary: Option<SearchReport> = None;
        let mut total_nodes = 0_u64;
        let mut total_tb_hits = 0_u64;
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
                    failure.get_or_insert(error);
                }
                Err(_) => {
                    done.store(true, Ordering::Relaxed);
                    failure.get_or_insert(BatchError::WorkerFailure);
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
    worker_id: u16,
    shared: Option<Arc<SharedTable>>,
    done: Option<&AtomicBool>,
    cancel: &AtomicBool,
) -> Result<SearchReport, BatchError> {
    let mut worker_options = request.options;
    if shared.is_some() {
        worker_options.set_hash_mib(1).map_err(BatchError::Engine)?;
    }
    let mut engine = Engine::with_options(worker_options).map_err(BatchError::Engine)?;
    engine.set_worker_id(worker_id);
    engine.set_tablebase(request.tablebase.clone());
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
        if cancel.load(Ordering::Relaxed) || done.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            engine.stop();
            break;
        }
        engine.step(256).map_err(BatchError::Engine)?;
    }
    Ok(engine.report())
}
