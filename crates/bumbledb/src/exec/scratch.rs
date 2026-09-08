//! Transient exact maps shared by staging and query operators.
//!
//! Two tiers, one logical map of exact byte keys:
//!
//! - **RAM**: an ordinary ordered table with no allocation quota.
//! - **Temporary LMDB**: an explicitly requested, execution-owned scratch
//!   environment. Existing entries copy over in bounded batches; no byte
//!   threshold or allocation refusal switches the representation.
//!   Scratch writes never claim authoritative
//!   durability: the environment is `NO_SYNC`, unreachable from any
//!   persistent-store constructor, and its loss loses only this query
//!   attempt. The authoritative store always uses durable commits.
//!
//! Keys are **exact full bytes** — the map is ordered by the key bytes and
//! never consults a hash verdict, so forced fingerprint collisions cannot
//! merge distinct tuples. Long logical keys are the caller's
//! encoded words/bytes; the scratch env bounds physical LMDB keys by
//! hashing oversized keys into exact-checked candidate buckets, comparing
//! full key bytes within a bucket.
//!
//! Disposal closes the native environment before unlinking its directory;
//! a failed execution drops the whole relation and cleans only its own
//! validated scratch path.
//!
//! Named maps ([`ScratchMapId`]) share one environment: Pack claims, the
//! group→token table, the token→group table, the insertion-order log, and
//! text forward/reverse are roster slots, not extra `ScratchRelation` owners.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::error::{Error, Result};
use crate::work::{WorkContext, WorkError};

pub mod append;
pub mod keys;
pub use append::ScratchAppend;
pub use keys::{
    ScratchClaimKey, ScratchExactKey, ScratchMapId, ScratchProbe, ScratchVisit, ScratchVisitor,
    ScratchWideClaimKey, ScratchWordKey,
};

/// Copy batch size for the RAM→LMDB transition (entries per transaction).
const SPILL_BATCH: u16 = 1024;

/// The scratch env's initial virtual map; grows geometrically on demand.
const INITIAL_MAP: usize = 64 << 20;

/// Physical LMDB keys are bounded: longer logical keys move into an
/// exact-checked bucket — `[0xFF, blake3_16(key), seq]` physical key with
/// the full logical key stored inside the value. 400 leaves headroom
/// below LMDB's ~511-byte bound for the namespace byte. `pub(crate)`:
/// callers whose correctness rides on the map's exact key ORDER (the
/// aggregate sink's streaming Pack union) must keep their keys inline —
/// bucketed keys iterate in fingerprint order, set semantics only.
pub(crate) const MAX_INLINE_KEY: usize = 400;

const _: () = assert!(
    ScratchClaimKey::BYTE_LEN <= MAX_INLINE_KEY,
    "Pack claims must stay inline so visit order is exact"
);

/// One staged write applied in [`ScratchWriteBatch::commit`].
struct StagedScratchPut {
    map: ScratchMapId,
    key: Box<[u8]>,
    value: Box<[u8]>,
    if_absent: bool,
}

/// Transaction-scoped named-map puts. Drop or [`Self::abort`] leaves the
/// relation untouched. The RAM tier applies the batch after its cancellation
/// check; the LMDB tier commits all writes and bucket identities together.
pub struct ScratchWriteBatch {
    staged: Vec<StagedScratchPut>,
}

impl ScratchWriteBatch {
    #[must_use]
    pub const fn new() -> Self {
        Self { staged: Vec::new() }
    }

    /// Stage a named-map put. Applied in [`Self::commit`] with the same
    /// LMDB transaction as other writes in this batch.
    /// # Errors
    /// Refuses an unallocatable batch capacity.
    pub fn put(&mut self, map: ScratchMapId, key: &[u8], value: &[u8]) -> Result<()> {
        self.stage(map, key, value, false)
    }

    /// Stage a named-map insert-if-absent. Membership is decided at commit.
    /// # Errors
    /// Refuses an unallocatable batch capacity.
    pub fn insert_if_absent(&mut self, map: ScratchMapId, key: &[u8], value: &[u8]) -> Result<()> {
        self.stage(map, key, value, true)
    }

    fn stage(
        &mut self,
        map: ScratchMapId,
        key: &[u8],
        value: &[u8],
        if_absent: bool,
    ) -> Result<()> {
        self.staged.try_reserve(1).map_err(|_| allocation())?;
        self.staged.push(StagedScratchPut {
            map,
            key: Box::from(key),
            value: Box::from(value),
            if_absent,
        });
        Ok(())
    }

    #[must_use]
    pub const fn pending_entries(&self) -> usize {
        self.staged.len()
    }

    /// Abort drops the batch without changing the relation.
    pub fn abort(self) {}

    /// Commit staged puts together. On the disk tier, `MapFull` aborts the
    /// transaction, grows the virtual mapping and retries the same batch.
    /// # Errors
    /// Cancellation, allocation failure, or scratch I/O failure.
    pub fn commit(self, relation: &mut ScratchRelation) -> Result<()> {
        relation.commit_batch(self)
    }
}

impl Default for ScratchWriteBatch {
    fn default() -> Self {
        Self::new()
    }
}

/// The bounded-iteration callback: full key bytes, value bytes; `false`
/// stops the walk early.
pub type KeyValueVisit<'v> = ScratchVisit<'v>;

/// Same-environment lookup for a named map during [`ScratchRelation::visit_with_lookup`].
/// A claim walk can `get` a group header without a second `ScratchRelation`.
pub struct ScratchLookup<'a> {
    inner: ScratchLookupInner<'a>,
}

enum ScratchLookupInner<'a> {
    Ram {
        maps: &'a [RamMap; ScratchMapId::COUNT],
    },
    Lmdb {
        env: &'a ScratchEnv,
        txn: &'a heed::RoTxn<'a, heed::WithoutTls>,
        work: &'a WorkContext,
    },
}

impl ScratchLookup<'_> {
    /// Get from another roster map on this environment.
    /// # Errors
    /// As [`ScratchRelation::get_map`].
    pub fn get(&self, map: ScratchMapId, key: &[u8], out: &mut Vec<u8>) -> Result<bool> {
        out.clear();
        match self.inner {
            ScratchLookupInner::Ram { maps } => match maps[map.index()].get(key) {
                Some(value) => {
                    out.extend_from_slice(value);
                    Ok(true)
                }
                None => Ok(false),
            },
            ScratchLookupInner::Lmdb { env, txn, work } => env.get_in_txn(map, txn, work, key, out),
        }
    }
}

fn work_error(error: WorkError) -> Error {
    Error::from_store(crate::storage::store::StoreError::Work(error))
}

fn allocation() -> Error {
    Error::from_store(crate::storage::store::StoreError::Allocation)
}

/// One transient exact map: insert-if-absent, get/put, bounded iteration,
/// disposal. Consumed by lifecycle materialization and query operators.
pub struct ScratchRelation {
    tier: Tier,
    work: WorkContext,
    entries: u64,
}

impl std::fmt::Debug for ScratchRelation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScratchRelation")
            .field("entries", &self.entries)
            .finish_non_exhaustive()
    }
}

type RamMap = BTreeMap<Box<[u8]>, Box<[u8]>>;

enum Tier {
    Ram { maps: [RamMap; ScratchMapId::COUNT] },
    Lmdb(Box<ScratchEnv>),
}

impl ScratchRelation {
    #[must_use]
    pub fn new(work: &WorkContext) -> Self {
        Self {
            tier: Tier::Ram {
                maps: std::array::from_fn(|_| BTreeMap::new()),
            },
            work: work.clone(),
            entries: 0,
        }
    }

    pub(crate) const fn len(&self) -> u64 {
        self.entries
    }

    #[must_use]
    pub const fn spilled(&self) -> bool {
        matches!(self.tier, Tier::Lmdb(_))
    }

    #[cfg(test)]
    pub(crate) fn inject_map_full_before_commit(&mut self, times: u32) {
        if let Tier::Lmdb(env) = &mut self.tier {
            env.state.map_full_before_commit = times;
        }
    }

    #[cfg(test)]
    pub(crate) fn committed_txn_for_test(&self) -> Option<usize> {
        match &self.tier {
            Tier::Ram { .. } => None,
            Tier::Lmdb(env) => Some(env.env.info().last_txn_id),
        }
    }

    #[cfg(test)]
    #[expect(
        clippy::used_underscore_binding,
        reason = "Tests inspect the path owned by the RAII cleanup guard"
    )]
    pub(crate) fn scratch_path(&self) -> Option<PathBuf> {
        match &self.tier {
            Tier::Lmdb(env) => Some(env._cleanup.0.clone()),
            Tier::Ram { .. } => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn setup_at(work: &WorkContext, path: &std::path::Path) -> Result<Self> {
        let env = ScratchEnv::create_at(work, path)?;
        Ok(Self {
            tier: Tier::Lmdb(Box::new(env)),
            work: work.clone(),
            entries: 0,
        })
    }

    /// Exact insert-if-absent: `Ok(true)` iff the key was new. `value` is
    /// stored alongside (empty for pure set membership).
    /// # Errors
    /// Stopped work, allocation/scratch refusal, or scratch I/O failure.
    pub(crate) fn insert_if_absent(&mut self, key: &[u8], value: &[u8]) -> Result<bool> {
        self.work.checkpoint().map_err(work_error)?;
        let fresh = match &mut self.tier {
            Tier::Ram { maps } => {
                let map = &mut maps[ScratchMapId::Default.index()];
                if map.contains_key(key) {
                    false
                } else {
                    map.insert(Box::from(key), Box::from(value));
                    true
                }
            }
            Tier::Lmdb(env) => {
                env.insert_if_absent(&self.work, ScratchMapId::Default, key, value)?
            }
        };
        if fresh {
            self.entries += 1;
        }
        Ok(fresh)
    }

    /// Upsert: stores `value` under `key`, replacing any previous value.
    /// # Errors
    /// Cancellation, allocation, or scratch-storage failure.
    pub fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        self.work.checkpoint().map_err(work_error)?;
        match &mut self.tier {
            Tier::Ram { maps } => {
                let map = &mut maps[ScratchMapId::Default.index()];
                let fresh = !map.contains_key(key);
                map.insert(Box::from(key), Box::from(value));
                if fresh {
                    self.entries += 1;
                }
            }
            Tier::Lmdb(env) => {
                if env.put(&self.work, ScratchMapId::Default, key, value)? {
                    self.entries += 1;
                }
            }
        }
        Ok(())
    }

    /// Copy the value of `key` into `out`. `Ok(false)` clears `out`.
    /// # Errors
    /// As [`Self::insert_if_absent`].
    pub(crate) fn get(&mut self, key: &[u8], out: &mut Vec<u8>) -> Result<bool> {
        self.work.checkpoint().map_err(work_error)?;
        out.clear();
        match &mut self.tier {
            Tier::Ram { maps, .. } => match maps[ScratchMapId::Default.index()].get(key) {
                Some(value) => {
                    out.extend_from_slice(value);
                    Ok(true)
                }
                None => Ok(false),
            },
            Tier::Lmdb(env) => env.get(&self.work, ScratchMapId::Default, key, out),
        }
    }

    /// Walk every (key, value) in key order, checking cancellation per
    /// entry. The callback returns `false` to stop early.
    /// # Errors
    /// Cancellation, scratch-storage failure, or the callback's failure.
    pub fn for_each(&mut self, visit: KeyValueVisit<'_>) -> Result<()> {
        self.for_each_from(&[], visit)
    }

    /// Early-stoppable fallible visit over exact keys.
    /// # Errors
    /// As [`Self::for_each`].
    pub fn visit(&mut self, visitor: &mut impl ScratchVisitor) -> Result<()> {
        self.for_each(&mut |key, value| visitor.visit(key, value))
    }

    /// As [`Self::visit`], starting at the first key ≥ `start`.
    /// # Errors
    /// As [`Self::for_each`].
    pub fn visit_from(&mut self, start: &[u8], visitor: &mut impl ScratchVisitor) -> Result<()> {
        self.for_each_from(start, &mut |key, value| visitor.visit(key, value))
    }

    /// Put an ordered fixed-word key. Encoding is big-endian word order.
    /// # Errors
    /// As [`Self::put`].
    pub fn put_words<const WORDS: usize>(
        &mut self,
        key: ScratchWordKey<WORDS>,
        value: &[u8],
    ) -> Result<()> {
        self.put(&key.encode(), value)
    }

    /// Get an ordered fixed-word key.
    /// # Errors
    /// Cancellation or scratch-storage failure.
    pub fn get_words<const WORDS: usize>(
        &mut self,
        key: ScratchWordKey<WORDS>,
        out: &mut Vec<u8>,
    ) -> Result<bool> {
        self.get(&key.encode(), out)
    }

    /// Put an exact arbitrary key (full-byte equality, forced collisions).
    /// # Errors
    /// As [`Self::put`].
    pub fn put_exact(&mut self, key: ScratchExactKey<'_>, value: &[u8]) -> Result<()> {
        self.put(key.as_bytes(), value)
    }

    /// Put into a named map on this relation's one environment.
    /// # Errors
    /// As [`Self::put`].
    pub fn put_map(&mut self, name: ScratchMapId, key: &[u8], value: &[u8]) -> Result<()> {
        self.work.checkpoint().map_err(work_error)?;
        match &mut self.tier {
            Tier::Ram { maps } => {
                let map = &mut maps[name.index()];
                let fresh = !map.contains_key(key);
                map.insert(Box::from(key), Box::from(value));
                if fresh && name == ScratchMapId::Default {
                    self.entries += 1;
                }
            }
            Tier::Lmdb(env) => {
                if env.put(&self.work, name, key, value)? && name == ScratchMapId::Default {
                    self.entries += 1;
                }
            }
        }
        Ok(())
    }

    /// Scoped borrow of one committed value. `visit` sees [`ScratchProbe::Hit`]
    /// or [`ScratchProbe::Miss`]. Work, admission, and I/O refuse as `Err`
    /// — never as miss.
    ///
    /// L05-delivery reads [`Self::value_len`] here before copying a row.
    /// # Errors
    /// Stopped work, reservation refusal, scratch I/O, or `visit`.
    pub fn lookup<R>(
        &mut self,
        map: ScratchMapId,
        key: &[u8],
        visit: impl FnOnce(ScratchProbe<&[u8]>) -> Result<R>,
    ) -> Result<R> {
        self.work.checkpoint().map_err(work_error)?;
        match &self.tier {
            Tier::Ram { maps, .. } => {
                let probe = match maps[map.index()].get(key) {
                    Some(value) => ScratchProbe::Hit(value.as_ref()),
                    None => ScratchProbe::Miss,
                };
                visit(probe)
            }
            Tier::Lmdb(env) => env.lookup(&self.work, map, key, visit),
        }
    }

    /// Encoded size of one committed value, borrowed then released.
    /// Not a per-row RAM index: the live map is the source.
    /// # Errors
    /// As [`Self::lookup`].
    pub fn value_len(&mut self, map: ScratchMapId, key: &[u8]) -> Result<ScratchProbe<u64>> {
        self.lookup(map, key, |probe| {
            Ok(match probe {
                ScratchProbe::Hit(bytes) => ScratchProbe::Hit(bytes.len() as u64),
                ScratchProbe::Miss => ScratchProbe::Miss,
            })
        })
    }

    /// Get from a named map. `Ok(false)` clears `out`.
    /// # Errors
    /// Cancellation or scratch-storage failure.
    pub fn get_map(&mut self, name: ScratchMapId, key: &[u8], out: &mut Vec<u8>) -> Result<bool> {
        self.work.checkpoint().map_err(work_error)?;
        out.clear();
        match &mut self.tier {
            Tier::Ram { maps, .. } => match maps[name.index()].get(key) {
                Some(value) => {
                    out.extend_from_slice(value);
                    Ok(true)
                }
                None => Ok(false),
            },
            Tier::Lmdb(env) => env.get(&self.work, name, key, out),
        }
    }

    /// Early-stoppable visit of one named map.
    /// # Errors
    /// As [`Self::visit`].
    pub fn visit_map(
        &mut self,
        name: ScratchMapId,
        visitor: &mut impl ScratchVisitor,
    ) -> Result<()> {
        self.for_each_from_map(name, &[], &mut |key, value| visitor.visit(key, value))
    }

    /// As [`Self::visit_map`], starting at the first key ≥ `start`.
    /// # Errors
    /// As [`Self::visit`].
    pub fn visit_map_from(
        &mut self,
        name: ScratchMapId,
        start: &[u8],
        visitor: &mut impl ScratchVisitor,
    ) -> Result<()> {
        self.for_each_from_map(name, start, &mut |key, value| visitor.visit(key, value))
    }

    /// Visit `name` while looking up other roster maps on the same env /
    /// the same RAM tables. Pack finalize: claim cursor + token→group get.
    /// # Errors
    /// As [`Self::visit`].
    pub fn visit_with_lookup(
        &mut self,
        name: ScratchMapId,
        visitor: &mut impl FnMut(ScratchLookup<'_>, &[u8], &[u8]) -> Result<bool>,
    ) -> Result<()> {
        self.work.checkpoint().map_err(work_error)?;
        match &self.tier {
            Tier::Ram { maps, .. } => {
                // All access during this visit is shared. Keep one walk
                // and borrow its entries; neither a cloned resume key nor
                // a fresh tree search is needed for the next row.
                for (key, value) in &maps[name.index()] {
                    self.work.checkpoint().map_err(work_error)?;
                    let lookup = ScratchLookup {
                        inner: ScratchLookupInner::Ram { maps },
                    };
                    if !visitor(lookup, key, value)? {
                        return Ok(());
                    }
                }
                Ok(())
            }
            Tier::Lmdb(env) => env.visit_with_lookup(&self.work, name, visitor),
        }
    }

    fn for_each_from_map(
        &mut self,
        name: ScratchMapId,
        start: &[u8],
        visit: KeyValueVisit<'_>,
    ) -> Result<()> {
        match &mut self.tier {
            Tier::Ram { maps, .. } => {
                for (key, value) in maps[name.index()].range::<[u8], _>((
                    std::ops::Bound::Included(start),
                    std::ops::Bound::Unbounded,
                )) {
                    self.work.checkpoint().map_err(work_error)?;
                    if !visit(key, value)? {
                        return Ok(());
                    }
                }
                Ok(())
            }
            Tier::Lmdb(env) => env.for_each_from(&self.work, name, start, visit),
        }
    }

    fn commit_batch(&mut self, batch: ScratchWriteBatch) -> Result<()> {
        self.work.checkpoint().map_err(work_error)?;
        if let Tier::Ram { maps } = &mut self.tier {
            // Check cancellation before the first mutation, not mid-batch.
            // The exact dictionary's two directions must commit together
            // in RAM just as they do in an LMDB write transaction.
            for put in batch.staged {
                let map = &mut maps[put.map.index()];
                let old = map.get(put.key.as_ref());
                if put.if_absent && old.is_some() {
                    continue;
                }
                let fresh = old.is_none();
                map.insert(put.key, put.value);
                if fresh && put.map == ScratchMapId::Default {
                    self.entries += 1;
                }
            }
            return Ok(());
        }
        match &mut self.tier {
            Tier::Lmdb(env) => {
                let inserted = env.commit_staged(&self.work, batch)?;
                self.entries += inserted;
                Ok(())
            }
            Tier::Ram { .. } => unreachable!("spilled or applied above"),
        }
    }

    /// The last entry whose key is ≤ `bound` (the predecessor query the
    /// judge's coverage-run probes ride). Copies the found key into
    /// `key_out` and its value into `value_out`; `Ok(false)` clears both.
    ///
    /// Exact only for inline-sized keys (≤ [`MAX_INLINE_KEY`]): oversized
    /// bucketed keys are excluded from the walk, and `bound` itself must be
    /// inline-sized — callers whose correctness rides on this query keep
    /// their keys short by construction (the judge's fixed-width run keys).
    /// # Errors
    /// As [`Self::insert_if_absent`].
    pub(crate) fn last_at_or_before(
        &mut self,
        bound: &[u8],
        key_out: &mut Vec<u8>,
        value_out: &mut Vec<u8>,
    ) -> Result<bool> {
        debug_assert!(
            bound.len() <= MAX_INLINE_KEY,
            "predecessor bounds must be inline-sized"
        );
        self.work.checkpoint().map_err(work_error)?;
        key_out.clear();
        value_out.clear();
        match &mut self.tier {
            Tier::Ram { maps, .. } => {
                match maps[ScratchMapId::Default.index()]
                    .range::<[u8], _>((
                        std::ops::Bound::Unbounded,
                        std::ops::Bound::Included(bound),
                    ))
                    .next_back()
                {
                    Some((key, value)) => {
                        key_out.extend_from_slice(key);
                        value_out.extend_from_slice(value);
                        Ok(true)
                    }
                    None => Ok(false),
                }
            }
            Tier::Lmdb(env) => {
                env.last_at_or_before(&self.work, ScratchMapId::Default, bound, key_out, value_out)
            }
        }
    }

    /// As [`Self::for_each`], starting at the first key ≥ `start`.
    /// # Errors
    /// As [`Self::for_each`].
    pub(crate) fn for_each_from(&mut self, start: &[u8], visit: KeyValueVisit<'_>) -> Result<()> {
        match &mut self.tier {
            Tier::Ram { maps, .. } => {
                for (key, value) in maps[ScratchMapId::Default.index()].range::<[u8], _>((
                    std::ops::Bound::Included(start),
                    std::ops::Bound::Unbounded,
                )) {
                    self.work.checkpoint().map_err(work_error)?;
                    if !visit(key, value)? {
                        return Ok(());
                    }
                }
                Ok(())
            }
            Tier::Lmdb(env) => env.for_each_from(&self.work, ScratchMapId::Default, start, visit),
        }
    }

    /// Explicitly move the map into private temporary LMDB storage.
    /// # Errors
    /// Scratch directory/environment failure or stopped work; the relation stays whole (RAM
    /// tier intact) on error, so the caller can surface a typed refusal
    /// without a half-moved table.
    pub fn force_spill(&mut self) -> Result<()> {
        if self.spilled() {
            return Ok(());
        }
        let Tier::Ram { maps } = &self.tier else {
            unreachable!("spilled() checked above");
        };
        let mut env = ScratchEnv::create(&self.work)?;
        let mut batch: Vec<(&[u8], &[u8])> = Vec::with_capacity(usize::from(SPILL_BATCH));
        for map_id in ScratchMapId::ALL {
            for (key, value) in &maps[map_id.index()] {
                batch.push((key, value));
                if batch.len() == usize::from(SPILL_BATCH) {
                    env.write_batch(&self.work, map_id, &batch)?;
                    batch.clear();
                }
            }
            if !batch.is_empty() {
                env.write_batch(&self.work, map_id, &batch)?;
                batch.clear();
            }
        }
        // Ownership switches only after the copy completed; dropping the
        // RAM tier releases its ordinary allocations.
        self.tier = Tier::Lmdb(Box::new(env));
        Ok(())
    }
}

/// The execution-owned temporary LMDB environment: one directory, one
/// environment, a fixed [`ScratchMapId`] roster, disposed with the relation.
struct ScratchEnv {
    // Drop order is declaration order: the environment's native handle
    // closes FIRST, and `_cleanup` (declared last) unlinks the directory
    // strictly afterwards — close before unlink, by construction.
    env: heed::Env<heed::WithoutTls>,
    databases: [heed::Database<heed::types::Bytes, heed::types::Bytes>; ScratchMapId::COUNT],
    map_bytes: usize,
    state: ScratchState,
    _cleanup: DirCleanup,
}

/// State disjoint from the environment handle borrowed by a transaction.
/// Bucket identities advance only after a successful commit.
#[derive(Default)]
struct ScratchState {
    /// Sequence for bucket suffixes of oversized keys.
    bucket_seq: u64,
    #[cfg(test)]
    map_full_before_commit: u32,
}

/// Unlinks the scratch directory on drop — declared last in
/// [`ScratchEnv`], so the native environment has already closed.
struct DirCleanup(PathBuf);

impl Drop for DirCleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl ScratchEnv {
    fn create(work: &WorkContext) -> Result<Self> {
        work.checkpoint().map_err(work_error)?;
        let cleanup = exclusive_scratch_dir()?;
        #[cfg(test)]
        if take_fail_after_exclusive_dir() {
            drop(cleanup);
            return Err(Error::from_store(crate::storage::store::StoreError::Io(
                crate::error::IoFailure::from_io(&std::io::Error::other(
                    "injected setup failure after exclusive create",
                )),
            )));
        }
        Self::open_owned(cleanup)
    }

    #[cfg(test)]
    fn create_at(work: &WorkContext, path: &std::path::Path) -> Result<Self> {
        work.checkpoint().map_err(work_error)?;
        let cleanup = exclusive_create(path)?;
        if take_fail_after_exclusive_dir() {
            drop(cleanup);
            return Err(Error::from_store(crate::storage::store::StoreError::Io(
                crate::error::IoFailure::from_io(&std::io::Error::other(
                    "injected setup failure after exclusive create",
                )),
            )));
        }
        Self::open_owned(cleanup)
    }

    #[expect(
        unsafe_code,
        reason = "heed marks flag-setting and environment opening unsafe: \
                  NO_SYNC weakens durability and scratch uses an exclusively \
                  created directory so no second environment opens it"
    )]
    fn open_owned(cleanup: DirCleanup) -> Result<Self> {
        let mut options = heed::EnvOpenOptions::new().read_txn_without_tls();
        options
            .map_size(INITIAL_MAP)
            .max_dbs(u32::try_from(ScratchMapId::COUNT).expect("finite scratch map roster"));
        unsafe {
            options.flags(heed::EnvFlags::NO_SYNC);
        }
        let env = match unsafe { options.open(&cleanup.0) } {
            Ok(env) => env,
            Err(error) => {
                return Err(Error::from(error));
            }
        };
        let mut wtxn = env.write_txn().map_err(Error::from)?;
        let mut created = Vec::with_capacity(ScratchMapId::COUNT);
        for name in ScratchMapId::ALL {
            match env.create_database(&mut wtxn, Some(name.lmdb_name())) {
                Ok(db) => created.push(db),
                Err(error) => {
                    drop(wtxn);
                    drop(env);
                    return Err(Error::from(error));
                }
            }
        }
        if let Err(error) = wtxn.commit() {
            drop(env);
            return Err(Error::from(error));
        }
        let databases = created.try_into().map_err(|_| allocation())?;
        Ok(Self {
            env,
            databases,
            map_bytes: INITIAL_MAP,
            state: ScratchState::default(),
            _cleanup: cleanup,
        })
    }

    fn db(&self, map: ScratchMapId) -> heed::Database<heed::types::Bytes, heed::types::Bytes> {
        self.databases[map.index()]
    }

    #[expect(
        unsafe_code,
        reason = "heed::Env::resize requires no active transactions; the \
                  scratch env is owned by one execution and grow is only \
                  called between its bounded operations, with no live \
                  transaction"
    )]
    fn grow(&mut self) -> Result<()> {
        let next = self.map_bytes.checked_mul(2).ok_or_else(allocation)?;
        // SAFETY (heed resize contract): the scratch env is owned by one
        // execution and no transaction is live between this relation's
        // bounded operations.
        unsafe { self.env.resize(next) }.map_err(Error::from)?;
        self.map_bytes = next;
        Ok(())
    }

    fn with_retry<T>(&mut self, mut operation: impl FnMut(&mut Self) -> Result<T>) -> Result<T> {
        loop {
            match operation(self) {
                Ok(result) => return Ok(result),
                Err(Error::Lmdb(crate::error::LmdbFailure::Mdb(heed::MdbError::MapFull))) => {
                    self.grow()?;
                }
                other => return other,
            }
        }
    }

    /// Bounded physical key: short keys inline behind a 0x00 tag;
    /// oversized keys become `0xFF ‖ fingerprint16 ‖ seq` buckets whose
    /// values carry the full logical key for exact comparison.
    fn inline_key(key: &[u8], out: &mut Vec<u8>) -> bool {
        if key.len() <= MAX_INLINE_KEY {
            out.clear();
            out.push(0x00);
            out.extend_from_slice(key);
            true
        } else {
            false
        }
    }

    fn bucket_prefix(key: &[u8], out: &mut Vec<u8>) {
        out.clear();
        out.push(0xFF);
        let digest = blake3::hash(key);
        out.extend_from_slice(&digest.as_bytes()[..16]);
    }

    /// Find an oversized key's physical slot by exact comparison inside
    /// its bucket. Returns the full physical key if present.
    fn find_bucket_slot(
        &self,
        map: ScratchMapId,
        txn: &heed::RoTxn<'_, heed::WithoutTls>,
        work: &WorkContext,
        prefix: &[u8],
        key: &[u8],
    ) -> Result<Option<Vec<u8>>> {
        let range = self.db(map).prefix_iter(txn, prefix).map_err(Error::from)?;
        for entry in range {
            work.checkpoint().map_err(work_error)?;
            let (physical, value) = entry.map_err(Error::from)?;
            let (stored_key, _) = split_bucket_value(value)?;
            if stored_key == key {
                return Ok(Some(physical.to_vec()));
            }
        }
        Ok(None)
    }

    fn insert_if_absent(
        &mut self,
        work: &WorkContext,
        map: ScratchMapId,
        key: &[u8],
        value: &[u8],
    ) -> Result<bool> {
        let work = work.clone();
        self.with_retry(move |env| {
            work.checkpoint().map_err(work_error)?;
            let db = env.db(map);
            let mut physical = Vec::new();
            if Self::inline_key(key, &mut physical) {
                let mut wtxn = env.env.write_txn().map_err(Error::from)?;
                if db
                    .get(&wtxn, physical.as_slice())
                    .map_err(Error::from)?
                    .is_some()
                {
                    return Ok(false);
                }
                db.put(&mut wtxn, physical.as_slice(), value)
                    .map_err(Error::from)?;
                env.state.finish_write(wtxn, env.state.bucket_seq)?;
                return Ok(true);
            }
            // Oversized key: exact bucket membership before insertion.
            let mut prefix = Vec::new();
            Self::bucket_prefix(key, &mut prefix);
            {
                let rtxn = env.env.read_txn().map_err(Error::from)?;
                if env
                    .find_bucket_slot(map, &rtxn, &work, &prefix, key)?
                    .is_some()
                {
                    return Ok(false);
                }
            }
            let mut wtxn = env.env.write_txn().map_err(Error::from)?;
            let mut physical = prefix;
            let seq = env.state.bucket_seq;
            physical.extend_from_slice(&seq.to_be_bytes());
            let stored = join_bucket_value(key, value);
            let next_seq = seq.checked_add(1).ok_or_else(allocation)?;
            db.put(&mut wtxn, physical.as_slice(), stored.as_slice())
                .map_err(Error::from)?;
            env.state.finish_write(wtxn, next_seq)?;
            Ok(true)
        })
    }

    fn put(
        &mut self,
        work: &WorkContext,
        map: ScratchMapId,
        key: &[u8],
        value: &[u8],
    ) -> Result<bool> {
        let work = work.clone();
        self.with_retry(move |env| {
            work.checkpoint().map_err(work_error)?;
            let db = env.db(map);
            let mut physical = Vec::new();
            if Self::inline_key(key, &mut physical) {
                let mut wtxn = env.env.write_txn().map_err(Error::from)?;
                let fresh = db
                    .get(&wtxn, physical.as_slice())
                    .map_err(Error::from)?
                    .is_none();
                db.put(&mut wtxn, physical.as_slice(), value)
                    .map_err(Error::from)?;
                env.state.finish_write(wtxn, env.state.bucket_seq)?;
                return Ok(fresh);
            }
            let mut prefix = Vec::new();
            Self::bucket_prefix(key, &mut prefix);
            let existing = {
                let rtxn = env.env.read_txn().map_err(Error::from)?;
                env.find_bucket_slot(map, &rtxn, &work, &prefix, key)?
            };
            let mut wtxn = env.env.write_txn().map_err(Error::from)?;
            let (physical, fresh, seq) = if let Some(physical) = existing {
                (physical, false, None)
            } else {
                let mut physical = prefix.clone();
                let seq = env.state.bucket_seq;
                physical.extend_from_slice(&seq.to_be_bytes());
                (physical, true, Some(seq))
            };
            let stored = join_bucket_value(key, value);
            let next_seq = match seq {
                Some(seq) => seq.checked_add(1).ok_or_else(allocation)?,
                None => env.state.bucket_seq,
            };
            db.put(&mut wtxn, physical.as_slice(), stored.as_slice())
                .map_err(Error::from)?;
            env.state.finish_write(wtxn, next_seq)?;
            Ok(fresh)
        })
    }

    fn get(
        &mut self,
        work: &WorkContext,
        map: ScratchMapId,
        key: &[u8],
        out: &mut Vec<u8>,
    ) -> Result<bool> {
        let rtxn = self.env.read_txn().map_err(Error::from)?;
        self.get_in_txn(map, &rtxn, work, key, out)
    }

    fn get_in_txn(
        &self,
        map: ScratchMapId,
        txn: &heed::RoTxn<'_, heed::WithoutTls>,
        work: &WorkContext,
        key: &[u8],
        out: &mut Vec<u8>,
    ) -> Result<bool> {
        let db = self.db(map);
        let mut physical = Vec::new();
        if Self::inline_key(key, &mut physical) {
            return match db.get(txn, physical.as_slice()).map_err(Error::from)? {
                Some(value) => {
                    out.extend_from_slice(value);
                    Ok(true)
                }
                None => Ok(false),
            };
        }
        let mut prefix = Vec::new();
        Self::bucket_prefix(key, &mut prefix);
        match self.find_bucket_slot(map, txn, work, &prefix, key)? {
            Some(physical) => {
                let value = db
                    .get(txn, physical.as_slice())
                    .map_err(Error::from)?
                    .ok_or(Error::Corruption(
                        crate::error::CorruptionError::MalformedValue("scratch bucket slot"),
                    ))?;
                let (_, payload) = split_bucket_value(value)?;
                out.extend_from_slice(payload);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    fn lookup<R>(
        &self,
        work: &WorkContext,
        map: ScratchMapId,
        key: &[u8],
        visit: impl FnOnce(ScratchProbe<&[u8]>) -> Result<R>,
    ) -> Result<R> {
        let rtxn = self.env.read_txn().map_err(Error::from)?;
        let db = self.db(map);
        let mut physical = Vec::new();
        if Self::inline_key(key, &mut physical) {
            return match db.get(&rtxn, physical.as_slice()).map_err(Error::from)? {
                Some(value) => visit(ScratchProbe::Hit(value)),
                None => visit(ScratchProbe::Miss),
            };
        }
        let mut prefix = Vec::new();
        Self::bucket_prefix(key, &mut prefix);
        match self.find_bucket_slot(map, &rtxn, work, &prefix, key)? {
            Some(physical) => {
                let value = db
                    .get(&rtxn, physical.as_slice())
                    .map_err(Error::from)?
                    .ok_or(Error::Corruption(
                        crate::error::CorruptionError::MalformedValue("scratch bucket slot"),
                    ))?;
                let (_, payload) = split_bucket_value(value)?;
                visit(ScratchProbe::Hit(payload))
            }
            None => visit(ScratchProbe::Miss),
        }
    }

    fn visit_with_lookup(
        &self,
        work: &WorkContext,
        map: ScratchMapId,
        visitor: &mut impl FnMut(ScratchLookup<'_>, &[u8], &[u8]) -> Result<bool>,
    ) -> Result<()> {
        let rtxn = self.env.read_txn().map_err(Error::from)?;
        // Advance the physical cursor, not a reconstructed logical lower
        // bound. Wide keys live in hash buckets, so their logical bytes
        // cannot serve as a continuation key for the physical map.
        let range = self.db(map).iter(&rtxn).map_err(Error::from)?;
        for entry in range {
            work.checkpoint().map_err(work_error)?;
            let (physical, raw) = entry.map_err(Error::from)?;
            let (key, value) = match physical.first() {
                Some(0x00) => (&physical[1..], raw),
                Some(0xFF) => split_bucket_value(raw)?,
                _ => {
                    return Err(Error::Corruption(
                        crate::error::CorruptionError::MalformedValue("scratch key tag"),
                    ));
                }
            };
            let lookup = ScratchLookup {
                inner: ScratchLookupInner::Lmdb {
                    env: self,
                    txn: &rtxn,
                    work,
                },
            };
            if !visitor(lookup, key, value)? {
                return Ok(());
            }
        }
        Ok(())
    }

    /// Key-ordered walk from the first logical key ≥ `start`. Exact for
    /// inline-sized keys (the log/seq use); oversized bucketed keys sort
    /// after every inline key, in fingerprint order — set semantics only.
    fn for_each_from(
        &mut self,
        work: &WorkContext,
        map: ScratchMapId,
        start: &[u8],
        visit: KeyValueVisit<'_>,
    ) -> Result<()> {
        let rtxn = self.env.read_txn().map_err(Error::from)?;
        let mut lower = Vec::with_capacity(1 + start.len().min(MAX_INLINE_KEY));
        lower.push(0x00);
        lower.extend_from_slice(&start[..start.len().min(MAX_INLINE_KEY)]);
        let bounds: (std::ops::Bound<&[u8]>, std::ops::Bound<&[u8]>) = (
            std::ops::Bound::Included(lower.as_slice()),
            std::ops::Bound::Unbounded,
        );
        let range = self.db(map).range(&rtxn, &bounds).map_err(Error::from)?;
        for entry in range {
            work.checkpoint().map_err(work_error)?;
            let (physical, value) = entry.map_err(Error::from)?;
            let keep = match physical.first() {
                Some(0x00) => visit(&physical[1..], value)?,
                Some(0xFF) => {
                    let (key, payload) = split_bucket_value(value)?;
                    visit(key, payload)?
                }
                _ => {
                    return Err(Error::Corruption(
                        crate::error::CorruptionError::MalformedValue("scratch key tag"),
                    ));
                }
            };
            if !keep {
                return Ok(());
            }
        }
        Ok(())
    }

    /// Inline-key predecessor query (see `ScratchRelation::last_at_or_before`
    /// for the exactness contract): the last inline entry ≤ `bound`.
    fn last_at_or_before(
        &mut self,
        work: &WorkContext,
        map: ScratchMapId,
        bound: &[u8],
        key_out: &mut Vec<u8>,
        value_out: &mut Vec<u8>,
    ) -> Result<bool> {
        let rtxn = self.env.read_txn().map_err(Error::from)?;
        let mut upper = Vec::with_capacity(1 + bound.len());
        upper.push(0x00);
        upper.extend_from_slice(bound);
        // Bucketed (0xFF-tagged) physical keys sort after every inline key,
        // so the Included upper bound excludes them by construction.
        let bounds: (std::ops::Bound<&[u8]>, std::ops::Bound<&[u8]>) = (
            std::ops::Bound::Unbounded,
            std::ops::Bound::Included(upper.as_slice()),
        );
        let mut range = self
            .db(map)
            .rev_range(&rtxn, &bounds)
            .map_err(Error::from)?;
        match range.next() {
            None => Ok(false),
            Some(entry) => {
                work.checkpoint().map_err(work_error)?;
                let (physical, value) = entry.map_err(Error::from)?;
                match physical.first() {
                    Some(0x00) => {
                        key_out.extend_from_slice(&physical[1..]);
                        value_out.extend_from_slice(value);
                        Ok(true)
                    }
                    _ => Err(Error::Corruption(
                        crate::error::CorruptionError::MalformedValue("scratch key tag"),
                    )),
                }
            }
        }
    }

    fn write_batch(
        &mut self,
        work: &WorkContext,
        map: ScratchMapId,
        batch: &[(&[u8], &[u8])],
    ) -> Result<()> {
        // The spill copy: entries come from the RAM tier (already
        // deduplicated and absent from the fresh environment), so blind
        // puts inside ONE transaction are exact — no per-entry commit.
        let work = work.clone();
        self.with_retry(move |env| {
            let db = env.db(map);
            let mut wtxn = env.env.write_txn().map_err(Error::from)?;
            let mut physical = Vec::new();
            let mut seq = env.state.bucket_seq;
            for (key, value) in batch {
                work.checkpoint().map_err(work_error)?;
                if Self::inline_key(key, &mut physical) {
                    db.put(&mut wtxn, physical.as_slice(), value)
                        .map_err(Error::from)?;
                } else {
                    Self::bucket_prefix(key, &mut physical);
                    physical.extend_from_slice(&seq.to_be_bytes());
                    seq = seq.checked_add(1).ok_or_else(allocation)?;
                    let stored = join_bucket_value(key, value);
                    db.put(&mut wtxn, physical.as_slice(), stored.as_slice())
                        .map_err(Error::from)?;
                }
            }
            work.checkpoint().map_err(work_error)?;
            env.state.finish_write(wtxn, seq)?;
            Ok(())
        })
    }

    fn commit_staged(&mut self, work: &WorkContext, batch: ScratchWriteBatch) -> Result<u64> {
        let work = work.clone();
        let staged = batch.staged;
        self.with_retry(move |env| {
            let mut wtxn = env.env.write_txn().map_err(Error::from)?;
            let mut inserted = 0u64;
            let mut seq = env.state.bucket_seq;
            let mut physical = Vec::new();
            for put in &staged {
                work.checkpoint().map_err(work_error)?;
                let db = env.db(put.map);
                if Self::inline_key(&put.key, &mut physical) {
                    // NO_OVERWRITE either inserts or returns the existing
                    // value in one tree walk. Fresh staged keys need no
                    // separate membership search before their insertion.
                    let exists = db
                        .get_or_put(&mut wtxn, physical.as_slice(), put.value.as_ref())
                        .map_err(Error::from)?
                        .is_some();
                    if put.if_absent && exists {
                        continue;
                    }
                    if !exists && put.map == ScratchMapId::Default {
                        inserted += 1;
                    }
                    if exists {
                        db.put(&mut wtxn, physical.as_slice(), put.value.as_ref())
                            .map_err(Error::from)?;
                    }
                    continue;
                }
                let mut prefix = Vec::new();
                Self::bucket_prefix(&put.key, &mut prefix);
                let existing = env.find_bucket_slot(put.map, &wtxn, &work, &prefix, &put.key)?;
                if put.if_absent && existing.is_some() {
                    continue;
                }
                let (physical_key, fresh) = if let Some(physical) = existing {
                    (physical, false)
                } else {
                    let mut physical = prefix;
                    physical.extend_from_slice(&seq.to_be_bytes());
                    seq = seq.checked_add(1).ok_or_else(allocation)?;
                    (physical, true)
                };
                let stored = join_bucket_value(&put.key, &put.value);
                if fresh && put.map == ScratchMapId::Default {
                    inserted += 1;
                }
                db.put(&mut wtxn, physical_key.as_slice(), stored.as_slice())
                    .map_err(Error::from)?;
            }
            work.checkpoint().map_err(work_error)?;
            env.state.finish_write(wtxn, seq)?;
            Ok(inserted)
        })
    }
}

impl ScratchState {
    fn finish_write(&mut self, wtxn: heed::RwTxn<'_>, next_bucket_seq: u64) -> Result<()> {
        #[cfg(test)]
        if self.map_full_before_commit > 0 {
            self.map_full_before_commit -= 1;
            drop(wtxn);
            return Err(Error::Lmdb(crate::error::LmdbFailure::Mdb(
                heed::MdbError::MapFull,
            )));
        }
        wtxn.commit().map_err(Error::from)?;
        self.bucket_seq = next_bucket_seq;
        Ok(())
    }
}

/// Exclusive temporary identity first; cleanup is installed only after
/// this process created the directory. A colliding path is not adopted
/// and is never unlinked.
fn exclusive_scratch_dir() -> Result<DirCleanup> {
    for attempt in 0..32u32 {
        let path = std::env::temp_dir().join(format!(
            "bumbledb-scratch-{}-{:x}-{attempt}",
            std::process::id(),
            fastrand_seed()
        ));
        match std::fs::create_dir(&path) {
            Ok(()) => return Ok(DirCleanup(path)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(Error::from(error)),
        }
    }
    Err(Error::from_store(crate::storage::store::StoreError::Io(
        crate::error::IoFailure::from_io(&std::io::Error::other(
            "scratch exclusive directory exhausted",
        )),
    )))
}

#[cfg(test)]
fn exclusive_create(path: &std::path::Path) -> Result<DirCleanup> {
    match std::fs::create_dir(path) {
        Ok(()) => Ok(DirCleanup(path.to_path_buf())),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Err(Error::from_store(
            crate::storage::store::StoreError::Io(crate::error::IoFailure::from_io(&error)),
        )),
        Err(error) => Err(Error::from(error)),
    }
}

#[cfg(test)]
thread_local! {
    static FAIL_AFTER_EXCLUSIVE_DIR: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
fn take_fail_after_exclusive_dir() -> bool {
    FAIL_AFTER_EXCLUSIVE_DIR.with(|flag| flag.replace(false))
}

#[cfg(test)]
pub(crate) fn inject_setup_fail_after_exclusive_dir() {
    FAIL_AFTER_EXCLUSIVE_DIR.with(|flag| flag.set(true));
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "a scratch-name seed mixes only the low clock bits"
)]
fn fastrand_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let addr = (&raw const now) as u64;
    (now as u64) ^ addr.rotate_left(17)
}

fn join_bucket_value(key: &[u8], value: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + key.len() + value.len());
    out.extend_from_slice(&(key.len() as u64).to_be_bytes());
    out.extend_from_slice(key);
    out.extend_from_slice(value);
    out
}

fn split_bucket_value(stored: &[u8]) -> Result<(&[u8], &[u8])> {
    let malformed = || {
        Error::Corruption(crate::error::CorruptionError::MalformedValue(
            "scratch bucket value",
        ))
    };
    let (len, rest) = stored.split_at_checked(8).ok_or_else(malformed)?;
    let key_len = usize::try_from(u64::from_be_bytes(len.try_into().expect("eight bytes")))
        .map_err(|_| malformed())?;
    rest.split_at_checked(key_len).ok_or_else(malformed)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod bounds;
