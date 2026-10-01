//! Immutable quantized models and incremental evaluation without platform intrinsics.

mod accumulator;
mod model;

pub use accumulator::Accumulator;
pub use model::Model;

const HIDDEN: usize = 1024;
