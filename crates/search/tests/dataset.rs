//! Integration tests for binary bulletformat conversion and all 11 position filters.

#![expect(
    clippy::unwrap_used,
    reason = "Integration tests assert deterministic dataset conversion and filtering."
)]

use gwaymaegyi_search::{
    BULLET_RECORD_BYTES, BulletRecord, DatasetError, FilterKind, GameResult, TrainingRecord,
    decode_bullet_records, encode_bullet_records, filter_lines,
};

fn rec(line: &str) -> TrainingRecord {
    line.parse().unwrap()
}

#[test]
fn bullet_record_round_trips_text_and_binary_streams() {
    let input = concat!(
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 | 24 | 1.0\n",
        "4k3/8/8/8/8/8/4P3/4K3 b - - 0 0 | -115 | 0.5\n",
    );
    let encoded = encode_bullet_records(input).unwrap();
    assert_eq!(encoded.len(), BULLET_RECORD_BYTES * 2);

    let decoded = decode_bullet_records(&encoded).unwrap();
    assert_eq!(decoded.len(), 2);
    assert_eq!(
        decoded[0].to_string(),
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - - 0 0 | 24 | 1.0"
    );
    assert_eq!(decoded[0].score(), 24);
    assert_eq!(decoded[0].result(), GameResult::WhiteWin);
    assert_eq!(
        decoded[1].to_string(),
        "4k3/8/8/8/8/8/4P3/4K3 w - - 0 0 | -115 | 0.5"
    );
    assert_eq!(decoded[1].score(), -115);
    assert_eq!(decoded[1].result(), GameResult::Draw);
}

#[test]
fn dataset_rejects_malformed_text_and_binary_inputs() {
    assert_eq!(
        "missing delimiters".parse::<TrainingRecord>(),
        Err(DatasetError::InvalidFormat)
    );
    assert_eq!(
        "4k3/8/8/8/8/8/8/4K3 w - - 0 1 | not_a_number | 1.0".parse::<TrainingRecord>(),
        Err(DatasetError::InvalidScore)
    );
    assert_eq!(
        "4k3/8/8/8/8/8/8/4K3 w - - 0 1 | 0 | 0.25".parse::<TrainingRecord>(),
        Err(DatasetError::InvalidResult)
    );
    assert_eq!(
        BulletRecord::from_bytes(&[0_u8; 31]),
        Err(DatasetError::InvalidBulletLength)
    );
    let mut invalid = [0_u8; BULLET_RECORD_BYTES];
    invalid[26] = 9;
    assert_eq!(
        BulletRecord::from_bytes(&invalid),
        Err(DatasetError::InvalidBulletRecord)
    );
}

#[test]
fn filter_catalog_parses_all_eleven_identifiers() {
    for (index, kind) in FilterKind::ALL.into_iter().enumerate() {
        let number = u8::try_from(index + 1).unwrap();
        assert_eq!(kind.number(), number);
        assert_eq!(FilterKind::parse(&number.to_string()), Some(kind));
        assert_eq!(FilterKind::parse(kind.name()), Some(kind));
    }
    assert_eq!(FilterKind::parse("unknown-filter"), None);
}

#[test]
fn all_eleven_position_filters_distinguish_matching_and_quiet_records() {
    let quiet = rec("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 | 15 | 0.5");
    for kind in FilterKind::ALL {
        assert!(
            !kind.matches(&quiet),
            "start position should not match {}",
            kind.name()
        );
    }

    let f1 = rec("r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQK1NR w KQkq - 0 1 | 40 | 1.0");
    assert!(FilterKind::MaterialSacrifice.matches(&f1));

    let f2 = rec("4k3/8/8/8/5q2/4rb2/8/4K3 w - - 0 1 | -300 | 0.0");
    assert!(FilterKind::KingDanger.matches(&f2));

    let f3 = rec("4k2r/pppppppp/2PPPP2/1PPPPPP1/1PPPPPP1/3PP3/8/R1BQKBNR w KQk - 0 1 | 120 | 1.0");
    assert!(FilterKind::SpaceAdvantage.matches(&f3));

    let f4 = rec("2kr3r/ppp2ppp/2n5/6P1/8/2N5/PPP2P1P/R4RK1 w - - 0 1 | 85 | 1.0");
    assert!(FilterKind::OppositeCastlingStorm.matches(&f4));

    let f5 = rec("rnbqkbnr/pppppppp/8/8/2B1P3/2N2N2/PPPPQPPP/R1B2RK1 w kq - 0 1 | 180 | 1.0");
    assert!(FilterKind::DevelopmentImbalance.matches(&f5));

    let f6 = rec("rnbq1rk1/pppppppp/8/8/4K3/8/PPPPPPPP/RNBQ1BNR w - - 0 1 | -200 | 0.0");
    assert!(FilterKind::ExposedKingMobility.matches(&f6));

    let f7 = rec("4k3/8/8/8/3qrb2/3nn3/8/4K2Q w - - 0 1 | -250 | 0.0");
    assert!(FilterKind::AttackVector.matches(&f7));

    let f8 = rec("3qk2r/8/3N4/8/8/8/PPP2PPP/R2QK2R w KQk - 0 1 | 150 | 1.0");
    assert!(FilterKind::MonsterOutpost.matches(&f8));

    let f9 = rec("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 | 350 | 1.0");
    assert!(FilterKind::LargeCompensation.matches(&f9));

    let f10 = rec("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 | 450 | 1.0");
    assert!(FilterKind::VerifiedCompensationWin.matches(&f10));

    let f11 = rec("r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQK1NR w KQkq - 0 1 | 25 | 1.0");
    assert!(FilterKind::TieredSacrificeCompensation.matches(&f11));

    let lines = format!("{quiet}\n{f11}\n");
    let matched = filter_lines(&lines, FilterKind::TieredSacrificeCompensation).unwrap();
    assert_eq!(matched, vec![f11]);
}
