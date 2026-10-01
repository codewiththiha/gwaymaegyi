use super::history::History;
use crate::{EngineError, MATE_THRESHOLD};
use gwaymaegyi_core::Move;

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

#[derive(Debug)]
pub(super) struct Cache {
    entries: Vec<Option<Entry>>,
    age: u8,
    pub history: History,
}
impl Cache {
    pub(super) fn new(mib: u16) -> Result<Self, EngineError> {
        let capacity = usize::from(mib) * 1024 * 1024 / size_of::<Option<Entry>>();
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
        })
    }
    pub(super) fn clear(&mut self) {
        self.entries.fill(None);
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
        self.entries[self.index(key)].filter(|entry| entry.key == key)
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
        let index = self.index(key);
        if self.entries[index].is_some_and(|entry| {
            entry.age == self.age && entry.depth > depth + 4 && bound != Bound::Exact
        }) {
            return;
        }
        let score = if score >= MATE_THRESHOLD {
            score + i32::from(ply)
        } else if score <= -MATE_THRESHOLD {
            score - i32::from(ply)
        } else {
            score
        };
        self.entries[index] = Some(Entry {
            key,
            depth,
            score,
            best,
            bound,
            age: self.age,
        });
    }
}
