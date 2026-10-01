//! Text and 32-byte packed training records with checked round-trip conversion.

#![expect(
    clippy::missing_errors_doc,
    reason = "Conversion and parsing errors are described in plain prose."
)]

use gwaymaegyi_core::{Board, Color, Piece, PieceKind, Square};
use std::{error::Error, fmt, str::FromStr};

pub const BULLET_RECORD_BYTES: usize = 32;

/// Failures when parsing or converting text and binary training records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DatasetError {
    InvalidFormat,
    InvalidFen(String),
    InvalidScore,
    InvalidResult,
    InvalidBulletLength,
    InvalidBulletRecord,
}

impl fmt::Display for DatasetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => {
                formatter.write_str("training line must have 'FEN | score | result' fields")
            }
            Self::InvalidFen(reason) => write!(formatter, "invalid training FEN: {reason}"),
            Self::InvalidScore => formatter.write_str("training score must be a 16-bit integer"),
            Self::InvalidResult => formatter.write_str("training result must be 0.0, 0.5, or 1.0"),
            Self::InvalidBulletLength => {
                formatter.write_str("packed bullet input length must be a multiple of 32 bytes")
            }
            Self::InvalidBulletRecord => formatter.write_str("invalid packed bullet record"),
        }
    }
}

impl Error for DatasetError {}

/// White-perspective game outcome stored in training data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameResult {
    BlackWin,
    Draw,
    WhiteWin,
}

impl GameResult {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BlackWin => "0.0",
            Self::Draw => "0.5",
            Self::WhiteWin => "1.0",
        }
    }

    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::BlackWin => 0,
            Self::Draw => 1,
            Self::WhiteWin => 2,
        }
    }

    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::BlackWin),
            1 => Some(Self::Draw),
            2 => Some(Self::WhiteWin),
            _ => None,
        }
    }
}

impl FromStr for GameResult {
    type Err = DatasetError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "0" | "0.0" | "0.00" => Ok(Self::BlackWin),
            "0.5" | "0.50" => Ok(Self::Draw),
            "1" | "1.0" | "1.00" => Ok(Self::WhiteWin),
            _ => Err(DatasetError::InvalidResult),
        }
    }
}

/// A validated training position with White-perspective score and outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrainingRecord {
    board: Board,
    fen: String,
    score: i16,
    result: GameResult,
}

impl TrainingRecord {
    /// Creates a record from a board, White-perspective score, and outcome.
    #[must_use]
    pub fn new(board: Board, score: i16, result: GameResult) -> Self {
        Self {
            fen: board.to_string(),
            board,
            score,
            result,
        }
    }

    #[must_use]
    pub const fn board(&self) -> &Board {
        &self.board
    }

    #[must_use]
    pub const fn fen(&self) -> &str {
        self.fen.as_str()
    }

    #[must_use]
    pub const fn score(&self) -> i16 {
        self.score
    }

    #[must_use]
    pub const fn result(&self) -> GameResult {
        self.result
    }
}

impl fmt::Display for TrainingRecord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} | {} | {}",
            self.fen,
            self.score,
            self.result.as_str()
        )
    }
}

impl FromStr for TrainingRecord {
    type Err = DatasetError;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let mut parts = line.split(" | ");
        let Some(raw_fen) = parts.next().map(str::trim) else {
            return Err(DatasetError::InvalidFormat);
        };
        let Some(raw_score) = parts.next().map(str::trim) else {
            return Err(DatasetError::InvalidFormat);
        };
        let Some(raw_result) = parts.next().map(str::trim) else {
            return Err(DatasetError::InvalidFormat);
        };
        if parts.next().is_some() || raw_fen.is_empty() {
            return Err(DatasetError::InvalidFormat);
        }
        let board = parse_training_board(raw_fen)?;
        let score: i16 = raw_score.parse().map_err(|_| DatasetError::InvalidScore)?;
        let result: GameResult = raw_result.parse()?;
        Ok(Self {
            board,
            fen: raw_fen.to_owned(),
            score,
            result,
        })
    }
}

fn parse_training_board(raw_fen: &str) -> Result<Board, DatasetError> {
    let fields: Vec<_> = raw_fen.split_whitespace().collect();
    if fields.len() == 6 && fields[5] == "0" {
        let normalized = format!(
            "{} {} {} {} {} 1",
            fields[0], fields[1], fields[2], fields[3], fields[4]
        );
        return normalized
            .parse()
            .map_err(|error: gwaymaegyi_core::FenError| {
                DatasetError::InvalidFen(error.to_string())
            });
    }
    raw_fen
        .parse()
        .map_err(|error: gwaymaegyi_core::FenError| DatasetError::InvalidFen(error.to_string()))
}

/// Packed 32-byte binary record matching the reference bulletformat layout.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BulletRecord {
    occupancy: u64,
    pieces: [u8; 16],
    score: i16,
    result: GameResult,
    white_king: Square,
    black_king: Square,
}

impl BulletRecord {
    /// Packs a validated training record into the 32-byte binary representation.
    pub fn from_training_record(record: &TrainingRecord) -> Result<Self, DatasetError> {
        let board = record.board();
        let occupied = board.occupied();
        if occupied.len() > 32 {
            return Err(DatasetError::InvalidBulletRecord);
        }
        let white_king = board
            .pieces(Color::White, PieceKind::King)
            .into_iter()
            .next()
            .ok_or(DatasetError::InvalidBulletRecord)?;
        let black_king = board
            .pieces(Color::Black, PieceKind::King)
            .into_iter()
            .next()
            .ok_or(DatasetError::InvalidBulletRecord)?;
        let mut pieces = [0_u8; 16];
        for (index, square) in occupied.into_iter().enumerate() {
            let piece = board
                .piece_on(square)
                .ok_or(DatasetError::InvalidBulletRecord)?;
            let code = encode_piece(piece);
            if index & 1 == 0 {
                pieces[index / 2] |= code;
            } else {
                pieces[index / 2] |= code << 4;
            }
        }
        Ok(Self {
            occupancy: occupied.0,
            pieces,
            score: record.score(),
            result: record.result(),
            white_king,
            black_king,
        })
    }

    /// Decodes a 32-byte slice into a validated packed record.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DatasetError> {
        let Ok(raw): Result<&[u8; BULLET_RECORD_BYTES], _> = bytes.try_into() else {
            return Err(DatasetError::InvalidBulletLength);
        };
        let occupancy = u64::from_le_bytes([
            raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
        ]);
        if occupancy.count_ones() > 32 {
            return Err(DatasetError::InvalidBulletRecord);
        }
        let mut pieces = [0_u8; 16];
        pieces.copy_from_slice(&raw[8..24]);
        let score = i16::from_le_bytes([raw[24], raw[25]]);
        let result = GameResult::from_code(raw[26]).ok_or(DatasetError::InvalidBulletRecord)?;
        let placement = decode_placement(occupancy, &pieces)?;
        let board: Board = format!("{placement} w - - 0 1")
            .parse()
            .map_err(|_| DatasetError::InvalidBulletRecord)?;
        let white_king = board
            .pieces(Color::White, PieceKind::King)
            .into_iter()
            .next()
            .ok_or(DatasetError::InvalidBulletRecord)?;
        let black_king = board
            .pieces(Color::Black, PieceKind::King)
            .into_iter()
            .next()
            .ok_or(DatasetError::InvalidBulletRecord)?;
        Ok(Self {
            occupancy,
            pieces,
            score,
            result,
            white_king,
            black_king,
        })
    }

    /// Serializes this record into its 32-byte little-endian binary form.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; BULLET_RECORD_BYTES] {
        let mut out = [0_u8; BULLET_RECORD_BYTES];
        out[0..8].copy_from_slice(&self.occupancy.to_le_bytes());
        out[8..24].copy_from_slice(&self.pieces);
        out[24..26].copy_from_slice(&self.score.to_le_bytes());
        out[26] = self.result.code();
        out[27] = u8::try_from(self.white_king.index()).unwrap_or(0);
        out[28] = u8::try_from(self.black_king.index() ^ 56).unwrap_or(0);
        out
    }

    /// Converts this packed record into a validated training record.
    pub fn to_training_record(&self) -> Result<TrainingRecord, DatasetError> {
        let placement = decode_placement(self.occupancy, &self.pieces)?;
        let fen = format!("{placement} w - - 0 0");
        let board = parse_training_board(&fen)?;
        Ok(TrainingRecord {
            board,
            fen,
            score: self.score,
            result: self.result,
        })
    }
}

const fn encode_piece(piece: Piece) -> u8 {
    let role = match piece.kind {
        PieceKind::Pawn => 0,
        PieceKind::Knight => 1,
        PieceKind::Bishop => 2,
        PieceKind::Rook => 3,
        PieceKind::Queen => 4,
        PieceKind::King => 5,
    };
    let color_offset = match piece.color {
        Color::White => 0,
        Color::Black => 8,
    };
    color_offset + role
}

const fn decode_piece_symbol(nibble: u8) -> Option<char> {
    match nibble {
        0 => Some('P'),
        1 => Some('N'),
        2 => Some('B'),
        3 => Some('R'),
        4 => Some('Q'),
        5 => Some('K'),
        8 => Some('p'),
        9 => Some('n'),
        10 => Some('b'),
        11 => Some('r'),
        12 => Some('q'),
        13 => Some('k'),
        _ => None,
    }
}

fn decode_placement(occupancy: u64, pieces: &[u8; 16]) -> Result<String, DatasetError> {
    let mut mailbox = [None; 64];
    let mut remaining = occupancy;
    let mut index = 0_usize;
    while remaining != 0 {
        let square = usize::try_from(remaining.trailing_zeros())
            .map_err(|_| DatasetError::InvalidBulletRecord)?;
        let byte = pieces
            .get(index / 2)
            .copied()
            .ok_or(DatasetError::InvalidBulletRecord)?;
        let nibble = if index & 1 == 0 {
            byte & 0x0F
        } else {
            byte >> 4
        };
        let symbol = decode_piece_symbol(nibble).ok_or(DatasetError::InvalidBulletRecord)?;
        mailbox[square] = Some(symbol);
        remaining &= remaining - 1;
        index += 1;
    }
    let mut placement = String::with_capacity(64);
    for rank in (0..8).rev() {
        let mut empty = 0_u8;
        for file in 0..8 {
            if let Some(symbol) = mailbox[rank * 8 + file] {
                if empty > 0 {
                    placement.push(char::from(b'0' + empty));
                    empty = 0;
                }
                placement.push(symbol);
            } else {
                empty += 1;
            }
        }
        if empty > 0 {
            placement.push(char::from(b'0' + empty));
        }
        if rank > 0 {
            placement.push('/');
        }
    }
    Ok(placement)
}

/// Decodes a stream of 32-byte packed records into training records.
pub fn decode_bullet_records(bytes: &[u8]) -> Result<Vec<TrainingRecord>, DatasetError> {
    let chunks = bytes.chunks_exact(BULLET_RECORD_BYTES);
    if !chunks.remainder().is_empty() {
        return Err(DatasetError::InvalidBulletLength);
    }
    chunks
        .map(|chunk| BulletRecord::from_bytes(chunk)?.to_training_record())
        .collect()
}

/// Encodes text training lines into concatenated 32-byte packed records.
pub fn encode_bullet_records(lines: &str) -> Result<Vec<u8>, DatasetError> {
    let mut output = Vec::new();
    for line in lines.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let record: TrainingRecord = line.parse()?;
        let packed = BulletRecord::from_training_record(&record)?;
        output.extend_from_slice(&packed.to_bytes());
    }
    Ok(output)
}
