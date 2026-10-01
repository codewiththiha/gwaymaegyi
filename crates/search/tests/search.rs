//! Search determinism, limits, legal output, policy controls, and rollback regressions.

use gwaymaegyi_core::{Game, START_FEN};
use gwaymaegyi_search::{
    Completion, Engine, Mode, Options, SearchLimits, SearchReport, SearchStatus,
};
use std::error::Error;

fn finish(engine: &mut Engine, quantum: u32) -> Result<SearchReport, Box<dyn Error>> {
    for _ in 0..200_000 {
        let report = engine.step(quantum)?;
        if !engine.searching() {
            return Ok(report);
        }
    }
    Err("search did not terminate".into())
}

#[test]
fn quantum_size_does_not_change_search_decisions() -> Result<(), Box<dyn Error>> {
    let mut a = Engine::new()?;
    let mut b = Engine::new()?;
    let limits = SearchLimits {
        depth: 3,
        nodes: 20_000,
    };
    a.start(limits)?;
    b.start(limits)?;
    let first = finish(&mut a, 1)?;
    let second = finish(&mut b, 512)?;
    assert_eq!(first, second);
    assert_eq!(first.depth, 3);
    assert_eq!(first.status, SearchStatus::Finished(Completion::Depth));
    let mut position = a.game().board().to_owned();
    for mv in &first.variations[0].moves {
        position = position.play_uci(&mv.to_uci(false), false)?;
    }
    Ok(())
}

#[test]
fn mate_and_stalemate_produce_no_phantom_moves() -> Result<(), Box<dyn Error>> {
    for (fen, expected) in [
        ("7k/6Q1/6K1/8/8/8/8/8 b - - 150 1", -30_000),
        ("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1", 0),
    ] {
        let mut engine = Engine::new()?;
        engine.set_position(fen, &[])?;
        engine.start(SearchLimits::default())?;
        let report = finish(&mut engine, 32)?;
        assert_eq!(report.best_move, None);
        assert_eq!(report.score_cp, Some(expected));
    }
    let mut engine = Engine::new()?;
    engine.set_position("7k/8/6KQ/8/8/8/8/8 w - - 0 1", &[])?;
    engine.start(SearchLimits {
        depth: 2,
        nodes: 10_000,
    })?;
    let report = finish(&mut engine, 64)?;
    let mv = report.best_move.ok_or("missing mating move")?;
    let child = engine.game().board().play_uci(&mv.to_uci(false), false)?;
    assert!(child.legal_moves().is_empty());
    assert!(child.in_check(child.side_to_move()));
    assert_eq!(report.score_cp, Some(29_999));
    Ok(())
}

#[test]
fn node_caps_and_immediate_stops_keep_a_legal_fallback() -> Result<(), Box<dyn Error>> {
    let mut engine = Engine::new()?;
    engine.start(SearchLimits {
        depth: 64,
        nodes: 23,
    })?;
    let report = finish(&mut engine, 512)?;
    assert_eq!(report.nodes, 23);
    assert_eq!(report.status, SearchStatus::Finished(Completion::Nodes));
    let mv = report.best_move.ok_or("missing fallback")?;
    assert!(engine.game().board().legal_moves().contains(&mv));
    engine.start(SearchLimits::default())?;
    engine.stop();
    assert_eq!(
        engine.report().status,
        SearchStatus::Finished(Completion::Stopped)
    );
    assert!(engine.report().best_move.is_some());
    Ok(())
}

#[test]
fn invalid_updates_are_transactional() -> Result<(), Box<dyn Error>> {
    let mut engine = Engine::new()?;
    engine.start(SearchLimits::default())?;
    engine.step(8)?;
    let before = engine.report();
    assert!(
        engine
            .start(SearchLimits {
                depth: 0,
                nodes: 10
            })
            .is_err()
    );
    assert!(engine.set_position(START_FEN, &["e2e5"]).is_err());
    assert!(engine.play_uci("invalid").is_err());
    assert_eq!(engine.report(), before);
    assert_eq!(engine.game().board().to_string(), START_FEN);
    assert!(engine.step(0).is_err());
    assert!(engine.step(65_537).is_err());
    Ok(())
}

#[test]
fn playing_controls_validate_presets_and_distinct_principal_variations()
-> Result<(), Box<dyn Error>> {
    let mut options = Options::default();
    assert!(options.set_elo(499).is_err());
    assert!(options.set_elo(3001).is_err());
    assert!(options.set_hash_mib(0).is_err());
    assert!(options.set_multi_pv(6).is_err());
    options.set_mode(Mode::Analysis);
    options.set_multi_pv(3)?;
    let mut engine = Engine::new()?;
    engine.configure(options)?;
    engine.start(SearchLimits {
        depth: 1,
        nodes: 20_000,
    })?;
    let result = finish(&mut engine, 64)?;
    assert_eq!(result.variations.len(), 3);
    for (index, line) in result.variations.iter().enumerate() {
        assert!(
            !result.variations[..index]
                .iter()
                .any(|other| other.moves[0] == line.moves[0])
        );
    }
    assert!(
        result
            .variations
            .windows(2)
            .all(|pair| pair[0].score_cp >= pair[1].score_cp)
    );
    options.set_mode(Mode::Human);
    options.set_elo(500)?;
    engine.configure(options)?;
    engine.start(SearchLimits {
        depth: 64,
        nodes: u64::MAX,
    })?;
    let weak = finish(&mut engine, 64)?;
    assert!(weak.depth <= 1);
    assert!(weak.nodes <= 256);
    assert!(
        engine
            .game()
            .board()
            .legal_moves()
            .contains(&weak.best_move.ok_or("missing weak move")?)
    );
    Ok(())
}

#[test]
fn repeated_positions_and_capture_resets_do_not_poison_search() -> Result<(), Box<dyn Error>> {
    let mut game = Game::start()?;
    for _ in 0..2 {
        for mv in ["g1f3", "g8f6", "f3g1", "f6g8"] {
            game.play_uci(mv, false)?;
        }
    }
    assert_eq!(game.repetitions(), 3);
    let mut engine = Engine::new()?;
    engine.set_position(
        START_FEN,
        &[
            "g1f3", "g8f6", "f3g1", "f6g8", "g1f3", "g8f6", "f3g1", "f6g8",
        ],
    )?;
    engine.start_moves(SearchLimits::default(), &["g1f3"])?;
    assert_eq!(finish(&mut engine, 32)?.score_cp, Some(0));
    Ok(())
}

#[test]
fn root_filters_validate_atomically_and_keep_the_requested_move() -> Result<(), Box<dyn Error>> {
    let mut engine = Engine::new()?;
    engine.start_moves(
        SearchLimits {
            depth: 2,
            nodes: 20_000,
        },
        &["e2e4"],
    )?;
    engine.step(16)?;
    let before = engine.report();
    assert!(
        engine
            .start_moves(SearchLimits::default(), &["e2e5"])
            .is_err()
    );
    assert_eq!(before, engine.report());
    let report = finish(&mut engine, 128)?;
    assert_eq!(
        report.best_move.map(|mv| mv.to_uci(false)),
        Some("e2e4".into())
    );
    assert_eq!(report.variations.len(), 1);
    Ok(())
}

#[test]
fn optional_clock_claims_do_not_hide_a_mating_root_move() -> Result<(), Box<dyn Error>> {
    let mut engine = Engine::new()?;
    engine.set_position("7k/8/6KQ/8/8/8/8/8 w - - 149 1", &[])?;
    engine.start(SearchLimits {
        depth: 2,
        nodes: 10_000,
    })?;
    assert_eq!(finish(&mut engine, 64)?.score_cp, Some(29_999));
    Ok(())
}
