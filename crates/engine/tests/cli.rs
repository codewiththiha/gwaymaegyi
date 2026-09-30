use std::error::Error;

fn output(args: &[&str]) -> Result<String, Box<dyn Error>> {
    let mut bytes = Vec::new();
    gwaymaegyi::run(args.iter().map(|value| (*value).to_owned()), &mut bytes)?;
    Ok(String::from_utf8(bytes)?)
}

#[test]
fn help_is_honest_about_search_support() -> Result<(), Box<dyn Error>> {
    let text = output(&[])?;
    assert!(text.contains("search/UCI sessions are not implemented yet"));
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
    ] {
        assert!(output(args).is_err());
    }
}

#[test]
fn moves_and_fen_use_the_shared_board() -> Result<(), Box<dyn Error>> {
    assert_eq!(output(&["moves"])?.split_whitespace().count(), 20);
    let fen = "4k3/8/8/8/8/8/8/4K3 w - - 0 1";
    assert_eq!(output(&["fen", fen])?, format!("{fen}\n"));
    Ok(())
}
