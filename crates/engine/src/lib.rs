//! Native utility/UCI entry points and re-exported portable engine APIs.

#[cfg(not(target_family = "wasm"))]
mod batch;
mod cli;
#[cfg(not(target_family = "wasm"))]
mod tablebase;
#[cfg(not(target_family = "wasm"))]
mod uci;

pub use cli::run;
#[cfg(not(target_family = "wasm"))]
pub use tablebase::NativeTablebases;

pub use gwaymaegyi_search::{
    BULLET_RECORD_BYTES, BulletRecord, DatasetError, Engine, EngineError, FilterKind, GameResult,
    Mode, Options, SearchLimits, SearchReport, SearchStatus, SkillLevel, Strength, TablebaseProbe,
    TablebaseRoot, TablebaseWdl, TrainingRecord, decode_bullet_records, encode_bullet_records,
    filter_lines,
};
#[cfg(not(target_family = "wasm"))]
pub use uci::run_uci;

#[cfg(not(target_family = "wasm"))]
pub use batch::{AnalysisRequest, BatchError, analyze_batch};
