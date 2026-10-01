mod pawns;

use super::attacks::piece_attacks;
use crate::{Bitboard, Board, Move, MoveKind, PieceKind};

pub(super) fn legal_successors(board: &Board) -> Vec<super::Successor> {
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
    super::castling::generate(board, &mut moves);
    moves
        .into_iter()
        .filter_map(|chess_move| {
            let child = board.after_generated_move(chess_move)?;
            (!child.in_check(board.side)).then_some(super::Successor {
                chess_move,
                board: child,
            })
        })
        .collect()
}
