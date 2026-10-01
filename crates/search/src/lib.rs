//! Deterministic search advances in bounded slices without platform clocks or threads.

mod config;
mod engine;
mod error;
mod report;

pub use config::{Mode, Options, SearchLimits, Strength};
pub use engine::Engine;
pub use error::EngineError;
pub use report::{Completion, PrincipalVariation, SearchReport, SearchStatus};

pub const MAX_DEPTH: u8 = 64;
const MAX_PLY: u8 = 96;
const MATE: i32 = 30_000;
const MATE_THRESHOLD: i32 = MATE - MAX_PLY as i32;
const INFINITY: i32 = 31_000;
