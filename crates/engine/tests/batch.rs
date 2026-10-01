//! Verify native independent-request parallelism, cancellation, and input ordering.

use gwaymaegyi::{AnalysisRequest, Options, SearchLimits, SearchStatus, analyze_batch};
use gwaymaegyi_core::START_FEN;
use std::{error::Error, sync::atomic::AtomicBool};

#[test]
fn worker_count_does_not_change_independent_results() -> Result<(), Box<dyn Error>> {
    let cases = vec![
        AnalysisRequest {
            fen: START_FEN.into(),
            moves: vec![],
            roots: vec![],
            options: Options::default(),
            limits: SearchLimits {
                depth: 3,
                nodes: 20_000,
            },
        },
        AnalysisRequest {
            fen: "7k/8/6KQ/8/8/8/8/8 w - - 0 1".into(),
            moves: vec![],
            roots: vec![],
            options: Options::default(),
            limits: SearchLimits {
                depth: 2,
                nodes: 10_000,
            },
        },
    ];
    let cancel = AtomicBool::new(false);
    assert_eq!(
        analyze_batch(&cases, 1, &cancel)?,
        analyze_batch(&cases, 2, &cancel)?
    );
    assert!(analyze_batch(&cases, 0, &cancel).is_err());
    let stopped = analyze_batch(&cases, 2, &AtomicBool::new(true))?;
    assert!(stopped.iter().all(|report| report.status
        == SearchStatus::Finished(gwaymaegyi_search::Completion::Stopped)
        && report.best_move.is_some()));
    Ok(())
}
