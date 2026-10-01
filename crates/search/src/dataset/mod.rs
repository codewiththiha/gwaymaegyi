//! Portable training records, packed binary conversion, and position filters.

mod compensation;
mod filter;
mod record;
mod tactical;

pub use filter::{FilterKind, filter_lines};
pub use record::{
    BULLET_RECORD_BYTES, BulletRecord, DatasetError, GameResult, TrainingRecord,
    decode_bullet_records, encode_bullet_records,
};
