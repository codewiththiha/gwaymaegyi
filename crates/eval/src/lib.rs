//! Immutable quantized weights and worker-owned incremental evaluation.

mod accumulator;
mod model;

pub use accumulator::Accumulator;
pub use model::Model;

const HIDDEN: usize = 1024;
