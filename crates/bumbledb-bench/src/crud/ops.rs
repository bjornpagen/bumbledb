//! Every generator is a pure function of families multiply per-commit work
//! internally, never the stream length) — and `model` is the lane's ONE
//! evolving [`CounterModel`], `(seed, sizes, count, model)` where `count =
//! protocol.warmups + protocol.samples` — the runner's total closure
//! invocations (batch

use std::collections::HashMap;

use bumbledb::Value;

use crate::corpus_gen::Rng;

use super::{CrudSizes, corpus, ids};

pub const UPDATE_SALT: u64 = 0xC24D_0001;

pub const UPSERT_SALT: u64 = 0xC24D_0002;

pub const RMW_SALT: u64 = 0xC24D_0003;

pub const READ_SALT: u64 = 0xC24D_0004;

pub const INSERT_SALT: u64 = 0xC24D_0005;

/// loaded corpus, and the no-collision condition between the seeded streams
/// lived in prose and hand-picked seeds — seed 1 under the Before the model,
/// each generator privately assumed the pristine
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterModel {

    counters: u64,

    vals: HashMap<u64, i64>,
}

impl CounterModel {

    #[must_use]
    pub fn at_load(sizes: CrudSizes) -> Self {
        Self {
            counters: sizes.counters,
            vals: HashMap::new(),
        }
    }

    #[must_use]
    pub fn get(&self, key: u64) -> Option<i64> {
        self.vals
            .get(&key)
            .copied()
            .or_else(|| (key < self.counters).then_some(0))
    }

    fn set(&mut self, key: u64, val: i64) {
        self.vals.insert(key, val);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateOp {
    pub key: u64,
    pub prev: i64,
    pub next: i64,
}

/// # Panics
#[must_use]
pub fn update_stream(
    seed: u64,
    sizes: CrudSizes,
    count: usize,
    model: &mut CounterModel,
) -> Vec<UpdateOp> {
    let mut rng = Rng::new(seed ^ UPDATE_SALT);
    (0..count)
        .map(|_| {
            let key = rng.range(sizes.counters);
            let prev = model
                .get(key)
                .expect("update keys draw over the loaded mass");
            let next = prev + 1 + i64::try_from(rng.range(1000)).expect("small");
            model.set(key, next);
            UpdateOp { key, prev, next }
        })
        .collect()
}

/// # Panics
/// Never in practice: key 0 is loaded, and protocol counts fit `i64`.
#[must_use]
pub fn hot_update_stream(count: usize, model: &mut CounterModel) -> Vec<UpdateOp> {
    (0..count)
        .map(|_| {
            let prev = model.get(0).expect("key 0 is loaded");
            let next = prev + 1;
            model.set(0, next);
            UpdateOp { key: 0, prev, next }
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpsertOp {
    pub key: u64,
    pub prev: Option<i64>,
    pub next: i64,
}

/// # Panics
#[must_use]
pub fn upsert_stream(
    seed: u64,
    sizes: CrudSizes,
    count: usize,
    model: &mut CounterModel,
) -> Vec<UpsertOp> {
    let mut rng = Rng::new(seed ^ UPSERT_SALT);
    (0..count)
        .map(|_| {
            let key = rng.range(sizes.counters * 2);
            let prev = model.get(key);
            let next = prev.unwrap_or(0) + 1 + i64::try_from(rng.range(1000)).expect("small");
            model.set(key, next);
            UpsertOp { key, prev, next }
        })
        .collect()
}

/// to the lane model, so any family after rmw models the store rmw
/// # Panics
#[must_use]
pub fn rmw_stream(seed: u64, sizes: CrudSizes, count: usize, model: &mut CounterModel) -> Vec<u64> {
    let mut rng = Rng::new(seed ^ RMW_SALT);
    (0..count)
        .map(|_| {
            let key = rng.range(sizes.counters);
            let prev = model.get(key).expect("rmw keys draw over the loaded mass");
            model.set(key, prev + 1);
            key
        })
        .collect()
}

#[must_use]
pub fn read_keys(seed: u64, sizes: CrudSizes) -> Vec<Vec<Value>> {
    let mut rng = Rng::new(seed ^ READ_SALT);
    vec![
        vec![Value::U64(rng.range(sizes.docs))],
        vec![Value::U64(rng.range(sizes.docs))],
        vec![Value::U64(rng.range(sizes.docs))],
        vec![Value::U64(u64::MAX / 2)],
    ]
}

/// # Panics
/// When `count` exceeds the delete pool — the pool-size ≥ warmups+samples
/// invariant, violated only by a misregistered protocol (a programmer error,
/// loud at derivation).
#[must_use]
pub fn delete_rows(seed: u64, sizes: CrudSizes, count: usize) -> Vec<Vec<Value>> {
    assert!(
        u64::try_from(count).expect("protocol counts are small") <= sizes.delete_pool,
        "the delete pool ({}) must cover every invocation ({count})",
        sizes.delete_pool
    );
    corpus::relation_rows(sizes, seed, ids::DOC)
        .skip(usize::try_from(sizes.docs).expect("fits"))
        .take(count)
        .collect()
}

#[must_use]
pub fn fresh_payload(seed: u64, cursor: u64) -> [u8; 32] {
    let mut rng = Rng::new(seed ^ INSERT_SALT ^ cursor);
    let mut payload = [0u8; 32];
    for chunk in payload.as_chunks_mut::<8>().0 {
        *chunk = rng.u64().to_le_bytes();
    }
    payload
}
