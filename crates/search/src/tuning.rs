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
}
impl Behavior {
    pub const ALL: [Self; 5] = [
        Self::Aspiration,
        Self::NullMove,
        Self::ReverseFutility,
        Self::QuietPruning,
        Self::LateReductions,
    ];
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Aspiration => "aspiration",
            Self::NullMove => "null-move",
            Self::ReverseFutility => "reverse-futility",
            Self::QuietPruning => "quiet-pruning",
            Self::LateReductions => "late-reductions",
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
pub enum Parameter {
    AspirationWindow,
    AspirationDepth,
    NullMinDepth,
    NullBase,
    NullDepthDiv,
    NullEvalDiv,
    RfpMargin,
    RfpDepth,
    FutilityBase,
    FutilityMargin,
    QuietDepth,
    LateBase,
    ReductionDepth,
    HistoryBonus,
    HistoryMax,
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
    pub const SPECS: [ParameterSpec; 15] = [
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
            parameter: Self::RfpDepth,
            name: "RFPMaxDepth",
            default: 5,
            min: 1,
            max: 12,
        },
        ParameterSpec {
            parameter: Self::FutilityBase,
            name: "FPMargin1",
            default: 100,
            min: 50,
            max: 150,
        },
        ParameterSpec {
            parameter: Self::FutilityMargin,
            name: "FPMargin2",
            default: 120,
            min: 75,
            max: 175,
        },
        ParameterSpec {
            parameter: Self::QuietDepth,
            name: "FPDepth",
            default: 3,
            min: 1,
            max: 11,
        },
        ParameterSpec {
            parameter: Self::LateBase,
            name: "LMPBase",
            default: 4,
            min: 1,
            max: 8,
        },
        ParameterSpec {
            parameter: Self::ReductionDepth,
            name: "LMRMinDepth",
            default: 3,
            min: 2,
            max: 6,
        },
        ParameterSpec {
            parameter: Self::HistoryBonus,
            name: "HistBonus",
            default: 291,
            min: 200,
            max: 400,
        },
        ParameterSpec {
            parameter: Self::HistoryMax,
            name: "HistMax",
            default: 2476,
            min: 1500,
            max: 3500,
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
    values: [i32; 15],
    enabled: [bool; 5],
}
impl Default for SearchTuning {
    fn default() -> Self {
        Self {
            values: Parameter::SPECS.map(|spec| spec.default),
            enabled: [true; 5],
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
}
