//! Native worker ranges, unrestricted aggregate hash budgets, and datagen resource validation.

use gwaymaegyi::{
    AnalysisRequest, DatagenConfig, MAX_DEPTH, MAX_HASH_MIB, MAX_THREADS, Options, SearchLimits,
    SearchReport, TrainingRecord, analyze_batch, analyze_parallel, generate_training_data,
};
use gwaymaegyi_core::START_FEN;
use std::{error::Error, sync::atomic::AtomicBool};

fn request(options: Options) -> AnalysisRequest {
    AnalysisRequest {
        fen: START_FEN.to_owned(),
        moves: Vec::new(),
        roots: Vec::new(),
        options,
        limits: SearchLimits {
            depth: 1,
            nodes: 20_000,
        },
        tablebase: None,
    }
}

#[test]
fn worker_counts_above_16_are_supported() -> Result<(), Box<dyn Error>> {
    let cancel = AtomicBool::new(false);
    assert_eq!(MAX_THREADS, 1024);
    assert_eq!(
        analyze_batch(&[], MAX_THREADS, &cancel)?,
        Vec::<SearchReport>::new()
    );
    assert!(analyze_batch(&[], MAX_THREADS + 1, &cancel).is_err());
    let mut options = Options::default();
    options.set_hash_mib(1)?;
    let position = request(options);
    assert!(analyze_parallel(&position, MAX_THREADS + 1, &cancel).is_err());
    let report = analyze_parallel(&position, 17, &cancel)?;
    assert!(report.best_move.is_some());
    assert_eq!(report.depth, 1);
    Ok(())
}

#[test]
fn batch_hash_budget_is_not_capped_at_256_mib() -> Result<(), Box<dyn Error>> {
    let mut options = Options::default();
    options.set_hash_mib(65)?;
    let positions = vec![request(options); 4];
    let reports = analyze_batch(&positions, 4, &AtomicBool::new(true))?;
    assert_eq!(reports.len(), 4);
    assert!(reports.iter().all(|report| report.best_move.is_some()));
    Ok(())
}

#[test]
fn large_datagen_runs_use_native_ranges_and_cancel_before_allocating() -> Result<(), Box<dyn Error>>
{
    let mut config = DatagenConfig {
        positions: 100_001,
        threads: MAX_THREADS,
        hash_mib: MAX_HASH_MIB,
        max_depth: MAX_DEPTH,
        ..DatagenConfig::default()
    };
    let cancel = AtomicBool::new(true);
    assert_eq!(
        generate_training_data(&config, &cancel)?,
        Vec::<TrainingRecord>::new()
    );
    config.positions = usize::MAX;
    assert_eq!(
        generate_training_data(&config, &cancel)?,
        Vec::<TrainingRecord>::new()
    );
    config.threads = MAX_THREADS + 1;
    assert!(generate_training_data(&config, &cancel).is_err());
    config.threads = 1;
    config.hash_mib = MAX_HASH_MIB + 1;
    assert!(generate_training_data(&config, &cancel).is_err());
    config.hash_mib = 1;
    config.max_depth = MAX_DEPTH + 1;
    assert!(generate_training_data(&config, &cancel).is_err());
    Ok(())
}
