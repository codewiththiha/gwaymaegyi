//! Browser bindings share the native rules without threads or filesystem access.

use gwaymaegyi_core::Board;
use wasm_bindgen::prelude::*;

/// Canonicalizes structurally valid standard and Chess960 FEN positions.
#[wasm_bindgen]
pub fn normalize_fen(fen: &str) -> Result<String, JsValue> {
    parse_board(fen).map(|board| board.to_string())
}

/// Returns space-separated UCI moves using the requested castling convention.
#[wasm_bindgen]
pub fn legal_moves(fen: &str, chess960: bool) -> Result<String, JsValue> {
    let board = parse_board(fen)?;
    let moves: Vec<_> = board.legal_moves().into_iter().map(|chess_move| chess_move.to_uci(chess960)).collect();
    Ok(moves.join(" "))
}

/// Returns the resulting FEN or reports a notation or legality failure.
#[wasm_bindgen]
pub fn play_uci(fen: &str, notation: &str, chess960: bool) -> Result<String, JsValue> {
    parse_board(fen)?.play_uci(notation, chess960).map(|board| board.to_string()).map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Returns a decimal count so JavaScript does not lose u64 precision.
#[wasm_bindgen]
pub fn perft(fen: &str, depth: u8) -> Result<String, JsValue> {
    if depth > 6 { return Err(JsValue::from_str("WASM perft depth must be between zero and six")); }
    let board = parse_board(fen)?;
    Ok(gwaymaegyi_core::perft(&board, depth).to_string())
}

fn parse_board(fen: &str) -> Result<Board, JsValue> {
    fen.parse::<Board>().map_err(|error| JsValue::from_str(&error.to_string()))
}
