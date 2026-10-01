//! Per-engine search behavior and validated parameters, with no mutable registry.

#![expect(
    clippy::missing_errors_doc,
    reason = "Validation failures use plain prose."
)]

use crate::EngineError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Behavior {
    Aspiration,
    NullMove,
    ReverseFutility,
    QuietPruning,
    LateReductions,
    Razoring,
    InternalReductions,
    ExchangePruning,
    HistoryPruning,
    Probcut,
    SingularExtensions,
}
impl Behavior {
    pub const ALL: [Self; 11] = [
        Self::Aspiration,
        Self::NullMove,
        Self::ReverseFutility,
        Self::QuietPruning,
        Self::LateReductions,
        Self::Razoring,
        Self::InternalReductions,
        Self::ExchangePruning,
        Self::HistoryPruning,
        Self::Probcut,
        Self::SingularExtensions,
    ];
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Aspiration => "aspiration",
            Self::NullMove => "null-move",
            Self::ReverseFutility => "reverse-futility",
            Self::QuietPruning => "quiet-pruning",
            Self::LateReductions => "late-reductions",
            Self::Razoring => "razoring",
            Self::InternalReductions => "internal-reductions",
            Self::ExchangePruning => "exchange-pruning",
            Self::HistoryPruning => "history-pruning",
            Self::Probcut => "probcut",
            Self::SingularExtensions => "singular-extensions",
        }
    }
    pub fn parse(name: &str) -> Result<Self, EngineError> {
        Self::ALL
            .into_iter()
            .find(|value| value.name() == name)
            .ok_or(EngineError::InvalidBehavior)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(usize)]
pub enum Parameter {
    AspirationWindow,
    AspirationDepth,
    RazoringMargin,
    RazoringDepth,
    IirDepth,
    SingularDepth,
    SingularDoubleMargin,
    SingularTripleMargin,
    ProbcutMargin,
    ProbcutDepth,
    SeePruneQuiet,
    SeePruneNoisy,
    SeePruneDepth,
    HistPruneDepth,
    HistDiv,
    HistBonus,
    HistMax,
    LmpBase,
    LmpDepth,
    FutilityMargin1,
    FutilityMargin2,
    FutilityDepth,
    LmrBase,
    LmrRatio,
    LmrMinDepth,
    NullMinDepth,
    NullBase,
    NullDepthDiv,
    NullEvalDiv,
    RfpMargin,
    RfpMaxDepth,
    CorrWeight,
    NodeTmFactor1,
    NodeTmFactor2,
    BmFactor1,
    ScoreDropDiv,
    ScoreDropMin,
    ScoreDropMax,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParameterSpec {
    pub parameter: Parameter,
    pub name: &'static str,
    pub default: i32,
    pub min: i32,
    pub max: i32,
}

impl Parameter {
    pub const SPECS: [ParameterSpec; 38] = [
        ParameterSpec {
            parameter: Self::AspirationWindow,
            name: "AspStartWindow",
            default: 20,
            min: 10,
            max: 100,
        },
        ParameterSpec {
            parameter: Self::AspirationDepth,
            name: "AspStartDepth",
            default: 4,
            min: 2,
            max: 16,
        },
        ParameterSpec {
            parameter: Self::RazoringMargin,
            name: "RazoringMargin",
            default: 400,
            min: 200,
            max: 600,
        },
        ParameterSpec {
            parameter: Self::RazoringDepth,
            name: "RazoringDepth",
            default: 5,
            min: 3,
            max: 8,
        },
        ParameterSpec {
            parameter: Self::IirDepth,
            name: "IIRMinDepth",
            default: 2,
            min: 1,
            max: 5,
        },
        ParameterSpec {
            parameter: Self::SingularDepth,
            name: "SEDepth",
            default: 5,
            min: 4,
            max: 10,
        },
        ParameterSpec {
            parameter: Self::SingularDoubleMargin,
            name: "SEDoubleExtMargin",
            default: 18,
            min: 10,
            max: 30,
        },
        ParameterSpec {
            parameter: Self::SingularTripleMargin,
            name: "SETripleExtMargin",
            default: 126,
            min: 75,
            max: 175,
        },
        ParameterSpec {
            parameter: Self::ProbcutMargin,
            name: "ProbcutMargin",
            default: 250,
            min: 150,
            max: 400,
        },
        ParameterSpec {
            parameter: Self::ProbcutDepth,
            name: "ProbcutDepth",
            default: 4,
            min: 2,
            max: 8,
        },
        ParameterSpec {
            parameter: Self::SeePruneQuiet,
            name: "SeePruningQuietMargin",
            default: -94,
            min: -150,
            max: -50,
        },
        ParameterSpec {
            parameter: Self::SeePruneNoisy,
            name: "SeePruningNoisyMargin",
            default: -86,
            min: -110,
            max: -50,
        },
        ParameterSpec {
            parameter: Self::SeePruneDepth,
            name: "SeePruningDepth",
            default: 7,
            min: 5,
            max: 11,
        },
        ParameterSpec {
            parameter: Self::HistPruneDepth,
            name: "HistPruningDepth",
            default: 4,
            min: 2,
            max: 7,
        },
        ParameterSpec {
            parameter: Self::HistDiv,
            name: "HistDiv",
            default: 9818,
            min: 5000,
            max: 15000,
        },
        ParameterSpec {
            parameter: Self::HistBonus,
            name: "HistBonus",
            default: 291,
            min: 200,
            max: 400,
        },
        ParameterSpec {
            parameter: Self::HistMax,
            name: "HistMax",
            default: 2476,
            min: 1500,
            max: 3500,
        },
        ParameterSpec {
            parameter: Self::LmpBase,
            name: "LMPBase",
            default: 2,
            min: 1,
            max: 5,
        },
        ParameterSpec {
            parameter: Self::LmpDepth,
            name: "LMPDepth",
            default: 6,
            min: 3,
            max: 7,
        },
        ParameterSpec {
            parameter: Self::FutilityMargin1,
            name: "FPMargin1",
            default: 98,
            min: 50,
            max: 150,
        },
        ParameterSpec {
            parameter: Self::FutilityMargin2,
            name: "FPMargin2",
            default: 121,
            min: 75,
            max: 175,
        },
        ParameterSpec {
            parameter: Self::FutilityDepth,
            name: "FPDepth",
            default: 9,
            min: 5,
            max: 11,
        },
        ParameterSpec {
            parameter: Self::LmrBase,
            name: "LMRBase",
            default: 4,
            min: 2,
            max: 8,
        },
        ParameterSpec {
            parameter: Self::LmrRatio,
            name: "LMRRatio",
            default: 20,
            min: 15,
            max: 30,
        },
        ParameterSpec {
            parameter: Self::LmrMinDepth,
            name: "LMRMinDepth",
            default: 3,
            min: 2,
            max: 6,
        },
        ParameterSpec {
            parameter: Self::NullMinDepth,
            name: "NMPMinDepth",
            default: 3,
            min: 1,
            max: 5,
        },
        ParameterSpec {
            parameter: Self::NullBase,
            name: "NMPBase",
            default: 4,
            min: 1,
            max: 5,
        },
        ParameterSpec {
            parameter: Self::NullDepthDiv,
            name: "NMPDepthDiv",
            default: 5,
            min: 3,
            max: 9,
        },
        ParameterSpec {
            parameter: Self::NullEvalDiv,
            name: "NMPEvalDiv",
            default: 175,
            min: 100,
            max: 300,
        },
        ParameterSpec {
            parameter: Self::RfpMargin,
            name: "RFPMargin",
            default: 85,
            min: 50,
            max: 150,
        },
        ParameterSpec {
            parameter: Self::RfpMaxDepth,
            name: "RFPMaxDepth",
            default: 10,
            min: 6,
            max: 12,
        },
        ParameterSpec {
            parameter: Self::CorrWeight,
            name: "CorrWeight",
            default: 25,
            min: 10,
            max: 40,
        },
        ParameterSpec {
            parameter: Self::NodeTmFactor1,
            name: "NodeTmFactor1",
            default: 149,
            min: 100,
            max: 200,
        },
        ParameterSpec {
            parameter: Self::NodeTmFactor2,
            name: "NodeTmFactor2",
            default: 177,
            min: 125,
            max: 225,
        },
        ParameterSpec {
            parameter: Self::BmFactor1,
            name: "BmFactor1",
            default: 152,
            min: 100,
            max: 200,
        },
        ParameterSpec {
            parameter: Self::ScoreDropDiv,
            name: "ScoreDropDiv",
            default: 540,
            min: 250,
            max: 850,
        },
        ParameterSpec {
            parameter: Self::ScoreDropMin,
            name: "ScoreDropMin",
            default: 90,
            min: 70,
            max: 100,
        },
        ParameterSpec {
            parameter: Self::ScoreDropMax,
            name: "ScoreDropMax",
            default: 118,
            min: 100,
            max: 140,
        },
    ];
    #[must_use]
    pub const fn spec(self) -> ParameterSpec {
        Self::SPECS[self as usize]
    }
    pub fn parse(name: &str) -> Result<Self, EngineError> {
        Self::SPECS
            .iter()
            .find(|spec| spec.name == name)
            .map(|spec| spec.parameter)
            .ok_or(EngineError::InvalidParameter)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SearchTuning {
    values: [i32; 38],
    enabled: [bool; 11],
}
impl Default for SearchTuning {
    fn default() -> Self {
        Self {
            values: Parameter::SPECS.map(|spec| spec.default),
            enabled: [true; 11],
        }
    }
}
impl SearchTuning {
    #[must_use]
    pub const fn get(self, parameter: Parameter) -> i32 {
        self.values[parameter as usize]
    }
    #[must_use]
    pub const fn enabled(self, behavior: Behavior) -> bool {
        self.enabled[behavior as usize]
    }
    pub const fn set_behavior(&mut self, behavior: Behavior, enabled: bool) {
        self.enabled[behavior as usize] = enabled;
    }
    pub fn set(&mut self, parameter: Parameter, value: i32) -> Result<(), EngineError> {
        let spec = parameter.spec();
        if !(spec.min..=spec.max).contains(&value) {
            return Err(EngineError::InvalidParameter);
        }
        self.values[parameter as usize] = value;
        Ok(())
    }

    /// Return the adaptive soft limit, bounded by the caller's hard time budget.
    ///
    /// # Precision
    /// Timing arithmetic uses `f64`; millisecond budgets are capped by native/WASM
    /// callers at one day, while node ratios only guide a bounded estimate.
    #[expect(
        clippy::cast_precision_loss,
        reason = "Times are capped at one day and node precision only guides a bounded ratio."
    )]
    #[expect(
        clippy::cast_sign_loss,
        reason = "The computed soft limit is clamped to a positive value before conversion."
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "The rounded positive limit is clamped before conversion; extreme casts saturate."
    )]
    #[must_use]
    pub fn soft_limit_ms(
        self,
        original_opt_ms: u64,
        max_ms: u64,
        best_move_nodes: u64,
        nodes: u64,
        stability: u32,
        score_delta: i32,
    ) -> Option<u64> {
        if original_opt_ms == 0 || max_ms == 0 {
            return None;
        }
        let node_share = best_move_nodes as f64 / nodes.max(1) as f64;
        let node_factor = (f64::from(self.get(Parameter::NodeTmFactor1)) / 100.0 - node_share)
            * f64::from(self.get(Parameter::NodeTmFactor2))
            / 100.0;
        let stability_factor =
            f64::from(stability).mul_add(-0.06, f64::from(self.get(Parameter::BmFactor1)) / 100.0);
        let score_factor =
            (1.0 + f64::from(score_delta) / f64::from(self.get(Parameter::ScoreDropDiv))).clamp(
                f64::from(self.get(Parameter::ScoreDropMin)) / 100.0,
                f64::from(self.get(Parameter::ScoreDropMax)) / 100.0,
            );
        let soft = (original_opt_ms as f64 * node_factor)
            .mul_add(stability_factor, 0.0)
            .mul_add(score_factor, 0.0)
            .clamp(1.0, max_ms as f64)
            .round() as u64;
        Some(soft)
    }
}
