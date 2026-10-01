//! Filter 11 tiered sacrifice, initiative, and durable compensation predicate.

use gwaymaegyi_core::{Board, Color, PieceKind};

use super::record::{GameResult, TrainingRecord};

const MIN_TOTAL_MATERIAL: i32 = 3_000;
const OPENING_TOTAL_MATERIAL: i32 = 6_200;
const MIDDLEGAME_TOTAL_MATERIAL: i32 = 4_400;

pub(super) fn matches_tiered_sacrifice_compensation(record: &TrainingRecord) -> bool {
    let (total_material, white_material) = weighted_material(record.board());
    if total_material < MIN_TOTAL_MATERIAL {
        return false;
    }
    let white_eval = i32::from(record.score());
    let (side_eval, side_material) = match record.board().side_to_move() {
        Color::White => (white_eval, white_material),
        Color::Black => (-white_eval, -white_material),
    };
    match record.result() {
        GameResult::WhiteWin => {
            is_interesting_for_winner(total_material, white_eval, white_material)
        }
        GameResult::BlackWin => {
            is_interesting_for_winner(total_material, -white_eval, -white_material)
        }
        GameResult::Draw => {
            (is_tiered_sacrifice(total_material, side_eval, side_material) && side_eval >= 40)
                || (is_durable_compensation(total_material, side_eval, side_material)
                    && side_eval >= 70)
        }
    }
}

fn is_interesting_for_winner(total_material: i32, winner_eval: i32, winner_material: i32) -> bool {
    if is_tiered_sacrifice(total_material, winner_eval, winner_material)
        || is_initiative_position(total_material, winner_eval, winner_material)
        || is_durable_compensation(total_material, winner_eval, winner_material)
    {
        return true;
    }
    let loser_eval = -winner_eval;
    let loser_material = -winner_material;
    (is_tiered_sacrifice(total_material, loser_eval, loser_material) && loser_eval >= 60)
        || (is_durable_compensation(total_material, loser_eval, loser_material) && loser_eval >= 90)
}

const fn scale_by_phase(
    total_material: i32,
    opening_value: i32,
    middlegame_value: i32,
    late_value: i32,
) -> i32 {
    if total_material >= OPENING_TOTAL_MATERIAL {
        opening_value
    } else if total_material >= MIDDLEGAME_TOTAL_MATERIAL {
        middlegame_value
    } else {
        late_value
    }
}

fn is_tiered_sacrifice(total_material: i32, side_eval: i32, side_material: i32) -> bool {
    if side_material > -80 {
        return false;
    }
    let deficit = -side_material;
    let min_eval = if deficit >= 500 {
        scale_by_phase(total_material, -35, -10, 20)
    } else if deficit >= 300 {
        scale_by_phase(total_material, -65, -40, -10)
    } else if deficit >= 170 {
        scale_by_phase(total_material, -95, -70, -35)
    } else {
        scale_by_phase(total_material, -60, -35, -10)
    };
    let min_comp = if deficit >= 500 {
        scale_by_phase(total_material, 340, 390, 440)
    } else if deficit >= 300 {
        scale_by_phase(total_material, 220, 255, 300)
    } else if deficit >= 170 {
        scale_by_phase(total_material, 145, 175, 210)
    } else {
        scale_by_phase(total_material, 105, 125, 150)
    };
    let compensation = side_eval - side_material;
    let max_eval = 650.max(deficit + 250);
    side_eval >= min_eval && side_eval <= max_eval && compensation >= min_comp
}

const fn is_initiative_position(total_material: i32, side_eval: i32, side_material: i32) -> bool {
    if total_material < MIDDLEGAME_TOTAL_MATERIAL || side_material < -80 || side_material > 120 {
        return false;
    }
    let min_eval = scale_by_phase(total_material, 110, 135, 165);
    let min_comp = scale_by_phase(total_material, 150, 175, 210);
    let compensation = side_eval - side_material;
    side_eval >= min_eval && side_eval <= 450 && compensation >= min_comp
}

const fn is_durable_compensation(total_material: i32, side_eval: i32, side_material: i32) -> bool {
    if side_material > -150 || side_material < -650 {
        return false;
    }
    let deficit = -side_material;
    let min_eval = scale_by_phase(total_material, -20, 0, 25);
    let min_comp = deficit + scale_by_phase(total_material, 40, 60, 80);
    let compensation = side_eval - side_material;
    side_eval >= min_eval && side_eval <= 260 && compensation >= min_comp
}

fn weighted_material(board: &Board) -> (i32, i32) {
    const VALUES: [(PieceKind, i32); 5] = [
        (PieceKind::Pawn, 100),
        (PieceKind::Knight, 320),
        (PieceKind::Bishop, 330),
        (PieceKind::Rook, 500),
        (PieceKind::Queen, 950),
    ];
    let mut total = 0;
    let mut white_minus_black = 0;
    for (kind, value) in VALUES {
        let white = i32::try_from(board.pieces(Color::White, kind).len()).unwrap_or(0);
        let black = i32::try_from(board.pieces(Color::Black, kind).len()).unwrap_or(0);
        total += (white + black) * value;
        white_minus_black += (white - black) * value;
    }
    (total, white_minus_black)
}
