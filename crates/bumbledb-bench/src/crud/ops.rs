//! Precomputed crud op streams — the representation that makes twin
//! divergence unrepresentable: ONE stream per family is the shared
//! truth both engines fold over ([`super::lanes`]), so different keys,
//! different values, or different op counts between the twins simply
//! cannot be expressed. Every generator is a pure function of
//! `(seed, sizes, count, model)` where `count = protocol.warmups +
//! protocol.samples` — the runner's total closure invocations (batch
//! families multiply per-commit work internally, never the stream
//! length) — and `model` is the lane's ONE evolving [`CounterModel`],
//! threaded through the generators in the exact run order, so a later
//! family's `prev` accounting always describes the store the earlier
//! families actually left. Identical inputs ⇒ identical streams,
//! forever; purity is pinned by test.
//!
//! Entropy: [`Rng`] seeded `seed ^ <per-family salt>` — one documented
//! salt const per family, so no two families ever share a draw
//! sequence (the writebench `0x0115_xxxx` convention, crud-prefixed).

use std::collections::HashMap;

use bumbledb::Value;

use crate::corpus_gen::Rng;

use super::{CrudSizes, corpus, ids};

/// The `crud_update` stream's salt.
pub const UPDATE_SALT: u64 = 0xC24D_0001;
/// The `crud_upsert` stream's salt.
pub const UPSERT_SALT: u64 = 0xC24D_0002;
/// The `crud_rmw` stream's salt.
pub const RMW_SALT: u64 = 0xC24D_0003;
/// The `crud_read_point`/`crud_mixed_90_10` read-rotation salt.
pub const READ_SALT: u64 = 0xC24D_0004;
/// The insert lanes' payload salt (mixed with the mint cursor).
pub const INSERT_SALT: u64 = 0xC24D_0005;

/// The lane's single evolving account of the `Counter` relation — the
/// representation that makes stream drift unrepresentable. Every
/// counter-stream generator draws its `prev` from this model and writes
/// its `next` back, and the lane fold ([`super::run`]) threads ONE model
/// through the generators in the exact run order, so a family's stream
/// always describes the store the earlier families actually left.
///
/// Before the model, each generator privately assumed the pristine
/// loaded corpus, and the no-collision condition between the seeded
/// streams lived in prose and hand-picked seeds — seed 1 under the
/// registry protocols collided (`crud_update` touched keys the
/// `crud_upsert` stream still modeled as the loaded 0), and the upsert
/// lane's drift check aborted the run, exactly as its contract promises.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterModel {
    /// The loaded mass: keys `0..counters` hold the seeded 0 at load.
    counters: u64,
    /// Every value a stream has written over the load, keyed.
    vals: HashMap<u64, i64>,
}

impl CounterModel {
    /// The model of the freshly loaded corpus: keys `0..sizes.counters`
    /// hold 0 ([`corpus::relation_rows`]), every other key misses.
    #[must_use]
    pub fn at_load(sizes: CrudSizes) -> Self {
        Self {
            counters: sizes.counters,
            vals: HashMap::new(),
        }
    }

    /// The modeled stored value at `key` (`None` = the key misses).
    #[must_use]
    pub fn get(&self, key: u64) -> Option<i64> {
        self.vals
            .get(&key)
            .copied()
            .or_else(|| (key < self.counters).then_some(0))
    }

    /// One modeled write: `Counter{key}` now holds `val`.
    fn set(&mut self, key: u64, val: i64) {
        self.vals.insert(key, val);
    }
}

/// One update op: replace `Counter{key, prev}` with `Counter{key, next}`.
/// `prev` is part of the stream so the engine's delete half is
/// delete-bearing BY CONTRACT — a stale `prev` aborts instead of
/// silently measuring an insert-only fork.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateOp {
    pub key: u64,
    pub prev: i64,
    pub next: i64,
}

/// The `crud_update` stream: keys drawn over the standing `Counter`
/// mass, `prev` drawn from the lane model (repeat keys and earlier
/// families chain correctly), `next = prev + 1 + draw(1000)` —
/// strictly increasing per key, so a delete+insert never cancels.
///
/// # Panics
///
/// Never in practice: the keys draw over the loaded mass (always
/// present in the model) and the value chain stays far below
/// `i64::MAX`.
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

/// The `crud_update_hot` stream: key fixed at 0, the value chain
/// stepping +1 from wherever the lane model holds key 0 — the hot-row
/// single-writer contention shape. No entropy exists to draw.
///
/// # Panics
///
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

/// One upsert op: set `Counter{key}` to `next`; `prev` is the stream's
/// account of what the store holds (`None` = the key misses). The
/// engine runner checks the store against `prev` inside the write
/// closure — stream drift aborts the transaction whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpsertOp {
    pub key: u64,
    pub prev: Option<i64>,
    pub next: i64,
}

/// The `crud_upsert` stream: keys drawn over TWICE the standing
/// `Counter` mass (≈half the draws miss), `prev` drawn from the lane
/// model — `Some` for keys the load or an earlier family landed,
/// `None` above the loaded mass until this stream's own upsert lands
/// them.
///
/// # Panics
///
/// Never in practice: the value chain stays far below `i64::MAX`.
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

/// The `crud_rmw` stream: keys over the standing `Counter` mass. No
/// values ride along — the read half of the round trip fetches the
/// stored value and the host computes `val + 1`, so both engines'
/// results are functions of their own committed states (which start
/// identical and therefore stay identical). The `+1` is still applied
/// to the lane model, so any family after rmw models the store rmw
/// actually left.
///
/// # Panics
///
/// Never in practice: the keys draw over the loaded mass.
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

/// The point-read rotation: EXACTLY 4 param sets (the rotation and the
/// oracle-gate sets) — three hits drawn over the standing `Doc` mass
/// and one miss at `u64::MAX / 2`, a key no insert lane can ever mint
/// (inserts mint upward from `docs + delete_pool`).
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

/// The delete lane's rows: pool rows re-derived whole through the ONE
/// corpus row function ([`corpus::relation_rows`]) at ids
/// `docs..docs + count`, in order — the engine hands the full fact to
/// `delete_dyn`, byte-identical to what the loader inserted.
///
/// # Panics
///
/// When `count` exceeds the delete pool — the pool-size ≥
/// warmups+samples invariant, violated only by a misregistered
/// protocol (a programmer error, loud at derivation).
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

/// One freshly minted `Doc` payload — a pure function of
/// `(seed, cursor)`, so the two engines' insert lanes mint
/// byte-identical rows from their own cursors.
#[must_use]
pub fn fresh_payload(seed: u64, cursor: u64) -> [u8; 32] {
    let mut rng = Rng::new(seed ^ INSERT_SALT ^ cursor);
    let mut payload = [0u8; 32];
    for chunk in payload.as_chunks_mut::<8>().0 {
        *chunk = rng.u64().to_le_bytes();
    }
    payload
}
