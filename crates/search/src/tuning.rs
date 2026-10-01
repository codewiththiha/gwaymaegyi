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
    pub const SPECS: [ParameterSpec; 31] = [
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
    values: [i32; 31],
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
