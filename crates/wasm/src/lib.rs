//! Browser bindings share the native rules without threads or filesystem access.

#![expect(
    clippy::missing_errors_doc,
    reason = "Failure details use plain prose rather than Markdown sections."
)]

use gwaymaegyi_core::Board;
use wasm_bindgen::prelude::*;

/// Canonicalizes standard and Chess960 FEN; malformed positions return an error.
#[wasm_bindgen]
pub fn normalize_fen(fen: &str) -> Result<String, JsValue> {
    parse_board(fen).map(|board| board.to_string())
}

/// Returns UCI moves in the requested convention, or a FEN validation error.
#[wasm_bindgen]
pub fn legal_moves(fen: &str, chess960: bool) -> Result<String, JsValue> {
    let board = parse_board(fen)?;
    let moves: Vec<_> = board
        .legal_moves()
        .into_iter()
        .map(|chess_move| chess_move.to_uci(chess960))
        .collect();
    Ok(moves.join(" "))
}

/// Returns the resulting FEN or reports a notation or legality failure.
#[wasm_bindgen]
pub fn play_uci(fen: &str, notation: &str, chess960: bool) -> Result<String, JsValue> {
    parse_board(fen)?
        .play_uci(notation, chess960)
        .map(|board| board.to_string())
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Returns a decimal count to preserve integer precision in JavaScript.
/// Invalid FEN or a non-integer depth outside zero through six returns an error.
#[wasm_bindgen]
pub fn perft(fen: &str, depth: f64) -> Result<String, JsValue> {
    // Adding zero normalizes negative zero without rounding fractional inputs.
    let requested = depth + 0.0;
    let valid = (0_u8..=6).find(|&value| f64::from(value).total_cmp(&requested).is_eq());
    let Some(depth) = valid else {
        return Err(JsValue::from_str(
            "WASM perft depth must be an integer between zero and six",
        ));
    };
    let board = parse_board(fen)?;
    Ok(gwaymaegyi_core::perft(&board, depth).to_string())
}

fn parse_board(fen: &str) -> Result<Board, JsValue> {
    fen.parse::<Board>()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}
