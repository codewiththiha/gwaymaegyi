use std::{fmt, str::FromStr};

use super::castling::rook_for_symbol;
use crate::{Board, Color, FenError, Piece, PieceKind, Square};

const fn parse_piece(symbol: char) -> Option<Piece> {
    let kind = match symbol.to_ascii_lowercase() {
        'p' => PieceKind::Pawn,
        'n' => PieceKind::Knight,
        'b' => PieceKind::Bishop,
        'r' => PieceKind::Rook,
        'q' => PieceKind::Queen,
        'k' => PieceKind::King,
        _ => return None,
    };
    let color = if symbol.is_ascii_uppercase() {
        Color::White
    } else {
        Color::Black
    };
    Some(Piece::new(color, kind))
}

fn piece_symbol(piece: Piece) -> char {
    let symbol: char = match piece.kind {
        PieceKind::Pawn => 'p',
        PieceKind::Knight => 'n',
        PieceKind::Bishop => 'b',
        PieceKind::Rook => 'r',
        PieceKind::Queen => 'q',
        PieceKind::King => 'k',
    };
    if piece.color == Color::White {
        symbol.to_ascii_uppercase()
    } else {
        symbol
    }
}

fn placement(board: &mut Board, value: &str) -> Result<(), FenError> {
    let ranks: Vec<_> = value.split('/').collect();
    if ranks.len() != 8 {
        return Err(FenError::Placement);
    }
    for (index, text) in ranks.into_iter().enumerate() {
        let rank = 7 - u8::try_from(index).map_err(|_| FenError::Placement)?;
        let mut file = 0u8;
        for symbol in text.chars() {
            if let Some(skip) = symbol.to_digit(10).filter(|&skip| (1..=8).contains(&skip)) {
                file = file
                    .checked_add(u8::try_from(skip).map_err(|_| FenError::Placement)?)
                    .ok_or(FenError::Placement)?;
            } else {
                let piece = parse_piece(symbol).ok_or(FenError::Placement)?;
                let square = Square::new(file, rank).ok_or(FenError::Placement)?;
                if piece.kind == PieceKind::Pawn && matches!(rank, 0 | 7) {
                    return Err(FenError::Placement);
                }
                board.place(square, piece);
                file += 1;
            }
            if file > 8 {
                return Err(FenError::Placement);
            }
        }
        if file != 8 {
            return Err(FenError::Placement);
        }
    }
    for color in [Color::White, Color::Black] {
        if board.pieces(color, PieceKind::King).len() != 1 {
            return Err(FenError::Kings(color));
        }
    }
    Ok(())
}

fn castling(board: &mut Board, value: &str) -> Result<(), FenError> {
    if value == "-" {
        return Ok(());
    }
    if value.is_empty() {
        return Err(FenError::Castling);
    }
    for symbol in value.chars() {
        let color = if symbol.is_ascii_uppercase() {
            Color::White
        } else {
            Color::Black
        };
        let king = board.king(color).ok_or(FenError::Castling)?;
        if king.rank() != color.home_rank() {
            return Err(FenError::Castling);
        }
        let rook = match symbol.to_ascii_lowercase() {
            'k' => rook_for_symbol(board, color, true),
            'q' => rook_for_symbol(board, color, false),
            'a'..='h' => Square::new(
                u8::try_from(u32::from(symbol.to_ascii_lowercase()))
                    .map_err(|_| FenError::Castling)?
                    - b'a',
                color.home_rank(),
            ),
            _ => None,
        }
        .ok_or(FenError::Castling)?;
        if board.piece_on(rook) != Some(Piece::new(color, PieceKind::Rook))
            || rook.file() == king.file()
        {
            return Err(FenError::Castling);
        }
        let side = usize::from(rook.file() > king.file());
        let slot = &mut board.castling.rooks[color.index()][side];
        if slot.is_some() {
            return Err(FenError::Castling);
        }
        *slot = Some(rook);
    }
    Ok(())
}

impl FromStr for Board {
    type Err = FenError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let fields: Vec<_> = value.split_whitespace().collect();
        let [pieces, side, rights, ep, halfmove, fullmove] = fields.as_slice() else {
            return Err(FenError::FieldCount);
        };
        let mut board = Self::empty();
        placement(&mut board, pieces)?;
        board.side = match *side {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err(FenError::SideToMove),
        };
        castling(&mut board, rights)?;
        if *ep != "-" {
            let square: Square = ep.parse().map_err(|_| FenError::EnPassant)?;
            let rank = if board.side == Color::White { 5 } else { 2 };
            let captured = square
                .offset(0, -board.side.pawn_step())
                .and_then(|square| board.piece_on(square));
            if square.rank() != rank
                || board.piece_on(square).is_some()
                || captured != Some(Piece::new(board.side.opposite(), PieceKind::Pawn))
            {
                return Err(FenError::EnPassant);
            }
            board.en_passant = Some(square);
        }
        board.halfmove = halfmove.parse().map_err(|_| FenError::HalfmoveClock)?;
        board.fullmove = fullmove.parse().map_err(|_| FenError::FullmoveNumber)?;
        if board.fullmove == 0 {
            return Err(FenError::FullmoveNumber);
        }
        board.identity = board.recomputed_key();
        Ok(board)
    }
}

impl fmt::Display for Board {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rank in (0..8).rev() {
            let mut empty = 0;
            for file in 0..8 {
                let Some(square) = Square::new(file, rank) else {
                    return Err(fmt::Error);
                };
                if let Some(piece) = self.piece_on(square) {
                    if empty != 0 {
                        write!(formatter, "{empty}")?;
                        empty = 0;
                    }
                    write!(formatter, "{}", piece_symbol(piece))?;
                } else {
                    empty += 1;
                }
            }
            if empty != 0 {
                write!(formatter, "{empty}")?;
            }
            if rank != 0 {
                formatter.write_str("/")?;
            }
        }
        write!(
            formatter,
            " {} ",
            if self.side == Color::White { 'w' } else { 'b' }
        )?;
        let mut any = false;
        for color in [Color::White, Color::Black] {
            for side in [1, 0] {
                if let Some(rook) = self.castling.rooks[color.index()][side] {
                    let symbol = match (side, rook.file()) {
                        (1, 7) => 'k',
                        (0, 0) => 'q',
                        _ => char::from(b'a' + rook.file()),
                    };
                    write!(
                        formatter,
                        "{}",
                        if color == Color::White {
                            symbol.to_ascii_uppercase()
                        } else {
                            symbol
                        }
                    )?;
                    any = true;
                }
            }
        }
        if !any {
            formatter.write_str("-")?;
        }
        let ep = self
            .en_passant
            .map_or_else(|| "-".to_owned(), |square| square.to_string());
        write!(formatter, " {ep} {} {}", self.halfmove, self.fullmove)
    }
}
