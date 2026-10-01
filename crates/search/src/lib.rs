//! Deterministic cooperative search and playing controls for native/WASM hosts.

mod bench;
mod config;
mod dataset;
mod engine;
mod error;
mod report;
mod strength;
mod tablebase;
mod tuning;

pub use bench::{BENCH_POSITIONS, BenchmarkEntry, BenchmarkReport, run_benchmark};
pub use config::{Mode, Options, SearchLimits};
pub use dataset::{
    BULLET_RECORD_BYTES, BulletRecord, DatasetError, FilterKind, GameResult, TrainingRecord,
    decode_bullet_records, encode_bullet_records, filter_lines,
};
pub use engine::{Engine, SharedTable};
pub use error::EngineError;
pub use report::{Completion, PrincipalVariation, SearchReport, SearchStatus};
pub use strength::{SkillLevel, Strength};
pub use tablebase::{TablebaseProbe, TablebaseRoot, TablebaseWdl};

#[cfg(not(target_family = "wasm"))]
pub const MAX_DEPTH: u8 = 127;
#[cfg(target_family = "wasm")]
pub const MAX_DEPTH: u8 = 64;

#[cfg(not(target_family = "wasm"))]
pub const MAX_HASH_MIB: u32 = 131_072;
#[cfg(target_family = "wasm")]
pub const MAX_HASH_MIB: u32 = 64;

#[cfg(not(target_family = "wasm"))]
pub const MAX_MULTI_PV: u8 = 255;
#[cfg(target_family = "wasm")]
pub const MAX_MULTI_PV: u8 = 32;

#[cfg(not(target_family = "wasm"))]
pub const MAX_WORK: u32 = u32::MAX;
#[cfg(target_family = "wasm")]
pub const MAX_WORK: u32 = 65_536;

#[cfg(not(target_family = "wasm"))]
const MAX_PLY: u8 = 128;
#[cfg(target_family = "wasm")]
const MAX_PLY: u8 = 96;
const MATE: i32 = 30_000;
const MATE_THRESHOLD: i32 = MATE - MAX_PLY as i32;
const INFINITY: i32 = 31_000;

pub use tuning::{Behavior, Parameter, ParameterSpec, SearchTuning};
