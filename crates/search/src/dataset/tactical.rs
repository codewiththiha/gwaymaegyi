//! King-danger, opposite-castling storm, shelter, attack-vector, and outpost predicates.

use gwaymaegyi_core::{Board, Color, PieceKind, Square};

use super::filter::standard_material;

pub(super) const fn both_sides_have_queens(board: &Board) -> bool {
    !board.pieces(Color::White, PieceKind::Queen).is_empty()
        && !board.pieces(Color::Black, PieceKind::Queen).is_empty()
}

fn king_square(board: &Board, color: Color) -> Option<Square> {
    board.pieces(color, PieceKind::King).into_iter().next()
}

pub(super) fn matches_king_danger(board: &Board) -> bool {
    side_king_in_danger(board, Color::White) || side_king_in_danger(board, Color::Black)
}

fn side_king_in_danger(board: &Board, defender: Color) -> bool {
    const ZONE_OFFSETS: [(i8, i8); 16] = [
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-2, 1),
        (-1, 1),
        (0, 1),
        (1, 1),
        (2, 1),
        (-1, 2),
        (0, 2),
        (1, 2),
        (-1, 3),
        (0, 3),
        (1, 3),
    ];
    let Some(king) = king_square(board, defender) else {
        return false;
    };
    let forward: i8 = match defender {
        Color::White => 1,
        Color::Black => -1,
    };
    let mut danger_tenths = 0_i32;
    let mut attack_tenths = 0_i32;
    for (df, dr_forward) in ZONE_OFFSETS {
        let Some(target) = offset_square(king, df, dr_forward * forward) else {
            continue;
        };
        let Some(piece) = board.piece_on(target) else {
            continue;
        };
        if piece.color == defender {
            danger_tenths -= match piece.kind {
                PieceKind::Pawn | PieceKind::Rook => 10,
                PieceKind::Knight | PieceKind::Bishop => 11,
                PieceKind::Queen => 17,
                PieceKind::King => 0,
            };
        } else {
            let value = match piece.kind {
                PieceKind::Pawn => 8,
                PieceKind::Knight => 22,
                PieceKind::Bishop => 20,
                PieceKind::Rook => 30,
                PieceKind::Queen => 60,
                PieceKind::King => 0,
            };
            danger_tenths += value;
            attack_tenths += value;
        }
    }
    danger_tenths > 40 && attack_tenths > 70
}

pub(super) fn matches_opposite_castling_storm(board: &Board) -> bool {
    let (total_material, _) = standard_material(board);
    if total_material <= 4_000 || !both_sides_have_queens(board) || board.has_castling_rights() {
        return false;
    }
    let Some(white_king) = king_square(board, Color::White) else {
        return false;
    };
    let Some(black_king) = king_square(board, Color::Black) else {
        return false;
    };
    let wf = white_king.file();
    let bf = black_king.file();
    let opposite_wings = (wf <= 3 && bf >= 4) || (wf >= 4 && bf <= 3);
    if !opposite_wings {
        return false;
    }
    has_storm_pawn(board, Color::White, black_king)
        || has_storm_pawn(board, Color::Black, white_king)
}

fn has_storm_pawn(board: &Board, attacker: Color, enemy_king: Square) -> bool {
    const STORM_OFFSETS: [(i8, i8); 10] = [
        (-1, 2),
        (0, 2),
        (1, 2),
        (-2, 3),
        (-1, 3),
        (0, 3),
        (1, 3),
        (2, 3),
        (0, 4),
        (1, 4),
    ];
    let toward_attacker: i8 = match attacker {
        Color::White => -1,
        Color::Black => 1,
    };
    let pawns = board.pieces(attacker, PieceKind::Pawn);
    STORM_OFFSETS
        .into_iter()
        .filter_map(|(df, dr)| offset_square(enemy_king, df, dr * toward_attacker))
        .any(|square| pawns.contains(square))
}

pub(super) fn matches_exposed_king_mobility(board: &Board) -> bool {
    let (total_material, _) = standard_material(board);
    if total_material <= 4_000 || !both_sides_have_queens(board) {
        return false;
    }
    no_shelter_score(board, Color::White) > 8 || no_shelter_score(board, Color::Black) > 8
}

fn no_shelter_score(board: &Board, defender: Color) -> i32 {
    const DIRS: [(i8, i8); 8] = [
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ];
    if board.has_castling_rights_for(defender) {
        return 0;
    }
    let Some(king) = king_square(board, defender) else {
        return 0;
    };
    let back_rank: u8 = match defender {
        Color::White => 0,
        Color::Black => 7,
    };
    let mut total = 0;
    for (df, dr) in DIRS {
        let mut cursor = king;
        while let Some(next) = offset_square(cursor, df, dr) {
            cursor = next;
            if cursor.rank() == back_rank {
                continue;
            }
            match board.piece_on(cursor) {
                None => total += 1,
                Some(piece) if piece.color == defender => break,
                Some(piece) if piece.kind == PieceKind::Pawn => {
                    total += 1;
                    break;
                }
                Some(_) => {
                    total += 2;
                    break;
                }
            }
        }
    }
    total
}

pub(super) fn matches_attack_vector(board: &Board) -> bool {
    let (total_material, _) = standard_material(board);
    if total_material <= 3_000 || !both_sides_have_queens(board) {
        return false;
    }
    potential_attack_score(board, Color::White) > 10
        || potential_attack_score(board, Color::Black) > 10
}

fn potential_attack_score(board: &Board, defender: Color) -> i32 {
    const KING_ZONE: [(i8, i8); 11] = [
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
        (-1, 2),
        (0, 2),
        (1, 2),
    ];
    let Some(king) = king_square(board, defender) else {
        return 0;
    };
    let forward: i8 = match defender {
        Color::White => 1,
        Color::Black => -1,
    };
    let kf = i32::from(king.file());
    let kr = i32::from(king.rank());
    let attacker = defender.opposite();
    let mut score = 0;
    for square in board.side_pieces(attacker) {
        let Some(piece) = board.piece_on(square) else {
            continue;
        };
        let weight = match piece.kind {
            PieceKind::Knight | PieceKind::Bishop => 2,
            PieceKind::Rook => 3,
            PieceKind::Queen => 5,
            PieceKind::Pawn | PieceKind::King => continue,
        };
        let in_zone = KING_ZONE
            .into_iter()
            .filter_map(|(df, dr)| offset_square(king, df, dr * forward))
            .any(|zone_sq| zone_sq == square);
        let sf = i32::from(square.file());
        let sr = i32::from(square.rank());
        let df = (kf - sf).abs();
        let dr = (kr - sr).abs();
        let aligned = match piece.kind {
            PieceKind::Knight => df <= 3 && dr <= 3 && ((df + dr) & 1 == 1),
            PieceKind::Bishop => (df - dr).abs() <= 1,
            PieceKind::Rook => df <= 1 || dr <= 1,
            PieceKind::Queen => (df - dr).abs() <= 1 || df <= 1 || dr <= 1,
            PieceKind::Pawn | PieceKind::King => false,
        };
        if in_zone || aligned {
            score += weight;
        }
    }
    score
}

pub(super) fn matches_monster_outpost(board: &Board) -> bool {
    let (total_material, _) = standard_material(board);
    if total_material <= 2_500 || !both_sides_have_queens(board) {
        return false;
    }
    has_monster_outpost(board, Color::White) || has_monster_outpost(board, Color::Black)
}

fn has_monster_outpost(board: &Board, color: Color) -> bool {
    const WHITE_OUTPOST_INDICES: [u8; 10] = [41, 42, 43, 44, 45, 46, 50, 51, 52, 53];
    let opponent = color.opposite();
    if !board.pieces(opponent, PieceKind::Bishop).is_empty() {
        return false;
    }
    let minors = board.pieces(color, PieceKind::Knight) | board.pieces(color, PieceKind::Bishop);
    let enemy_pawns = board.pieces(opponent, PieceKind::Pawn);
    let forward: i8 = match color {
        Color::White => 1,
        Color::Black => -1,
    };
    for base_index in WHITE_OUTPOST_INDICES {
        let index = match color {
            Color::White => base_index,
            Color::Black => base_index ^ 0x38,
        };
        let Some(square) = Square::from_index(index) else {
            continue;
        };
        if !minors.contains(square) {
            continue;
        }
        let left_guard = offset_square(square, -1, forward);
        let right_guard = offset_square(square, 1, forward);
        let guarded = left_guard.is_some_and(|sq| enemy_pawns.contains(sq))
            || right_guard.is_some_and(|sq| enemy_pawns.contains(sq));
        if !guarded {
            return true;
        }
    }
    false
}

pub(super) const fn offset_square(origin: Square, df: i8, dr: i8) -> Option<Square> {
    let Some(file) = origin.file().checked_add_signed(df) else {
        return None;
    };
    let Some(rank) = origin.rank().checked_add_signed(dr) else {
        return None;
    };
    Square::new(file, rank)
}
