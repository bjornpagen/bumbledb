//! Precomputed crud op streams — the representation that makes twin
//! divergence unrepresentable: ONE stream per family is the shared
//! truth both engines fold over ([`super::lanes`]), so different keys,
//! different values, or different op counts between the twins simply
//! cannot be expressed. Every generator is a pure function of
//! `(seed, sizes, count)` where `count = protocol.warmups +
//! protocol.samples` — the runner's total closure invocations (batch
//! families multiply per-commit work internally, never the stream
//! length). Identical inputs ⇒ identical streams, forever; purity is
//! pinned by test.
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
/// mass, `prev` tracked through a local map initialized to the seeded 0
/// (repeat keys chain correctly), `next = prev + 1 + draw(1000)` —
/// strictly increasing per key, so a delete+insert never cancels.
///
/// # Panics
///
/// Never in practice: the value chain stays far below `i64::MAX`.
#[must_use]
pub fn update_stream(seed: u64, sizes: CrudSizes, count: usize) -> Vec<UpdateOp> {
    let mut rng = Rng::new(seed ^ UPDATE_SALT);
    let mut vals: HashMap<u64, i64> = HashMap::new();
    (0..count)
        .map(|_| {
            let key = rng.range(sizes.counters);
            let prev = vals.get(&key).copied().unwrap_or(0);
            let next = prev + 1 + i64::try_from(rng.range(1000)).expect("small");
            vals.insert(key, next);
            UpdateOp { key, prev, next }
        })
        .collect()
}

/// The `crud_update_hot` stream: key fixed at 0, value chain 0→1→2… —
/// the hot-row single-writer contention shape. A pure function of
/// `count` alone; no entropy exists to draw.
///
/// # Panics
///
/// Never in practice: protocol counts fit `i64`.
#[must_use]
pub fn hot_update_stream(count: usize) -> Vec<UpdateOp> {
    (0..count)
        .map(|i| {
            let prev = i64::try_from(i).expect("protocol counts are small");
            UpdateOp {
                key: 0,
                prev,
                next: prev + 1,
            }
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
/// `Counter` mass (≈half the draws miss), `prev` tracked through a map
/// lazily seeded with 0 for keys below `sizes.counters` (the loaded
/// corpus value) and `None` above (never loaded — a miss until this
/// stream's own upsert lands it).
///
/// # Panics
///
/// Never in practice: the value chain stays far below `i64::MAX`.
#[must_use]
pub fn upsert_stream(seed: u64, sizes: CrudSizes, count: usize) -> Vec<UpsertOp> {
    let mut rng = Rng::new(seed ^ UPSERT_SALT);
    let mut vals: HashMap<u64, Option<i64>> = HashMap::new();
    (0..count)
        .map(|_| {
            let key = rng.range(sizes.counters * 2);
            let prev = *vals
                .entry(key)
                .or_insert_with(|| (key < sizes.counters).then_some(0));
            let next = prev.unwrap_or(0) + 1 + i64::try_from(rng.range(1000)).expect("small");
            vals.insert(key, Some(next));
            UpsertOp { key, prev, next }
        })
        .collect()
}

/// The `crud_rmw` stream: keys over the standing `Counter` mass. No
/// values ride along — the read half of the round trip fetches the
/// stored value and the host computes `val + 1`, so both engines'
/// results are functions of their own committed states (which start
/// identical and therefore stay identical).
#[must_use]
pub fn rmw_stream(seed: u64, sizes: CrudSizes, count: usize) -> Vec<u64> {
    let mut rng = Rng::new(seed ^ RMW_SALT);
    (0..count).map(|_| rng.range(sizes.counters)).collect()
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
