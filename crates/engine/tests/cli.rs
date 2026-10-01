//! Utility-command output and invalid-input regression tests.

use std::error::Error;

fn output(args: &[&str]) -> Result<String, Box<dyn Error>> {
    let mut bytes = Vec::new();
    gwaymaegyi::run(args.iter().map(|value| (*value).to_owned()), &mut bytes)?;
    Ok(String::from_utf8(bytes)?)
}

#[test]
fn help_is_honest_about_search_support() -> Result<(), Box<dyn Error>> {
    let text = output(&[])?;
    assert!(text.contains("No arguments starts a UCI session"));
    assert!(text.contains("uncalibrated"));
    Ok(())
}

#[test]
fn version_matches_the_manifest() -> Result<(), Box<dyn Error>> {
    assert_eq!(
        output(&["--version"])?,
        format!("gwaymaegyi {}\n", env!("CARGO_PKG_VERSION"))
    );
    Ok(())
}

#[test]
fn perft_has_the_correct_zero_and_root_counts() -> Result<(), Box<dyn Error>> {
    assert_eq!(output(&["perft", "0"])?, "1 nodes\n");
    assert!(output(&["perft", "2"])?.ends_with("400 nodes\n"));
    Ok(())
}

#[test]
fn invalid_commands_return_errors() {
    for args in [
        &["unknown"][..],
        &["perft"],
        &["perft", "255"],
        &["fen"],
        &["fen", "bad"],
        &["convert"],
        &["convert", "unknown", "in.bin", "out.txt"],
        &["filter"],
        &["filter", "unknown", "in.txt"],
    ] {
        assert!(output(args).is_err());
    }
}

#[test]
fn moves_and_fen_use_the_shared_board() -> Result<(), Box<dyn Error>> {
    assert_eq!(output(&["moves"])?.split_whitespace().count(), 20);
    let fen = "4k3/8/8/8/8/8/8/4K3 w - - 0 1";
    assert_eq!(output(&["fen", fen])?, format!("{fen}\n"));
    let bench = output(&["bench", "2", "2"])?;
    assert!(bench.starts_with("bench: 2 positions, depth 2, "));
    Ok(())
}

#[test]
fn convert_and_filter_subcommands_round_trip_records() -> Result<(), Box<dyn Error>> {
    let dir = std::env::temp_dir();
    let pid = std::process::id();
    let text_in = dir.join(format!("gwaymaegyi-cli-in-{pid}.txt"));
    let bin_path = dir.join(format!("gwaymaegyi-cli-packed-{pid}.bin"));
    let text_out = dir.join(format!("gwaymaegyi-cli-out-{pid}.txt"));
    let sample = concat!(
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 | 15 | 0.5\n",
        "r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQK1NR w KQkq - 0 1 | 40 | 1.0\n",
    );
    std::fs::write(&text_in, sample)?;

    let text_in_str = text_in.to_string_lossy().into_owned();
    let bin_str = bin_path.to_string_lossy().into_owned();
    let text_out_str = text_out.to_string_lossy().into_owned();

    assert_eq!(
        output(&["convert", "encode", &text_in_str, &bin_str])?,
        "encoded 2 records\n"
    );
    assert_eq!(
        output(&["convert", "decode", &bin_str, &text_out_str])?,
        "decoded 2 records\n"
    );
    let filtered = output(&["filter", "material-sacrifice", &text_in_str])?;
    assert_eq!(filtered.lines().count(), 1);
    assert!(filtered.contains("| 40 | 1.0"));

    let _ = std::fs::remove_file(text_in);
    let _ = std::fs::remove_file(bin_path);
    let _ = std::fs::remove_file(text_out);
    Ok(())
}

#[test]
fn datagen_and_parallel_analysis_produce_valid_outputs() -> Result<(), Box<dyn Error>> {
    use gwaymaegyi::{
        AnalysisRequest, DatagenConfig, Options, SearchLimits, analyze_parallel,
        generate_training_data, multipv_threshold, normalize_opening_line,
    };
    use std::sync::atomic::AtomicBool;

    assert_eq!(normalize_opening_line("\u{FEFF}  \r\n")?, None);
    let normalized = normalize_opening_line(
        "\u{FEFF}rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1\r",
    )?;
    assert_eq!(normalized.as_deref(), Some(gwaymaegyi_core::START_FEN));
    assert!(multipv_threshold(5) > multipv_threshold(20));
    assert_eq!(multipv_threshold(30), 0);

    let cancel = AtomicBool::new(false);
    let config = DatagenConfig {
        positions: 2,
        threads: 1,
        seed: 7,
        opt_nodes: 64,
        max_nodes: 256,
        max_depth: 2,
        ..DatagenConfig::default()
    };
    let records = generate_training_data(&config, &cancel)?;
    assert_eq!(records.len(), 2);

    let cli_out = output(&["datagen", "1", "1", "7"])?;
    assert_eq!(cli_out.lines().count(), 1);
    assert!(cli_out.contains(" | "));

    let request = AnalysisRequest {
        fen: gwaymaegyi_core::START_FEN.to_owned(),
        moves: Vec::new(),
        roots: Vec::new(),
        options: Options::default(),
        limits: SearchLimits {
            depth: 2,
            nodes: 4_000,
        },
        tablebase: None,
    };
    let report = analyze_parallel(&request, 2, &cancel)?;
    assert!(report.best_move.is_some());
    assert!(report.nodes > 0);
    Ok(())
}

