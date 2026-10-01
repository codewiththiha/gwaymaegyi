//! Search determinism, limits, legal output, policy controls, and rollback regressions.

use gwaymaegyi_core::{Board, Game, Move, START_FEN};
use gwaymaegyi_search::{
    Completion, Engine, Mode, Options, SearchLimits, SearchReport, SearchStatus, TablebaseProbe,
    TablebaseRoot, TablebaseWdl,
};
use std::{error::Error, sync::Arc};

#[derive(Debug)]
struct FixedRootTablebase(TablebaseRoot);
impl TablebaseProbe for FixedRootTablebase {
    fn max_pieces(&self) -> u32 {
        7
    }
    fn probe_wdl(&self, _board: &Board) -> Option<TablebaseWdl> {
        None
    }
    fn probe_root(&self, _board: &Board, _allowed: &[Move]) -> Option<TablebaseRoot> {
        Some(self.0)
    }
}

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
    engine.set_position("k7/2K5/8/8/8/8/8/1Q6 w - - 0 1", &[])?;
    engine.start(SearchLimits {
        depth: 2,
        nodes: 10_000,
    })?;
    let report = finish(&mut engine, 64)?;
    let mv = report.best_move.ok_or("missing mating move")?;
    let child = engine.game().board().play_uci(&mv.to_uci(false), false)?;
    assert_eq!(child.legal_moves(), Vec::new());
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
    engine.start(SearchLimits {
        depth: 64,
        nodes: u64::MAX,
    })?;
    engine.step(64)?;
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

#[test]
fn skill_presets_cover_the_documented_levels() -> Result<(), Box<dyn Error>> {
    use gwaymaegyi_search::{SkillLevel, Strength};
    let mut options = Options::default();
    assert_eq!(options.skill_level(), Some(SkillLevel::FULL));
    for (index, &elo) in SkillLevel::NOMINAL_ELO.iter().enumerate() {
        let level = SkillLevel::new(u8::try_from(index + 1)?)?;
        options.set_skill_level(level);
        assert_eq!(options.strength(), Strength::Approximate(elo));
        assert_eq!(options.skill_level(), Some(level));
    }
    assert!(SkillLevel::new(0).is_err());
    assert!(SkillLevel::new(22).is_err());
    options.set_elo(1350)?;
    assert_eq!(options.skill_level(), None);
    options.set_skill_level(SkillLevel::FULL);
    assert_eq!(options.strength(), Strength::Full);
    Ok(())
}

#[test]
fn live_budget_updates_preserve_search_progress() -> Result<(), Box<dyn Error>> {
    let mut a = Engine::new()?;
    let mut b = Engine::new()?;
    let short = SearchLimits {
        depth: 2,
        nodes: 20_000,
    };
    let long = SearchLimits {
        depth: 3,
        nodes: 20_000,
    };
    a.start(short)?;
    b.start(long)?;
    a.step(25)?;
    b.step(25)?;
    let before = a.report();
    assert!(a.set_limits(SearchLimits { depth: 0, nodes: 0 }).is_err());
    assert_eq!(a.report(), before);
    a.set_limits(long)?;
    assert_eq!(a.report(), before);
    assert_eq!(a.requested_limits(), Some(long));
    assert_eq!(finish(&mut a, 128)?, finish(&mut b, 128)?);
    a.start(SearchLimits::full())?;
    a.step(16)?;
    let used = a.report().nodes;
    a.set_limits(SearchLimits {
        depth: 64,
        nodes: used,
    })?;
    assert_eq!(a.report().nodes, used);
    assert_eq!(a.report().status, SearchStatus::Finished(Completion::Nodes));
    assert!(a.set_limits(long).is_err());
    Ok(())
}

#[test]
fn limited_strength_remains_in_force_when_compute_caps_increase() -> Result<(), Box<dyn Error>> {
    let mut engine = Engine::new()?;
    let mut options = engine.options();
    options.set_elo(500)?;
    engine.configure(options)?;
    engine.start(SearchLimits {
        depth: 3,
        nodes: 1000,
    })?;
    engine.set_limits(SearchLimits::full())?;
    assert_eq!(engine.requested_limits(), Some(SearchLimits::full()));
    assert_eq!(
        engine.effective_limits(),
        Some(SearchLimits {
            depth: 1,
            nodes: 256
        })
    );
    Ok(())
}

#[test]
fn behavior_and_parameters_validate_before_committing() -> Result<(), Box<dyn Error>> {
    use gwaymaegyi_search::{Behavior, Parameter, SearchTuning};
    let mut tuning = SearchTuning::default();
    let before = tuning;
    assert!(tuning.set(Parameter::NullDepthDiv, 0).is_err());
    assert_eq!(tuning, before);
    for spec in Parameter::SPECS {
        tuning.set(spec.parameter, spec.default)?;
    }
    assert!(Parameter::parse("unknown").is_err());
    assert!(Behavior::parse("unknown").is_err());
    tuning.set_behavior(Behavior::NullMove, false);
    assert!(!tuning.enabled(Behavior::NullMove));
    let mut engine = Engine::new()?;
    let mut options = engine.options();
    options.set_tuning(tuning);
    engine.configure(options)?;
    assert_eq!(engine.options().tuning(), tuning);
    Ok(())
}

#[test]
fn aspiration_retries_remain_quantum_invariant() -> Result<(), Box<dyn Error>> {
    let mut a = Engine::new()?;
    let mut b = Engine::new()?;
    a.start(SearchLimits {
        depth: 5,
        nodes: 100_000,
    })?;
    b.start(SearchLimits {
        depth: 5,
        nodes: 100_000,
    })?;
    let first = finish(&mut a, 1)?;
    let second = finish(&mut b, 512)?;
    assert_eq!(first, second);
    assert_eq!(first.depth, 5);
    Ok(())
}

#[test]
fn human_play_policy_is_deterministic_and_weakens_play() -> Result<(), Box<dyn Error>> {
    let mut engine = Engine::new()?;
    let mut options = Options::default();
    options.set_mode(Mode::Balanced);
    options.set_elo(500)?;
    options.set_multi_pv(5)?;
    engine.configure(options)?;
    let mut moves = Vec::new();
    for _ in 0..4 {
        engine.start(SearchLimits {
            depth: 6,
            nodes: 200_000,
        })?;
        let report = finish(&mut engine, 128)?;
        let mv = report.best_move.ok_or("missing move")?;
        engine.play_uci(&mv.to_uci(false))?;
        moves.push(mv.to_uci(false));
    }
    // Same seed and budget rules reproduce the identical game.
    let mut clone = Engine::new()?;
    clone.configure(options)?;
    for expected in &moves {
        clone.start(SearchLimits {
            depth: 6,
            nodes: 200_000,
        })?;
        let report = finish(&mut clone, 128)?;
        let mv = report.best_move.ok_or("missing move")?;
        assert_eq!(mv.to_uci(false), *expected);
        clone.play_uci(expected)?;
    }
    // Full strength stays at the best line.
    let mut full = Engine::new()?;
    let mut full_options = Options::default();
    full_options.set_multi_pv(5)?;
    full.configure(full_options)?;
    full.start(SearchLimits {
        depth: 3,
        nodes: 20_000,
    })?;
    let report = finish(&mut full, 128)?;
    let Some(best) = report.variations.first() else {
        return Err("missing variation".into());
    };
    let chosen = report.best_move.ok_or("missing move")?;
    assert_eq!(chosen, best.moves[0]);
    Ok(())
}

#[test]
fn all_algorithm_behaviors_are_discoverable_and_instance_owned() -> Result<(), Box<dyn Error>> {
    use gwaymaegyi_search::{Behavior, SearchTuning};
    assert_eq!(Behavior::ALL.len(), 11);
    let original = SearchTuning::default();
    let mut changed = original;
    for behavior in Behavior::ALL {
        assert_eq!(Behavior::parse(behavior.name())?, behavior);
        changed.set_behavior(behavior, false);
        assert!(!changed.enabled(behavior));
        assert!(original.enabled(behavior));
    }
    let mut options = Options::default();
    options.set_tuning(changed);
    let mut engine = Engine::new()?;
    engine.configure(options)?;
    engine.start(SearchLimits {
        depth: 2,
        nodes: 20_000,
    })?;
    let report = finish(&mut engine, 128)?;
    assert_eq!(report.depth, 2);
    assert!(report.best_move.is_some());
    Ok(())
}

#[test]
fn reference_tuning_inventory_is_discoverable_and_in_range() -> Result<(), Box<dyn Error>> {
    use gwaymaegyi_search::Parameter;
    assert_eq!(Parameter::SPECS.len(), 38);
    for spec in Parameter::SPECS {
        assert!(spec.min <= spec.default && spec.default <= spec.max);
        assert_eq!(Parameter::parse(spec.name)?, spec.parameter);
    }
    Ok(())
}

#[derive(Debug)]
struct DrawTablebase;
impl TablebaseProbe for DrawTablebase {
    fn max_pieces(&self) -> u32 {
        32
    }
    fn probe_wdl(&self, _: &Board) -> Option<TablebaseWdl> {
        Some(TablebaseWdl::Draw)
    }
}

#[test]
fn tablebase_wdl_is_used_only_below_the_root_and_reported() -> Result<(), Box<dyn Error>> {
    use std::sync::Arc;
    let mut engine = Engine::new()?;
    engine.set_tablebase(Some(Arc::new(DrawTablebase)));
    engine.start(SearchLimits {
        depth: 2,
        nodes: 20_000,
    })?;
    let report = finish(&mut engine, 128)?;
    assert!(report.tablebase_hits > 0);
    assert!(report.score_cp.is_some());
    engine.set_tablebase(None);
    assert!(!engine.searching());
    Ok(())
}

#[test]
fn root_tablebase_is_exact_but_respects_search_controls() -> Result<(), Box<dyn Error>> {
    let fen = "8/8/8/2R5/1K6/8/5k2/8 w - - 0 1";
    let board: Board = fen.parse()?;
    let tablebase_move = board.resolve_uci("c5c4", false)?;
    let probe = Arc::new(FixedRootTablebase(TablebaseRoot {
        best_move: tablebase_move,
        wdl: TablebaseWdl::Win,
        dtz: 21,
    }));
    let limits = SearchLimits {
        depth: 3,
        nodes: 20_000,
    };

    let mut exact = Engine::new()?;
    exact.set_tablebase(Some(probe.clone()));
    exact.set_position(fen, &[])?;
    exact.start(limits)?;
    let report = exact.report();
    assert_eq!(report.status, SearchStatus::Finished(Completion::Tablebase));
    assert_eq!(report.best_move, Some(tablebase_move));
    assert_eq!(report.score_cp, Some(29_000));
    assert_eq!(report.mate_in(), None);
    assert_eq!(report.tablebase_hits, 1);
    assert_eq!(report.nodes, 0);

    let allowed_move = board.resolve_uci("c5b5", false)?;
    let mut filtered = Engine::new()?;
    filtered.set_tablebase(Some(probe.clone()));
    filtered.set_position(fen, &[])?;
    filtered.start_moves(limits, &["c5b5"])?;
    assert_eq!(filtered.report().status, SearchStatus::Running);
    let filtered_report = finish(&mut filtered, 128)?;
    assert_eq!(filtered_report.best_move, Some(allowed_move));
    assert_ne!(
        filtered_report.status,
        SearchStatus::Finished(Completion::Tablebase)
    );

    let mut options = Options::default();
    options.set_multi_pv(2)?;
    let mut multipv = Engine::new()?;
    multipv.set_tablebase(Some(probe));
    multipv.configure(options)?;
    multipv.set_position(fen, &[])?;
    multipv.start(limits)?;
    assert_eq!(multipv.report().status, SearchStatus::Running);
    let multipv_report = finish(&mut multipv, 128)?;
    assert_eq!(multipv_report.variations.len(), 2);
    Ok(())
}

#[test]
fn shared_table_coordinates_multi_worker_search_entries() -> Result<(), Box<dyn Error>> {
    use gwaymaegyi_search::SharedTable;
    let shared = Arc::new(SharedTable::new(1)?);
    let mut primary = Engine::new()?;
    let mut helper = Engine::new()?;
    primary.set_worker_id(0);
    helper.set_worker_id(1);
    primary.set_shared_table(Some(Arc::clone(&shared)));
    helper.set_shared_table(Some(Arc::clone(&shared)));
    let limits = SearchLimits {
        depth: 2,
        nodes: 4_000,
    };
    helper.start(limits)?;
    let helper_report = finish(&mut helper, 128)?;
    assert!(helper_report.best_move.is_some());
    shared.next_search();
    primary.start(limits)?;
    let primary_report = finish(&mut primary, 128)?;
    assert!(primary_report.best_move.is_some());
    shared.clear();
    Ok(())
}
