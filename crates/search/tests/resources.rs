//! Native resource ranges, large MultiPV output, and searches beyond browser ceilings.

#![cfg(not(target_family = "wasm"))]

use gwaymaegyi_core::START_FEN;
use gwaymaegyi_search::{
    Completion, Engine, EngineError, MAX_DEPTH, MAX_HASH_MIB, MAX_MULTI_PV, MAX_WORK, Mode,
    Options, SearchLimits, SearchReport, SearchStatus, SharedTable, Strength,
};
use std::{error::Error, sync::Arc};

fn finish(engine: &mut Engine) -> Result<SearchReport, Box<dyn Error>> {
    for _ in 0..10_000 {
        let report = engine.step(4096)?;
        if !engine.searching() {
            return Ok(report);
        }
    }
    Err("native resource regression did not terminate".into())
}

#[test]
fn native_options_accept_the_full_ranges_without_allocating_them() -> Result<(), Box<dyn Error>> {
    assert_eq!(MAX_DEPTH, 127);
    assert_eq!(MAX_HASH_MIB, 131_072);
    assert_eq!(MAX_MULTI_PV, 255);
    assert_eq!(MAX_WORK, u32::MAX);
    assert_eq!(SearchLimits::default(), SearchLimits::full());
    let mut options = Options::default();
    assert_eq!(options.hash_mib(), 32);
    assert_eq!(options.multi_pv(), 1);
    assert_eq!(options.strength(), Strength::Full);
    options.set_hash_mib(MAX_HASH_MIB)?;
    options.set_multi_pv(MAX_MULTI_PV)?;
    let before = options;
    assert_eq!(options.set_hash_mib(MAX_HASH_MIB + 1), Err(EngineError::InvalidHash));
    assert_eq!(options.set_hash_mib(0), Err(EngineError::InvalidHash));
    assert_eq!(options.set_multi_pv(0), Err(EngineError::InvalidMultiPv));
    assert_eq!(options, before);
    SearchLimits::full().validate()?;
    assert_eq!(
        SearchLimits { depth: MAX_DEPTH + 1, nodes: u64::MAX }.validate(),
        Err(EngineError::InvalidLimits)
    );
    Ok(())
}

#[test]
fn private_and_shared_hash_tables_accept_more_than_64_mib() -> Result<(), Box<dyn Error>> {
    let mut options = Options::default();
    options.set_hash_mib(65)?;
    let mut engine = Engine::with_options(options)?;
    assert_eq!(engine.options().hash_mib(), 65);
    let limits = SearchLimits { depth: 1, nodes: 10_000 };
    engine.start(limits)?;
    let private = finish(&mut engine)?;
    assert_eq!(private.status, SearchStatus::Finished(Completion::Depth));
    drop(engine);

    let shared = Arc::new(SharedTable::new(65)?);
    options.set_hash_mib(1)?;
    let mut worker = Engine::with_options(options)?;
    worker.set_shared_table(Some(shared));
    worker.start(limits)?;
    assert_eq!(finish(&mut worker)?, private);
    Ok(())
}

#[test]
fn maximum_multipv_covers_every_legal_root_without_duplicates() -> Result<(), Box<dyn Error>> {
    let mut options = Options::default();
    options.set_mode(Mode::Analysis);
    options.set_hash_mib(1)?;
    options.set_multi_pv(MAX_MULTI_PV)?;
    let mut engine = Engine::with_options(options)?;
    for (fen, minimum_roots) in [
        ("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", 32),
        ("7k/5ppp/8/8/Q1Q1Q1Q1/8/2Q1Q1Q1/K7 w - - 0 1", 64),
        (START_FEN, 19),
    ] {
        engine.set_position(fen, &[])?;
        let roots = engine.game().board().legal_moves();
        assert!(roots.len() > minimum_roots);
        engine.start(SearchLimits { depth: 1, nodes: 2_000_000 })?;
        let report = finish(&mut engine)?;
        assert_eq!(report.status, SearchStatus::Finished(Completion::Depth));
        assert_eq!(report.variations.len(), roots.len());
        for root in roots {
            assert_eq!(
                report.variations.iter().filter(|line| line.moves.first() == Some(&root)).count(),
                1
            );
        }
        assert!(report.variations.windows(2).all(|pair| pair[0].score_cp >= pair[1].score_cp));
    }
    Ok(())
}

#[test]
fn native_search_completes_depth_127_and_accepts_full_slice_range() -> Result<(), Box<dyn Error>> {
    let mut options = Options::default();
    options.set_mode(Mode::Analysis);
    options.set_hash_mib(1)?;
    let mut engine = Engine::with_options(options)?;
    engine.set_position("7k/8/6KQ/8/8/8/8/8 w - - 0 1", &[])?;
    engine.start_moves(SearchLimits::full(), &["h6g7"])?;
    let report = finish(&mut engine)?;
    assert_eq!(report.depth, 127);
    assert_eq!(report.status, SearchStatus::Finished(Completion::Depth));
    assert_eq!(report.mate_in(), Some(1));
    assert_eq!(report.best_move.map(|move_| move_.to_uci(false)).as_deref(), Some("h6g7"));
    assert_eq!(engine.step(u32::MAX)?, report);
    Ok(())
}
