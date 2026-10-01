//! Owned WASM report getters with lossless node counts and serialized variations.

use gwaymaegyi_search::{Completion, SearchReport, SearchStatus};
use wasm_bindgen::prelude::*;

/// An owned view; JavaScript should free it after copying its primitive data.
#[wasm_bindgen]
#[derive(Debug)]
pub struct EngineReport {
    report: SearchReport,
    chess960: bool,
}
impl EngineReport {
    pub(super) const fn new(report: SearchReport, chess960: bool) -> Self {
        Self { report, chess960 }
    }
}
#[wasm_bindgen]
impl EngineReport {
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn status(&self) -> String {
        match self.report.status {
            SearchStatus::Idle => "idle",
            SearchStatus::Running => "running",
            SearchStatus::Finished(reason) => match reason {
                Completion::Depth => "depth",
                Completion::Nodes => "nodes",
                Completion::Stopped => "stopped",
                Completion::Terminal => "terminal",
            },
        }
        .into()
    }
    #[must_use]
    #[wasm_bindgen(getter)]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "WASM exports cannot be const functions."
    )]
    pub fn finished(&self) -> bool {
        matches!(self.report.status, SearchStatus::Finished(_))
    }
    #[must_use]
    #[wasm_bindgen(getter)]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "WASM exports cannot be const functions."
    )]
    pub fn depth(&self) -> u8 {
        self.report.depth
    }
    #[must_use]
    #[wasm_bindgen(getter)]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "WASM exports cannot be const functions."
    )]
    pub fn selective_depth(&self) -> u8 {
        self.report.selective_depth
    }
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn nodes(&self) -> String {
        self.report.nodes.to_string()
    }
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn best_move(&self) -> Option<String> {
        self.report
            .best_move
            .map(|chess_move| chess_move.to_uci(self.chess960))
    }
    #[must_use]
    #[wasm_bindgen(getter)]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "WASM exports cannot be const functions."
    )]
    pub fn score_cp(&self) -> Option<i32> {
        self.report.score_cp
    }
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn mate(&self) -> Option<i32> {
        self.report.mate_in()
    }
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn pv(&self) -> Vec<String> {
        self.report
            .variations
            .iter()
            .find(|line| line.moves.first().copied() == self.report.best_move)
            .map_or_else(
                || self.best_move().into_iter().collect(),
                |line| {
                    line.moves
                        .iter()
                        .map(|chess_move| chess_move.to_uci(self.chess960))
                        .collect()
                },
            )
    }
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn variations_json(&self) -> String {
        let lines = self
            .report
            .variations
            .iter()
            .map(|line| {
                let moves = line
                    .moves
                    .iter()
                    .map(|chess_move| format!("\"{}\"", chess_move.to_uci(self.chess960)))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{{\"scoreCp\":{},\"pv\":[{moves}]}}", line.score_cp)
            })
            .collect::<Vec<_>>()
            .join(",");
        format!("[{lines}]")
    }
}
