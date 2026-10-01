//! Configuration, resource, and continuation errors exposed by the portable engine.

use std::{error::Error, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EngineError {
    InvalidMode,
    InvalidBehavior,
    InvalidParameter,
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
            Self::InvalidBehavior => "unknown search behavior",
            Self::InvalidParameter => {
                "unknown search parameter or value outside its supported range"
            }
            Self::InvalidElo => "Elo must be zero for full strength or between 500 and 3000",
            Self::InvalidSkill => "skill level must be 1 through 21; 21 selects full strength",
            Self::NoActiveSearch => "a running search is required to update limits",
            Self::InvalidHash => {
                return write!(out, "hash size must be between 1 and {} MiB", crate::MAX_HASH_MIB);
            }
            Self::InvalidMultiPv => {
                return write!(out, "MultiPV must be between 1 and {}", crate::MAX_MULTI_PV);
            }
            Self::InvalidLimits => {
                return write!(
                    out,
                    "depth must be 1 through {} and the node limit must be positive",
                    crate::MAX_DEPTH
                );
            }
            Self::InvalidSlice => {
                return write!(out, "slice budget must be between 1 and {} work units", crate::MAX_WORK);
            }
            Self::InvalidPosition(reason) | Self::InvalidMove(reason) => reason,
            Self::Resources => "engine memory allocation failed",
            Self::InternalState => "invalid internal search transition",
        })
    }
}
impl Error for EngineError {}
