mod pawns;

use crate::{Bitboard, Board, Move, MoveKind, PieceKind, attacks::piece_attacks};

pub(super) fn legal_moves(board: &Board) -> Vec<Move> {
    let mut moves = Vec::with_capacity(64);
    pawns::generate(board, &mut moves);
    let occupied = board.occupied();
    let ours = board.colors[board.side.index()];
    let enemy_king = board.pieces(board.side.opposite(), PieceKind::King);
    for kind in [
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
        PieceKind::King,
    ] {
        for from in board.pieces(board.side, kind) {
            let targets = Bitboard(
                piece_attacks(kind, board.side, from, occupied).0 & !ours.0 & !enemy_king.0,
            );
            for to in targets {
                moves.push(Move::new(from, to, MoveKind::Normal));
            }
        }
    }
    crate::castling::generate(board, &mut moves);
    moves.retain(|&chess_move| {
        board
            .after_generated_move(chess_move)
            .is_some_and(|child| !child.in_check(board.side))
    });
    moves
}
