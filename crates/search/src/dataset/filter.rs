//! Position filter catalog and material, space, development, and compensation predicates.

#![expect(
    clippy::missing_errors_doc,
    reason = "Filter parsing and line processing errors are described in plain prose."
)]

use gwaymaegyi_core::{Board, Color, File, PieceKind, Rank, Square};

use super::{
    compensation::matches_tiered_sacrifice_compensation,
    record::{DatasetError, GameResult, TrainingRecord},
    tactical::{
        both_sides_have_queens, matches_attack_vector, matches_exposed_king_mobility,
        matches_king_danger, matches_monster_outpost, matches_opposite_castling_storm,
    },
};

/// All 11 training dataset position filters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FilterKind {
    MaterialSacrifice,
    KingDanger,
    SpaceAdvantage,
    OppositeCastlingStorm,
    DevelopmentImbalance,
    ExposedKingMobility,
    AttackVector,
    MonsterOutpost,
    LargeCompensation,
    VerifiedCompensationWin,
    TieredSacrificeCompensation,
}

impl FilterKind {
    pub const ALL: [Self; 11] = [
        Self::MaterialSacrifice,
        Self::KingDanger,
        Self::SpaceAdvantage,
        Self::OppositeCastlingStorm,
        Self::DevelopmentImbalance,
        Self::ExposedKingMobility,
        Self::AttackVector,
        Self::MonsterOutpost,
        Self::LargeCompensation,
        Self::VerifiedCompensationWin,
        Self::TieredSacrificeCompensation,
    ];

    #[must_use]
    pub const fn number(self) -> u8 {
        match self {
            Self::MaterialSacrifice => 1,
            Self::KingDanger => 2,
            Self::SpaceAdvantage => 3,
            Self::OppositeCastlingStorm => 4,
            Self::DevelopmentImbalance => 5,
            Self::ExposedKingMobility => 6,
            Self::AttackVector => 7,
            Self::MonsterOutpost => 8,
            Self::LargeCompensation => 9,
            Self::VerifiedCompensationWin => 10,
            Self::TieredSacrificeCompensation => 11,
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::MaterialSacrifice => "material-sacrifice",
            Self::KingDanger => "king-danger",
            Self::SpaceAdvantage => "space-advantage",
            Self::OppositeCastlingStorm => "opposite-castling-storm",
            Self::DevelopmentImbalance => "development-imbalance",
            Self::ExposedKingMobility => "exposed-king-mobility",
            Self::AttackVector => "attack-vector",
            Self::MonsterOutpost => "monster-outpost",
            Self::LargeCompensation => "large-compensation",
            Self::VerifiedCompensationWin => "verified-compensation-win",
            Self::TieredSacrificeCompensation => "tiered-sacrifice-compensation",
        }
    }

    #[must_use]
    pub fn parse(input: &str) -> Option<Self> {
        let normalized = input.trim().to_ascii_lowercase().replace('_', "-");
        match normalized.as_str() {
            "1" | "filter-1" | "material-sacrifice" => Some(Self::MaterialSacrifice),
            "2" | "filter-2" | "king-danger" => Some(Self::KingDanger),
            "3" | "filter-3" | "space-advantage" => Some(Self::SpaceAdvantage),
            "4" | "filter-4" | "opposite-castling-storm" => Some(Self::OppositeCastlingStorm),
            "5" | "filter-5" | "development-imbalance" => Some(Self::DevelopmentImbalance),
            "6" | "filter-6" | "exposed-king-mobility" => Some(Self::ExposedKingMobility),
            "7" | "filter-7" | "attack-vector" => Some(Self::AttackVector),
            "8" | "filter-8" | "monster-outpost" => Some(Self::MonsterOutpost),
            "9" | "filter-9" | "large-compensation" => Some(Self::LargeCompensation),
            "10" | "filter-10" | "verified-compensation-win" => Some(Self::VerifiedCompensationWin),
            "11" | "filter-11" | "tiered-sacrifice-compensation" => {
                Some(Self::TieredSacrificeCompensation)
            }
            _ => None,
        }
    }

    /// Returns whether a validated training record satisfies this filter.
    #[must_use]
    pub fn matches(self, record: &TrainingRecord) -> bool {
        let board = record.board();
        match self {
            Self::MaterialSacrifice => matches_material_sacrifice(record),
            Self::KingDanger => matches_king_danger(board),
            Self::SpaceAdvantage => matches_space_advantage(board),
            Self::OppositeCastlingStorm => matches_opposite_castling_storm(board),
            Self::DevelopmentImbalance => matches_development_imbalance(board),
            Self::ExposedKingMobility => matches_exposed_king_mobility(board),
            Self::AttackVector => matches_attack_vector(board),
            Self::MonsterOutpost => matches_monster_outpost(board),
            Self::LargeCompensation => matches_large_compensation(record),
            Self::VerifiedCompensationWin => matches_verified_compensation_win(record),
            Self::TieredSacrificeCompensation => matches_tiered_sacrifice_compensation(record),
        }
    }
}

/// Filters newline-delimited training records, returning matching records in input order.
pub fn filter_lines(lines: &str, filter: FilterKind) -> Result<Vec<TrainingRecord>, DatasetError> {
    let mut matched = Vec::new();
    for line in lines.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let record: TrainingRecord = line.parse()?;
        if filter.matches(&record) {
            matched.push(record);
        }
    }
    Ok(matched)
}

pub(super) fn standard_material(board: &Board) -> (i32, i32) {
    const VALUES: [(PieceKind, i32); 5] = [
        (PieceKind::Pawn, 100),
        (PieceKind::Knight, 300),
        (PieceKind::Bishop, 300),
        (PieceKind::Rook, 500),
        (PieceKind::Queen, 900),
    ];
    let mut total = 0;
    let mut white_minus_black = 0;
    for (kind, value) in VALUES {
        let white = i32::try_from(board.pieces(Color::White, kind).len()).unwrap_or(0);
        let black = i32::try_from(board.pieces(Color::Black, kind).len()).unwrap_or(0);
        total += (white + black) * value;
        white_minus_black += (white - black) * value;
    }
    let side_balance = match board.side_to_move() {
        Color::White => white_minus_black,
        Color::Black => -white_minus_black,
    };
    (total, side_balance)
}

fn side_to_move_eval(record: &TrainingRecord) -> i32 {
    let raw = i32::from(record.score());
    match record.board().side_to_move() {
        Color::White => raw,
        Color::Black => -raw,
    }
}

fn matches_material_sacrifice(record: &TrainingRecord) -> bool {
    let (total_material, side_material) = standard_material(record.board());
    let eval = side_to_move_eval(record);
    total_material >= 2_500 && eval > -50 && side_material < -100
}

fn matches_large_compensation(record: &TrainingRecord) -> bool {
    let (total_material, side_material) = standard_material(record.board());
    if total_material < 3_000 {
        return false;
    }
    let eval = side_to_move_eval(record);
    let margin = 300.max(eval.abs() * 3 / 4);
    eval > side_material + margin || eval < side_material - margin
}

fn matches_verified_compensation_win(record: &TrainingRecord) -> bool {
    let (total_material, side_material) = standard_material(record.board());
    if total_material < 3_000 {
        return false;
    }
    let eval = side_to_move_eval(record);
    (eval > side_material + 400 && side_material < 100 && record.result() == GameResult::WhiteWin)
        || (eval < side_material - 400
            && side_material > -100
            && record.result() == GameResult::BlackWin)
}

fn matches_space_advantage(board: &Board) -> bool {
    let (total_material, _) = standard_material(board);
    if total_material <= 3_000 {
        return false;
    }
    (space_score(board, Color::White) - space_score(board, Color::Black)).abs() > 9
}

fn space_score(board: &Board, color: Color) -> i32 {
    const WHITE_CENTER: [Square; 18] = [
        Square::D3,
        Square::E3,
        Square::B4,
        Square::C4,
        Square::D4,
        Square::E4,
        Square::F4,
        Square::G4,
        Square::B5,
        Square::C5,
        Square::D5,
        Square::E5,
        Square::F5,
        Square::G5,
        Square::C6,
        Square::D6,
        Square::E6,
        Square::F6,
    ];
    let mut score = 0;
    for base in WHITE_CENTER {
        let square = match color {
            Color::White => base,
            Color::Black => flip_vertical(base),
        };
        if board.colors(color).contains(square) {
            let rank = i32::from(square.rank().index());
            let bonus = match color {
                Color::White => rank - 2,
                Color::Black => 5 - rank,
            };
            score += bonus;
        }
    }
    let occupied = board.occupied();
    for file_index in 0..7_u8 {
        let Some(file) = File::from_index(file_index) else {
            continue;
        };
        let (first_rank, second_rank) = match color {
            Color::White => (Rank::Second, Rank::Third),
            Color::Black => (Rank::Seventh, Rank::Sixth),
        };
        let first = Square::from_coords(file, first_rank);
        let second = Square::from_coords(file, second_rank);
        if !occupied.contains(first) && !occupied.contains(second) {
            score += 1;
        }
    }
    score
}

fn matches_development_imbalance(board: &Board) -> bool {
    let (total_material, _) = standard_material(board);
    if total_material <= 4_000 || !both_sides_have_queens(board) {
        return false;
    }
    (development_penalty(board, Color::White) - development_penalty(board, Color::Black)).abs() > 3
}

fn development_penalty(board: &Board, color: Color) -> i32 {
    const WHITE_UNDEVELOPED_MINORS: [Square; 18] = [
        Square::A1,
        Square::B1,
        Square::C1,
        Square::D1,
        Square::E1,
        Square::F1,
        Square::G1,
        Square::H1,
        Square::A2,
        Square::B2,
        Square::C2,
        Square::D2,
        Square::E2,
        Square::F2,
        Square::G2,
        Square::H2,
        Square::A3,
        Square::H3,
    ];
    const KING_PENALTY_WHITE_PERSPECTIVE: [i32; 64] = [
        0, 0, 0, 2, 2, 2, 0, 0, 1, 1, 2, 4, 4, 3, 1, 1, 3, 4, 5, 6, 6, 5, 4, 3, 6, 6, 6, 6, 6, 6,
        6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
        6, 6, 6, 6,
    ];
    let minors = board.pieces(color, PieceKind::Knight) | board.pieces(color, PieceKind::Bishop);
    let mut penalty = 0;
    for base in WHITE_UNDEVELOPED_MINORS {
        let square = match color {
            Color::White => base,
            Color::Black => flip_vertical(base),
        };
        if minors.contains(square) {
            penalty += 1;
        }
    }
    if let Some(king) = board.pieces(color, PieceKind::King).into_iter().next() {
        let lookup = match color {
            Color::White => king.index(),
            Color::Black => flip_vertical(king).index(),
        };
        penalty += KING_PENALTY_WHITE_PERSPECTIVE[lookup];
    }
    let friendly = board.colors(color);
    let rooks = board.pieces(color, PieceKind::Rook);
    let corners = match color {
        Color::White => [
            (Square::A1, Square::B1, Square::A2),
            (Square::H1, Square::G1, Square::H2),
        ],
        Color::Black => [
            (Square::A8, Square::B8, Square::A7),
            (Square::H8, Square::G8, Square::H7),
        ],
    };
    for (corner, side_neighbor, front_neighbor) in corners {
        if rooks.contains(corner)
            && friendly.contains(side_neighbor)
            && friendly.contains(front_neighbor)
        {
            penalty += 1;
        }
    }
    penalty
}

pub(super) const fn flip_vertical(square: Square) -> Square {
    let index = square.index() ^ 56;
    match Square::from_index(index) {
        Some(flipped) => flipped,
        None => Square::A1,
    }
}
