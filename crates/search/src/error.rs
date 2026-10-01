//! Configuration, resource, and continuation errors exposed by the portable engine.

use std::{error::Error, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EngineError {
    InvalidMode,
    InvalidElo,
    InvalidSkill,
    InvalidHash,
    InvalidMultiPv,
    InvalidLimits,
    InvalidSlice,
    NoActiveSearch,
    InvalidPosition(String),
    InvalidMove(String),
    Resources,
    InternalState,
}

impl fmt::Display for EngineError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(match self {
            Self::InvalidMode => "mode must be balanced, aggressive, human-like, or analysis",
            Self::InvalidElo => "Elo must be zero for full strength or between 500 and 3000",
            Self::InvalidSkill => "skill level must be 1 through 21; 21 selects full strength",
            Self::NoActiveSearch => "a running search is required to update limits",
            Self::InvalidHash => "hash size must be between 1 and 64 MiB",
            Self::InvalidMultiPv => "MultiPV must be between 1 and 5",
            Self::InvalidLimits => "depth must be 1 through 64 and the node limit must be positive",
            Self::InvalidSlice => "slice budget must be between 1 and 65536 work units",
            Self::InvalidPosition(reason) | Self::InvalidMove(reason) => reason,
            Self::Resources => "engine memory allocation failed",
            Self::InternalState => "invalid internal search transition",
        })
    }
}
impl Error for EngineError {}
