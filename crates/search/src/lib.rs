//! Deterministic cooperative search and playing controls for native/WASM hosts.

mod config;
mod engine;
mod error;
mod report;
mod strength;
mod tablebase;
mod tuning;

pub use config::{Mode, Options, SearchLimits};
pub use engine::Engine;
pub use error::EngineError;
pub use report::{Completion, PrincipalVariation, SearchReport, SearchStatus};
pub use strength::{SkillLevel, Strength};
pub use tablebase::{TablebaseProbe, TablebaseRoot, TablebaseWdl};

pub const MAX_DEPTH: u8 = 64;
const MAX_PLY: u8 = 96;
const MATE: i32 = 30_000;
const MATE_THRESHOLD: i32 = MATE - MAX_PLY as i32;
const INFINITY: i32 = 31_000;

pub use tuning::{Behavior, Parameter, ParameterSpec, SearchTuning};
