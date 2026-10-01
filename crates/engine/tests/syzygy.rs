//! Optional integration check for native Syzygy WDL and root-DTZ files.

use gwaymaegyi::{
    Engine, NativeTablebases, SearchLimits, SearchStatus, TablebaseProbe, TablebaseWdl,
};
use gwaymaegyi_core::Board;
use gwaymaegyi_search::Completion;
use std::{error::Error, sync::Arc};

#[test]
fn supplied_krvk_tables_probe_wdl_root_dtz_and_engine_result() -> Result<(), Box<dyn Error>> {
    let Ok(path) = std::env::var("GWAYMAEGYI_SYZYGY_PATH") else {
        eprintln!("skipping optional Syzygy fixture: GWAYMAEGYI_SYZYGY_PATH is unset");
        return Ok(());
    };
    let tables = Arc::new(NativeTablebases::open(&path).map_err(std::io::Error::other)?);
    let fen = "8/8/8/2R5/1K6/8/5k2/8 w - - 0 1";
    let board: Board = fen.parse()?;
    assert_eq!(
        tables
            .probe_wdl_checked(&board)
            .map_err(std::io::Error::other)?,
        Some(TablebaseWdl::Win)
    );
    let root = tables
        .probe_root(&board, &[])
        .ok_or("KRvK root DTZ probe failed")?;
    assert_eq!(root.best_move.to_uci(false), "c5c4");
    assert_eq!(root.wdl, TablebaseWdl::Win);
    assert_eq!(root.dtz, 21);

    let mut engine = Engine::new()?;
    engine.set_tablebase(Some(tables as Arc<dyn TablebaseProbe>));
    engine.set_position(fen, &[])?;
    engine.start(SearchLimits {
        depth: 4,
        nodes: 100_000,
    })?;
    let report = engine.report();
    assert_eq!(report.status, SearchStatus::Finished(Completion::Tablebase));
    assert_eq!(
        report.best_move.map(|chess_move| chess_move.to_uci(false)),
        Some("c5c4".into())
    );
    assert_eq!(report.score_cp, Some(29_000));
    assert_eq!(report.tablebase_hits, 1);
    Ok(())
}
