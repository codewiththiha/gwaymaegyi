use gwaymaegyi_core::{Board, ClaimableDraw, Color, DrawReason, Game, Outcome, START_FEN};
use std::error::Error;

#[test]
fn incremental_keys_match_full_recomputation() -> Result<(), Box<dyn Error>> {
    for fen in [
        START_FEN,
        "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
        "4k3/P7/8/3pP3/8/8/8/4K3 w - d6 0 1",
        "4k3/8/8/8/8/8/8/R4KR1 w GA - 0 1",
    ] {
        let root: Board = fen.parse()?;
        assert_eq!(root.key(), root.recomputed_key());
        for child in root.legal_successors() {
            assert_eq!(child.board().key(), child.board().recomputed_key());
            for grandchild in child.board().legal_successors() {
                assert_eq!(
                    grandchild.board().key(),
                    grandchild.board().recomputed_key()
                );
            }
        }
    }
    Ok(())
}

#[test]
fn identity_excludes_counters_but_includes_turn_and_rook_origins() -> Result<(), Box<dyn Error>> {
    let base: Board = "4k3/8/8/8/8/8/8/RR2K2R w A - 0 1".parse()?;
    let clock: Board = "4k3/8/8/8/8/8/8/RR2K2R w A - 99 32".parse()?;
    let turn: Board = "4k3/8/8/8/8/8/8/RR2K2R b A - 0 1".parse()?;
    let rook: Board = "4k3/8/8/8/8/8/8/RR2K2R w B - 0 1".parse()?;
    assert_eq!(base.key(), clock.key());
    assert_ne!(base.key().full(), turn.key().full());
    assert_ne!(base.key().full(), rook.key().full());
    Ok(())
}

#[test]
fn en_passant_identity_requires_a_legal_capture() -> Result<(), Box<dyn Error>> {
    for (ep, none, different) in [
        (
            "4k3/8/8/3p4/8/8/8/4K3 w - d6 0 1",
            "4k3/8/8/3p4/8/8/8/4K3 w - - 0 1",
            false,
        ),
        (
            "4k3/8/8/r4pPK/8/8/8/8 w - f6 0 1",
            "4k3/8/8/r4pPK/8/8/8/8 w - - 0 1",
            false,
        ),
        (
            "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1",
            "4k3/8/8/3pP3/8/8/8/4K3 w - - 0 1",
            true,
        ),
    ] {
        let a: Board = ep.parse()?;
        let b: Board = none.parse()?;
        assert_eq!(a.key().full() != b.key().full(), different);
    }
    Ok(())
}

#[test]
fn repetition_distinguishes_claims_and_automatic_draws() -> Result<(), Box<dyn Error>> {
    let mut game = Game::start()?;
    for lap in 1..=4 {
        for mv in ["g1f3", "g8f6", "f3g1", "f6g8"] {
            game.play_uci(mv, false)?;
        }
        assert_eq!(game.repetitions(), lap + 1);
        if lap == 2 {
            assert_eq!(
                game.claimable_draws(),
                vec![ClaimableDraw::ThreefoldRepetition]
            );
        }
    }
    assert_eq!(
        game.outcome(),
        Outcome::Draw(DrawReason::FivefoldRepetition)
    );
    let before = game.board().key();
    assert!(game.play_uci("not a move", false).is_err());
    assert_eq!(game.board().key(), before);
    Ok(())
}

#[test]
fn mate_precedes_clock_draws_and_two_knights_are_not_dead() -> Result<(), Box<dyn Error>> {
    let mate = Game::new("7k/6Q1/6K1/8/8/8/8/8 b - - 150 1".parse()?);
    assert_eq!(
        mate.outcome(),
        Outcome::Checkmate {
            winner: Color::White
        }
    );
    let knights: Board = "4k3/8/8/8/8/8/NN6/4K3 w - - 0 1".parse()?;
    assert!(!knights.insufficient_material());
    let fifty = Game::new("4k3/8/8/8/8/8/8/R3K3 w - - 100 1".parse()?);
    assert_eq!(fifty.claimable_draws(), vec![ClaimableDraw::FiftyMoves]);
    let automatic = Game::new("4k3/8/8/8/8/8/8/R3K3 w - - 150 1".parse()?);
    assert_eq!(
        automatic.outcome(),
        Outcome::Draw(DrawReason::SeventyFiveMoves)
    );
    Ok(())
}
