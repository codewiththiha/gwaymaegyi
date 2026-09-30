use crate::{Board, Move, MoveKind, Piece, PieceKind, Promotion, Square};

fn append(moves: &mut Vec<Move>, from: Square, to: Square) {
    if matches!(to.rank(), 0 | 7) {
        for promotion in [
            Promotion::Queen,
            Promotion::Rook,
            Promotion::Bishop,
            Promotion::Knight,
        ] {
            moves.push(Move::new(from, to, MoveKind::Promotion(promotion)));
        }
    } else {
        moves.push(Move::new(from, to, MoveKind::Normal));
    }
}

pub(super) fn generate(board: &Board, moves: &mut Vec<Move>) {
    let color = board.side;
    let step = color.pawn_step();
    for from in board.pieces(color, PieceKind::Pawn) {
        if let Some(to) = from
            .offset(0, step)
            .filter(|&to| board.piece_on(to).is_none())
        {
            append(moves, from, to);
            let start_rank = color.home_rank().abs_diff(1);
            if from.rank() == start_rank {
                if let Some(two) = to
                    .offset(0, step)
                    .filter(|&two| board.piece_on(two).is_none())
                {
                    moves.push(Move::new(from, two, MoveKind::Normal));
                }
            }
        }
        for file in [-1, 1] {
            let Some(to) = from.offset(file, step) else {
                continue;
            };
            if board
                .piece_on(to)
                .is_some_and(|piece| piece.color != color && piece.kind != PieceKind::King)
            {
                append(moves, from, to);
            } else if board.en_passant == Some(to) && board.piece_on(to).is_none() {
                let captured = to
                    .offset(0, -step)
                    .and_then(|square| board.piece_on(square));
                if captured == Some(Piece::new(color.opposite(), PieceKind::Pawn)) {
                    moves.push(Move::new(from, to, MoveKind::EnPassant));
                }
            }
        }
    }
}
