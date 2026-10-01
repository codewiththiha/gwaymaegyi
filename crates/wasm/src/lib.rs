//! WASM rules, raw evaluation, capabilities, and stateful engine exports.
//! The same portable implementation powers both browser and native search.

#![expect(
    clippy::missing_errors_doc,
    reason = "Failure details use plain prose rather than Markdown sections."
)]

mod session;
mod snapshot;

pub use session::Engine;
pub use snapshot::EngineReport;

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
/// Invalid FEN, fractional depths, and depths outside zero through six are errors.
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

fn js_error(error: impl std::fmt::Display) -> JsError {
    JsError::new(&error.to_string())
}

/// Raw quantized model units; engine reports use normalized centipawns instead.
#[wasm_bindgen]
pub fn raw_evaluate(fen: &str, model: &str, perspective: &str) -> Result<i32, JsError> {
    let board: Board = fen.parse().map_err(js_error)?;
    let model = match model {
        "balanced" => gwaymaegyi_eval::Model::Balanced,
        "endgame" => gwaymaegyi_eval::Model::Endgame,
        "aggressive" => gwaymaegyi_eval::Model::Aggressive,
        _ => {
            return Err(JsError::new(
                "model must be balanced, endgame, or aggressive",
            ));
        }
    };
    let color = match perspective {
        "white" => gwaymaegyi_core::Color::White,
        "black" => gwaymaegyi_core::Color::Black,
        _ => return Err(JsError::new("perspective must be white or black")),
    };
    Ok(gwaymaegyi_eval::Accumulator::new(&board, model).score_for(color))
}

#[wasm_bindgen]
#[must_use]
pub fn capabilities_json() -> String {
    let modes = gwaymaegyi_search::Mode::ALL
        .into_iter()
        .map(|mode| format!("\"{}\"", mode.as_str()))
        .collect::<Vec<_>>()
        .join(",");
    let skills = gwaymaegyi_search::SkillLevel::NOMINAL_ELO
        .iter()
        .enumerate()
        .map(|(index, elo)| format!("{{\"level\":{},\"elo\":{elo}}}", index + 1))
        .chain(std::iter::once("{\"level\":21,\"elo\":0}".into()))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"version\":\"{}\",\"modes\":[{modes}],\"eloMin\":500,\"eloMax\":3000,\"eloCalibrated\":false,\"skillLevels\":[{skills}],\"performanceProfiles\":[\"full\",\"balanced\",\"responsive\"],\"liveLimits\":true,\"maxDepth\":{},\"maxMultiPv\":5,\"maxHashMiB\":64,\"maxWork\":65536,\"cooperativeSearch\":true,\"chess960\":true,\"simd128\":{},\"nativeSyzygy\":false}}",
        env!("CARGO_PKG_VERSION"),
        gwaymaegyi_search::MAX_DEPTH,
        cfg!(all(target_arch = "wasm32", target_feature = "simd128"))
    )
}

/// Discover only implemented, validated search controls.
#[must_use]
#[wasm_bindgen]
pub fn search_controls_json() -> String {
    let behaviors = gwaymaegyi_search::Behavior::ALL
        .into_iter()
        .map(|item| format!("\"{}\"", item.name()))
        .collect::<Vec<_>>()
        .join(",");
    let parameters = gwaymaegyi_search::Parameter::SPECS
        .into_iter()
        .map(|spec| {
            format!(
                "{{\"name\":\"{}\",\"default\":{},\"min\":{},\"max\":{}}}",
                spec.name, spec.default, spec.min, spec.max
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"behaviors\":[{behaviors}],\"parameters\":[{parameters}]}}")
}

/// Evaluates whether a single training line matches one of the 11 position filters.
#[wasm_bindgen]
pub fn filter_training_line(line: &str, filter: &str) -> Result<bool, JsError> {
    let kind = gwaymaegyi_search::FilterKind::parse(filter)
        .ok_or_else(|| JsError::new("unknown position filter"))?;
    let record: gwaymaegyi_search::TrainingRecord = line.parse().map_err(js_error)?;
    Ok(kind.matches(&record))
}

/// Packs a single text training line into a 32-byte binary bullet record.
#[wasm_bindgen]
pub fn encode_training_line(line: &str) -> Result<Vec<u8>, JsError> {
    let record: gwaymaegyi_search::TrainingRecord = line.parse().map_err(js_error)?;
    let packed = gwaymaegyi_search::BulletRecord::from_training_record(&record).map_err(js_error)?;
    Ok(packed.to_bytes().to_vec())
}

/// Unpacks a 32-byte binary bullet record into a text training line.
#[wasm_bindgen]
pub fn decode_bullet_record(bytes: &[u8]) -> Result<String, JsError> {
    let packed = gwaymaegyi_search::BulletRecord::from_bytes(bytes).map_err(js_error)?;
    let record = packed.to_training_record().map_err(js_error)?;
    Ok(record.to_string())
}
