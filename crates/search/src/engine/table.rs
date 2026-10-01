//! Bounded score cache and lock-striped shared transposition table for SMP search.

#![expect(
    clippy::missing_errors_doc,
    reason = "Allocation and hash-size errors are described in plain prose."
)]

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU8, Ordering},
};

use gwaymaegyi_core::Move;

use super::history::History;
use crate::{EngineError, MATE_THRESHOLD};

const SHARDS: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Entry {
    pub key: u64,
    pub depth: i16,
    pub score: i32,
    pub best: Option<Move>,
    pub bound: Bound,
    age: u8,
}

impl Entry {
    pub(super) fn score_at(self, ply: u8) -> i32 {
        if self.score >= MATE_THRESHOLD {
            self.score - i32::from(ply)
        } else if self.score <= -MATE_THRESHOLD {
            self.score + i32::from(ply)
        } else {
            self.score
        }
    }
}

const fn normalize_score(score: i32, ply: u8) -> i32 {
    if score >= MATE_THRESHOLD {
        score + (ply as i32)
    } else if score <= -MATE_THRESHOLD {
        score - (ply as i32)
    } else {
        score
    }
}

/// Thread-safe, lock-striped transposition table shared across parallel search workers.
#[derive(Debug)]
pub struct SharedTable {
    shards: Vec<Mutex<Vec<Option<Entry>>>>,
    slot_mask: usize,
    age: AtomicU8,
}

fn entry_capacity(mib: u32) -> Result<usize, EngineError> {
    if !(1..=crate::MAX_HASH_MIB).contains(&mib) {
        return Err(EngineError::InvalidHash);
    }
    let bytes = usize::try_from(mib)
        .map_err(|_| EngineError::Resources)?
        .checked_mul(1024 * 1024)
        .ok_or(EngineError::Resources)?;
    Ok(bytes / size_of::<Option<Entry>>())
}

impl SharedTable {
    /// Allocates a shared transposition table of `mib` MiB.
    pub fn new(mib: u32) -> Result<Self, EngineError> {
        let capacity = entry_capacity(mib)?;
        let total = (1_usize << capacity.ilog2()).max(SHARDS);
        let per_shard = (total / SHARDS).max(1);
        let mut shards = Vec::with_capacity(SHARDS);
        for _ in 0..SHARDS {
            let mut entries = Vec::new();
            entries
                .try_reserve_exact(per_shard)
                .map_err(|_| EngineError::Resources)?;
            entries.resize(per_shard, None);
            shards.push(Mutex::new(entries));
        }
        Ok(Self {
            shards,
            slot_mask: per_shard - 1,
            age: AtomicU8::new(0),
        })
    }

    /// Clears all shared transposition entries and resets the generation counter.
    pub fn clear(&self) {
        for shard in &self.shards {
            if let Ok(mut entries) = shard.lock() {
                entries.fill(None);
            }
        }
        self.age.store(0, Ordering::Relaxed);
    }

    /// Advances the table generation counter for a new search.
    pub fn next_search(&self) {
        self.age.fetch_add(1, Ordering::Relaxed);
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "Shard and slot masks intentionally select low bits of the 64-bit Zobrist key."
    )]
    const fn indices(&self, key: u64) -> (usize, usize) {
        let shard = ((key >> 32) as usize) & (SHARDS - 1);
        let slot = (key as usize) & self.slot_mask;
        (shard, slot)
    }

    pub(super) fn probe(&self, key: u64) -> Option<Entry> {
        let (shard_idx, slot_idx) = self.indices(key);
        let shard = self.shards.get(shard_idx)?;
        let entry = shard.lock().ok()?.get(slot_idx).copied().flatten()?;
        (entry.key == key).then_some(entry)
    }

    pub(super) fn store(
        &self,
        key: u64,
        depth: i16,
        ply: u8,
        score: i32,
        best: Option<Move>,
        bound: Bound,
    ) {
        let (shard_idx, slot_idx) = self.indices(key);
        let Some(shard) = self.shards.get(shard_idx) else {
            return;
        };
        let Ok(mut entries) = shard.lock() else {
            return;
        };
        let age = self.age.load(Ordering::Relaxed);
        let current = entries.get(slot_idx).copied().flatten();
        if current.is_some_and(|entry| {
            entry.age == age && entry.depth > depth + 4 && bound != Bound::Exact
        }) {
            return;
        }
        if let Some(slot) = entries.get_mut(slot_idx) {
            *slot = Some(Entry {
                key,
                depth,
                score: normalize_score(score, ply),
                best,
                bound,
                age,
            });
        }
    }
}

#[derive(Debug)]
pub(super) struct Cache {
    entries: Vec<Option<Entry>>,
    age: u8,
    pub history: History,
    shared: Option<Arc<SharedTable>>,
}

impl Cache {
    pub(super) fn new(mib: u32) -> Result<Self, EngineError> {
        let capacity = entry_capacity(mib)?;
        let length = 1_usize << capacity.ilog2();
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(length)
            .map_err(|_| EngineError::Resources)?;
        entries.resize(length, None);
        Ok(Self {
            entries,
            age: 0,
            history: History::default(),
            shared: None,
        })
    }

    pub(super) fn set_shared(&mut self, shared: Option<Arc<SharedTable>>) {
        self.shared = shared;
    }

    pub(super) fn clear_entries(&mut self) {
        self.entries.fill(None);
    }

    pub(super) fn clear(&mut self) {
        self.clear_entries();
        self.history = History::default();
    }

    pub(super) const fn next_search(&mut self) {
        self.age = self.age.wrapping_add(1);
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "A power-of-two table intentionally indexes only the low hash bits."
    )]
    fn index(&self, key: u64) -> usize {
        (key as usize) & (self.entries.len() - 1)
    }

    pub(super) fn probe(&self, key: u64) -> Option<Entry> {
        if let Some(entry) = self.entries[self.index(key)].filter(|entry| entry.key == key) {
            return Some(entry);
        }
        self.shared.as_ref().and_then(|shared| shared.probe(key))
    }

    pub(super) fn store(
        &mut self,
        key: u64,
        depth: i16,
        ply: u8,
        score: i32,
        best: Option<Move>,
        bound: Bound,
    ) {
        if let Some(shared) = &self.shared {
            shared.store(key, depth, ply, score, best, bound);
        }
        let index = self.index(key);
        if self.entries[index].is_some_and(|entry| {
            entry.age == self.age && entry.depth > depth + 4 && bound != Bound::Exact
        }) {
            return;
        }
        self.entries[index] = Some(Entry {
            key,
            depth,
            score: normalize_score(score, ply),
            best,
            bound,
            age: self.age,
        });
    }
}
