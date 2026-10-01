//! Stateful WASM controls delegating to the portable engine.
//! Owned reports are separate views; JavaScript must release them after copying.

#![expect(
    clippy::missing_errors_doc,
    reason = "Binding failures are exposed as JavaScript errors."
)]

mod input;
use crate::{EngineReport, js_error};
use gwaymaegyi_core::{ClaimableDraw, Color, DrawReason, Outcome, START_FEN};
use gwaymaegyi_search::{Engine as CoreEngine, SearchLimits, SkillLevel, Strength};
use wasm_bindgen::prelude::*;

/// Stateful browser API; hosts decide when to advance each retained continuation.
#[wasm_bindgen]
#[derive(Debug)]
pub struct Engine {
    inner: CoreEngine,
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Self, JsError> {
        Ok(Self {
            inner: CoreEngine::new().map_err(js_error)?,
        })
    }

    pub fn configure(
        &mut self,
        mode: &str,
        elo: f64,
        hash_mib: f64,
        multi_pv: f64,
        chess960: bool,
        seed: &str,
    ) -> Result<(), JsError> {
        let mut options = self.inner.options();
        options.set_mode(mode.parse().map_err(js_error)?);
        options.set_elo(input::integer(elo)?).map_err(js_error)?;
        options
            .set_hash_mib(input::integer(hash_mib)?)
            .map_err(js_error)?;
        options
            .set_multi_pv(input::integer(multi_pv)?)
            .map_err(js_error)?;
        options.set_chess960(chess960);
        options.set_seed(seed.parse().map_err(js_error)?);
        self.inner.configure(options).map_err(js_error)
    }
    pub fn set_mode(&mut self, mode: &str) -> Result<(), JsError> {
        let mut options = self.inner.options();
        options.set_mode(mode.parse().map_err(js_error)?);
        self.inner.configure(options).map_err(js_error)
    }
    pub fn set_elo(&mut self, elo: f64) -> Result<(), JsError> {
        let mut options = self.inner.options();
        options.set_elo(input::integer(elo)?).map_err(js_error)?;
        self.inner.configure(options).map_err(js_error)
    }
    /// Validated preset control; level 21 disables deliberate strength reduction.
    pub fn set_skill_level(&mut self, level: f64) -> Result<(), JsError> {
        let level = SkillLevel::new(input::integer(level)?).map_err(js_error)?;
        let mut options = self.inner.options();
        options.set_skill_level(level);
        self.inner.configure(options).map_err(js_error)
    }

    pub fn configure_skill(
        &mut self,
        mode: &str,
        level: f64,
        hash_mib: f64,
        multi_pv: f64,
        chess960: bool,
        seed: &str,
    ) -> Result<(), JsError> {
        let level = SkillLevel::new(input::integer(level)?).map_err(js_error)?;
        self.configure(
            mode,
            f64::from(level.nominal_elo().unwrap_or(0)),
            hash_mib,
            multi_pv,
            chess960,
            seed,
        )
    }

    pub fn set_hash_mib(&mut self, size: f64) -> Result<(), JsError> {
        let mut options = self.inner.options();
        options
            .set_hash_mib(input::integer(size)?)
            .map_err(js_error)?;
        self.inner.configure(options).map_err(js_error)
    }

    pub fn set_multi_pv(&mut self, count: f64) -> Result<(), JsError> {
        let mut options = self.inner.options();
        options
            .set_multi_pv(input::integer(count)?)
            .map_err(js_error)?;
        self.inner.configure(options).map_err(js_error)
    }

    pub fn set_behavior(&mut self, name: &str, enabled: bool) -> Result<(), JsError> {
        let mut options = self.inner.options();
        let mut tuning = options.tuning();
        tuning.set_behavior(
            gwaymaegyi_search::Behavior::parse(name).map_err(js_error)?,
            enabled,
        );
        options.set_tuning(tuning);
        self.inner.configure(options).map_err(js_error)
    }
    pub fn set_parameter(&mut self, name: &str, value: f64) -> Result<(), JsError> {
        let mut options = self.inner.options();
        let mut tuning = options.tuning();
        tuning
            .set(
                gwaymaegyi_search::Parameter::parse(name).map_err(js_error)?,
                input::integer(value)?,
            )
            .map_err(js_error)?;
        options.set_tuning(tuning);
        self.inner.configure(options).map_err(js_error)
    }
    #[must_use]
    pub fn tuning_json(&self) -> String {
        let tuning = self.inner.options().tuning();
        let behaviors = gwaymaegyi_search::Behavior::ALL
            .into_iter()
            .map(|item| format!("\"{}\":{}", item.name(), tuning.enabled(item)))
            .collect::<Vec<_>>()
            .join(",");
        let parameters = gwaymaegyi_search::Parameter::SPECS
            .into_iter()
            .map(|spec| format!("\"{}\":{}", spec.name, tuning.get(spec.parameter)))
            .collect::<Vec<_>>()
            .join(",");
        format!("{{\"behaviors\":{{{behaviors}}},\"parameters\":{{{parameters}}}}}")
    }

    pub fn reset(&mut self) -> Result<(), JsError> {
        self.inner.set_position(START_FEN, &[]).map_err(js_error)
    }
    #[expect(
        clippy::needless_pass_by_value,
        reason = "WASM transfers owned string arrays across its ABI."
    )]
    pub fn set_position(&mut self, fen: &str, moves: Vec<String>) -> Result<(), JsError> {
        let borrowed: Vec<_> = moves.iter().map(String::as_str).collect();
        self.inner.set_position(fen, &borrowed).map_err(js_error)
    }
    pub fn play_uci(&mut self, notation: &str) -> Result<(), JsError> {
        self.inner.play_uci(notation).map_err(js_error)
    }
    /// Maximum supported compute budget; selected approximate strength caps still apply.
    pub fn start_full(&mut self) -> Result<EngineReport, JsError> {
        self.inner.start(SearchLimits::full()).map_err(js_error)?;
        Ok(self.report())
    }

    /// Running budget changes retain the exact continuation; invalid updates change nothing.
    pub fn set_limits(&mut self, depth: f64, nodes: &str) -> Result<EngineReport, JsError> {
        self.inner
            .set_limits(input::limits(depth, nodes)?)
            .map_err(js_error)?;
        Ok(self.report())
    }

    /// Calculate the adaptive worker soft limit while preserving a hard cap.
    pub fn soft_time_limit_ms(
        &self,
        original_opt_ms: &str,
        max_ms: &str,
        best_move_nodes: &str,
        nodes: &str,
        stability: f64,
        score_delta: f64,
    ) -> Result<String, JsError> {
        let parse = |value: &str| {
            value
                .parse::<u64>()
                .map_err(|_| JsError::new("time and node counts must be unsigned decimal strings"))
        };
        let soft_ms = self
            .inner
            .options()
            .tuning()
            .soft_limit_ms(
                parse(original_opt_ms)?,
                parse(max_ms)?,
                parse(best_move_nodes)?,
                parse(nodes)?,
                input::integer::<u32>(stability)?,
                input::integer::<i32>(score_delta)?,
            )
            .ok_or_else(|| JsError::new("time budgets must be positive"))?;
        Ok(soft_ms.to_string())
    }

    #[must_use]
    pub fn limits_json(&self) -> String {
        match (self.inner.requested_limits(), self.inner.effective_limits()) {
            (Some(requested), Some(effective)) => format!(
                "{{\"requested\":{{\"depth\":{},\"nodes\":\"{}\"}},\"effective\":{{\"depth\":{},\"nodes\":\"{}\"}}}}",
                requested.depth, requested.nodes, effective.depth, effective.nodes
            ),
            _ => "null".into(),
        }
    }

    pub fn start(&mut self, depth: f64, nodes: &str) -> Result<EngineReport, JsError> {
        self.inner
            .start(input::limits(depth, nodes)?)
            .map_err(js_error)?;
        Ok(self.report())
    }
    #[expect(
        clippy::needless_pass_by_value,
        reason = "WASM transfers owned string arrays across its ABI."
    )]
    pub fn start_moves(
        &mut self,
        depth: f64,
        nodes: &str,
        roots: Vec<String>,
    ) -> Result<EngineReport, JsError> {
        let borrowed: Vec<_> = roots.iter().map(String::as_str).collect();
        self.inner
            .start_moves(input::limits(depth, nodes)?, &borrowed)
            .map_err(js_error)?;
        Ok(self.report())
    }
    pub fn step(&mut self, work: f64) -> Result<EngineReport, JsError> {
        self.inner
            .step(input::integer(work)?)
            .map_err(js_error)
            .map(|report| EngineReport::new(report, self.inner.options().chess960()))
    }
    pub fn stop(&mut self) -> EngineReport {
        self.inner.stop();
        self.report()
    }
    #[must_use]
    pub fn report(&self) -> EngineReport {
        EngineReport::new(self.inner.report(), self.inner.options().chess960())
    }
    #[must_use]
    pub fn evaluate(&self) -> i32 {
        self.inner.evaluate()
    }

    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn skill_level(&self) -> Option<u8> {
        self.inner.options().skill_level().map(SkillLevel::value)
    }

    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn mode(&self) -> String {
        self.inner.options().mode().as_str().into()
    }
    #[must_use]
    #[wasm_bindgen(getter)]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "WASM exports cannot be const functions."
    )]
    pub fn elo(&self) -> u16 {
        match self.inner.options().strength() {
            Strength::Full => 0,
            Strength::Approximate(elo) => elo,
        }
    }
    #[must_use]
    #[wasm_bindgen(getter)]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "WASM exports cannot be const functions."
    )]
    pub fn hash_mib(&self) -> u32 {
        self.inner.options().hash_mib()
    }
    #[must_use]
    #[wasm_bindgen(getter)]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "WASM exports cannot be const functions."
    )]
    pub fn multi_pv(&self) -> u8 {
        self.inner.options().multi_pv()
    }
    #[must_use]
    #[wasm_bindgen(getter)]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "WASM exports cannot be const functions."
    )]
    pub fn chess960(&self) -> bool {
        self.inner.options().chess960()
    }
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn seed(&self) -> String {
        self.inner.options().seed().to_string()
    }
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn fen(&self) -> String {
        self.inner.game().board().to_string()
    }
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn searching(&self) -> bool {
        self.inner.searching()
    }
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn legal_moves(&self) -> Vec<String> {
        self.inner
            .game()
            .board()
            .legal_moves()
            .iter()
            .map(|chess_move| chess_move.to_uci(self.inner.options().chess960()))
            .collect()
    }
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn claims(&self) -> Vec<String> {
        self.inner
            .game()
            .claimable_draws()
            .iter()
            .map(|claim| {
                match claim {
                    ClaimableDraw::ThreefoldRepetition => "threefold-repetition",
                    ClaimableDraw::FiftyMoves => "fifty-moves",
                }
                .into()
            })
            .collect()
    }
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn outcome(&self) -> String {
        match self.inner.outcome() {
            Outcome::Ongoing => "ongoing".into(),
            Outcome::Checkmate { winner } => format!(
                "checkmate:{}",
                if winner == Color::White {
                    "white"
                } else {
                    "black"
                }
            ),
            Outcome::Draw(reason) => format!(
                "draw:{}",
                match reason {
                    DrawReason::Stalemate => "stalemate",
                    DrawReason::InsufficientMaterial => "insufficient-material",
                    DrawReason::FivefoldRepetition => "fivefold-repetition",
                    DrawReason::SeventyFiveMoves => "seventy-five-moves",
                }
            ),
        }
    }
}
