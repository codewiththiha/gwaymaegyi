//! Native utility/UCI entry points and re-exported portable engine APIs.

mod cli;
mod uci;

pub use cli::run;

pub use gwaymaegyi_search::{
    Engine, EngineError, Mode, Options, SearchLimits, SearchReport, SearchStatus, SkillLevel,
    Strength,
};
pub use uci::run_uci;
