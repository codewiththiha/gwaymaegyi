//! Native Syzygy WDL adapter using the safe tablebase handle.
//! Filesystem discovery and the tablebase singleton remain outside portable search.

use gwaymaegyi_core::{Bitboard, Board, Color, Move, PieceKind, Square};
use gwaymaegyi_search::{TablebaseProbe, TablebaseRoot, TablebaseWdl};
use pyrrhic_rs::{
    Color as TbColor, DtzProbeValue, EngineAdapter, Piece as TbPiece, TableBases, WdlProbeResult,
};
use std::{fmt, sync::Mutex};

#[derive(Clone)]
struct RulesAdapter;
impl EngineAdapter for RulesAdapter {
    fn pawn_attacks(color: TbColor, square: u64) -> u64 {
        attack(PieceKind::Pawn, color, square, 0)
    }
    fn knight_attacks(square: u64) -> u64 {
        attack(PieceKind::Knight, TbColor::White, square, 0)
    }
    fn bishop_attacks(square: u64, occupied: u64) -> u64 {
        attack(PieceKind::Bishop, TbColor::White, square, occupied)
    }
    fn rook_attacks(square: u64, occupied: u64) -> u64 {
        attack(PieceKind::Rook, TbColor::White, square, occupied)
    }
    fn queen_attacks(square: u64, occupied: u64) -> u64 {
        attack(PieceKind::Queen, TbColor::White, square, occupied)
    }
    fn king_attacks(square: u64) -> u64 {
        attack(PieceKind::King, TbColor::White, square, 0)
    }
}
fn attack(kind: PieceKind, color: TbColor, index: u64, occupied: u64) -> u64 {
    let Ok(index) = u8::try_from(index) else {
        return 0;
    };
    let Some(square) = Square::from_index(index) else {
        return 0;
    };
    let color = if color == TbColor::White {
        Color::White
    } else {
        Color::Black
    };
    Board::attacks_from(kind, color, square, Bitboard(occupied)).0
}

/// Shared, thread-safe Syzygy WDL data. Root DTZ remains a separate non-parallel operation.
pub struct NativeTablebases {
    inner: TableBases<RulesAdapter>,
    root_probe: Mutex<()>,
}
impl fmt::Debug for NativeTablebases {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output
            .debug_struct("NativeTablebases")
            .field("max_pieces", &self.inner.max_pieces())
            .finish_non_exhaustive()
    }
}
impl NativeTablebases {
    /// Opens a platform-separated list of readable directories containing WDL tables.
    /// Only one path is active process-wide because the upstream probe uses global state.
    ///
    /// # Errors
    /// Returns an error when paths cannot be scanned or tablebase initialization fails.
    pub fn open(path: &str) -> Result<Self, String> {
        let paths: Vec<_> = std::env::split_paths(std::ffi::OsStr::new(path))
            .filter(|root| !root.as_os_str().is_empty())
            .collect();
        if paths.is_empty() {
            return Err("SyzygyPath is empty".into());
        }
        let mut has_wdl = false;
        for root in &paths {
            let entries = std::fs::read_dir(root)
                .map_err(|_| "SyzygyPath must contain readable tablebase directories")?;
            let mut root_has_wdl = false;
            for entry in entries {
                let entry = entry.map_err(|_| "SyzygyPath could not be read")?;
                if entry.metadata().is_ok_and(|metadata| metadata.is_file())
                    && entry
                        .path()
                        .extension()
                        .is_some_and(|extension| extension == "rtbw")
                {
                    root_has_wdl = true;
                    break;
                }
            }
            has_wdl |= root_has_wdl;
        }
        if !has_wdl {
            return Err("SyzygyPath contains no WDL (.rtbw) files".into());
        }
        #[cfg(windows)]
        if paths.iter().any(|root| root.is_absolute()) {
            return Err("Pyrrhic's colon-separated Windows path format requires relative tablebase directories".into());
        }
        let normalized = paths
            .iter()
            .map(|root| {
                root.to_str()
                    .ok_or("SyzygyPath must use valid UTF-8 path names")
            })
            .collect::<Result<Vec<_>, _>>()?
            .join(":");
        TableBases::<RulesAdapter>::new(&normalized)
            .map(|inner| Self {
                inner,
                root_probe: Mutex::new(()),
            })
            .map_err(|error| format!("Syzygy initialization failed: {error:?}"))
    }
}
impl NativeTablebases {
    /// Probe WDL with an explicit error result for integration diagnostics.
    /// Positions outside the wrapper's rule boundary return `Ok(None)`.
    ///
    /// # Errors
    /// Returns a diagnostic when the upstream WDL probe rejects a supported position.
    pub fn probe_wdl_checked(&self, board: &Board) -> Result<Option<TablebaseWdl>, String> {
        if board.has_castling_rights()
            || board.halfmove_clock() != 0
            || board.occupied().len() > self.inner.max_pieces()
        {
            return Ok(None);
        }
        let pieces = |color, kind| board.pieces(color, kind).0;
        let white = board.side_pieces(Color::White).0;
        let black = board.side_pieces(Color::Black).0;
        let role = |kind| pieces(Color::White, kind) | pieces(Color::Black, kind);
        let ep = board
            .legal_en_passant_square()
            .map_or(0, |square| u32::try_from(square.index()).unwrap_or(0));
        let result = self
            .inner
            .probe_wdl(
                white,
                black,
                role(PieceKind::King),
                role(PieceKind::Queen),
                role(PieceKind::Rook),
                role(PieceKind::Bishop),
                role(PieceKind::Knight),
                role(PieceKind::Pawn),
                ep,
                board.side_to_move() == Color::White,
            )
            .map_err(|error| format!("Syzygy WDL probe failed: {error:?}"))?;
        Ok(Some(match result {
            WdlProbeResult::Loss => TablebaseWdl::Loss,
            WdlProbeResult::BlessedLoss => TablebaseWdl::BlessedLoss,
            WdlProbeResult::Draw => TablebaseWdl::Draw,
            WdlProbeResult::CursedWin => TablebaseWdl::CursedWin,
            WdlProbeResult::Win => TablebaseWdl::Win,
        }))
    }
}
impl TablebaseProbe for NativeTablebases {
    fn max_pieces(&self) -> u32 {
        self.inner.max_pieces()
    }
    fn probe_wdl(&self, board: &Board) -> Option<TablebaseWdl> {
        self.probe_wdl_checked(board).ok().flatten()
    }

    fn probe_root(&self, board: &Board, allowed: &[Move]) -> Option<TablebaseRoot> {
        if board.has_castling_rights() || board.occupied().len() > self.inner.max_pieces() {
            return None;
        }
        let _guard = self.root_probe.lock().ok()?;
        let pieces = |color, kind| board.pieces(color, kind).0;
        let roles = |kind| pieces(Color::White, kind) | pieces(Color::Black, kind);
        let ep = board
            .legal_en_passant_square()
            .map_or(0, |square| u32::try_from(square.index()).unwrap_or(0));
        let probe = self
            .inner
            .probe_root(
                board.side_pieces(Color::White).0,
                board.side_pieces(Color::Black).0,
                roles(PieceKind::King),
                roles(PieceKind::Queen),
                roles(PieceKind::Rook),
                roles(PieceKind::Bishop),
                roles(PieceKind::Knight),
                roles(PieceKind::Pawn),
                board.halfmove_clock(),
                ep,
                board.side_to_move() == Color::White,
            )
            .ok()?;
        let mut candidates = Vec::new();
        for item in probe.moves.get(..probe.num_moves)? {
            let DtzProbeValue::DtzResult(item) = item else {
                continue;
            };
            let (Some(from), Some(to)) = (
                Square::from_index(item.from_square),
                Square::from_index(item.to_square),
            ) else {
                continue;
            };
            if item.ep && board.legal_en_passant_square() != Some(to) {
                continue;
            }
            let promotion = match item.promotion {
                TbPiece::Queen => 'q',
                TbPiece::Rook => 'r',
                TbPiece::Bishop => 'b',
                TbPiece::Knight => 'n',
                _ => ' ',
            };
            let mut notation = format!("{from}{to}");
            if promotion != ' ' {
                notation.push(promotion);
            }
            let Ok(chess_move) = board.resolve_uci(&notation, false) else {
                continue;
            };
            if !allowed.is_empty() && !allowed.contains(&chess_move) {
                continue;
            }
            let wdl = match item.wdl {
                WdlProbeResult::Loss => TablebaseWdl::Loss,
                WdlProbeResult::BlessedLoss => TablebaseWdl::BlessedLoss,
                WdlProbeResult::Draw => TablebaseWdl::Draw,
                WdlProbeResult::CursedWin => TablebaseWdl::CursedWin,
                WdlProbeResult::Win => TablebaseWdl::Win,
            };
            let category = match wdl {
                TablebaseWdl::Win => 2,
                TablebaseWdl::Loss => 0,
                _ => 1,
            };
            let distance = if wdl == TablebaseWdl::Loss {
                i32::from(item.dtz)
            } else {
                -i32::from(item.dtz)
            };
            candidates.push((
                category,
                distance,
                TablebaseRoot {
                    best_move: chess_move,
                    wdl,
                    dtz: item.dtz,
                },
            ));
        }
        candidates
            .into_iter()
            .max_by_key(|(category, distance, _)| (*category, *distance))
            .map(|(_, _, result)| result)
    }
}

#[cfg(test)]
mod tests {
    use super::{NativeTablebases, RulesAdapter, attack};
    use gwaymaegyi_core::{PieceKind, Square};
    use pyrrhic_rs::{Color as TbColor, EngineAdapter};

    #[test]
    fn native_attack_adapter_preserves_a1_lsb_orientation() {
        assert_eq!(
            RulesAdapter::knight_attacks(1),
            attack(PieceKind::Knight, TbColor::White, 1, 0)
        );
        assert_eq!(RulesAdapter::rook_attacks(0, 0) & 1, 0);
        assert_ne!(RulesAdapter::rook_attacks(0, 0) & (1 << 8), 0);
        assert!(Square::from_index(63).is_some());
    }

    #[test]
    fn unavailable_paths_return_a_checked_error() {
        let result = NativeTablebases::open("/this/path/does/not/exist");
        assert!(result.is_err());
        assert!(NativeTablebases::open("/another/missing/path").is_err());
    }
}
