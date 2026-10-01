//! Native utility/UCI entry points and re-exported portable engine APIs.

#[cfg(not(target_family = "wasm"))]
mod batch;
mod cli;
mod uci;

pub use cli::run;

pub use gwaymaegyi_search::{
    Engine, EngineError, Mode, Options, SearchLimits, SearchReport, SearchStatus, SkillLevel,
    Strength,
};
pub use uci::run_uci;

#[cfg(not(target_family = "wasm"))]
pub use batch::{AnalysisRequest, BatchError, analyze_batch};
