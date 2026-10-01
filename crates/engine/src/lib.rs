//! Testable native commands; process-level input and output stay in the binary.

mod cli;
mod uci;

pub use cli::run;

pub use gwaymaegyi_search::{
    Engine, EngineError, Mode, Options, SearchLimits, SearchReport, SearchStatus, Strength,
};
pub use uci::run_uci;
