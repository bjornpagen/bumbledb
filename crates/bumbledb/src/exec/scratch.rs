//! The one charged transient relation/map (chapter 12 §4): every
//! intermediate owner that can outgrow RAM — projection/union distinct
//! sets, aggregate group state, recursion seen/frontier sets, completed
//! results — spills through this abstraction, never through a private
//! external-sort/partition framework.
//!
//! Two tiers, one logical map of exact byte keys:
//!
//! - **RAM**: a fallibly grown ordered table, charged to the operation's
//!   `working_bytes` in chunked reservations before growth.
//! - **Temporary LMDB**: an execution-owned scratch environment created on
//!   first crossing of the RAM allowance. Existing entries copy over in
//!   bounded batches (the transfer's transient overlap is reserved before
//!   the copy starts); once spilled, the execution stays on disk — no
//!   oscillating tier manager. Scratch writes never claim authoritative
//!   durability: the environment is `NO_SYNC`, unreachable from any
//!   persistent-store constructor, and its loss loses only this query
//!   attempt (ENG-008 stays intact — the production store has no such
//!   flag anywhere).
//!
//! Keys are **exact full bytes** — the map is ordered by the key bytes and
//! never consults a hash verdict, so forced fingerprint collisions cannot
//! merge distinct tuples (Q-COLLISION). Long logical keys are the caller's
//! encoded words/bytes; the scratch env bounds physical LMDB keys by
//! hashing oversized keys into exact-checked candidate buckets, comparing
//! full key bytes within a bucket.
//!
//! Disposal closes the native environment before unlinking its directory;
//! a failed execution drops the whole relation and cleans only its own
//! validated scratch path.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::error::{Error, Result};
use crate::work::{ByteKind, ByteReservation, WorkContext, WorkError};

/// The default RAM allowance per scratch relation before the LMDB tier
/// takes over. A tuning default (chapter 40 magic-number discipline: the
/// crossover is measured at F3), never a semantic limit — the disk tier is
/// complete for every operator.
pub(crate) const DEFAULT_RAM_BYTES: usize = 8 << 20;

/// Working-byte reservations are taken in chunks of this size so the
/// charge vector stays small while growth is still charged before it
/// happens.
const CHARGE_CHUNK: usize = 64 << 10;

/// Copy batch size for the RAM→LMDB transition (entries per transaction).
const SPILL_BATCH: usize = 1024;

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

/// The bounded-iteration callback: full key bytes, value bytes; `false`
/// stops the walk early.
type KeyValueVisit<'v> = &'v mut dyn FnMut(&[u8], &[u8]) -> Result<bool>;

fn work_error(error: WorkError) -> Error {
    Error::from_store(crate::storage::store::StoreError::Work(error))
}

fn allocation() -> Error {
    Error::from_store(crate::storage::store::StoreError::Allocation)
}

/// One transient exact map: insert-if-absent, get/put, bounded iteration,
/// disposal. Not a public API and not a cache service.
pub(crate) struct ScratchRelation {
    tier: Tier,
    ram_limit: usize,
    work: WorkContext,
    entries: u64,
}

enum Tier {
    Ram {
        map: BTreeMap<Box<[u8]>, Box<[u8]>>,
        bytes: usize,
        charged: usize,
        charges: Vec<ByteReservation>,
    },
    Lmdb(Box<ScratchEnv>),
}

impl ScratchRelation {
    pub(crate) fn new(work: &WorkContext, ram_limit: usize) -> Self {
        Self {
            tier: Tier::Ram {
                map: BTreeMap::new(),
                bytes: 0,
                charged: 0,
                charges: Vec::new(),
            },
            ram_limit,
            work: work.clone(),
            entries: 0,
        }
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "test-suite constructor over DEFAULT_RAM_BYTES")
    )]
    pub(crate) fn with_default_budget(work: &WorkContext) -> Self {
        Self::new(work, DEFAULT_RAM_BYTES)
    }

    pub(crate) const fn len(&self) -> u64 {
        self.entries
    }

    /// Re-home this relation onto a caller-supplied operation ledger: every
    /// SUBSEQUENT step/checkpoint/reservation charges `work` instead of the
    /// ledger the relation was built under. The chapter-35 retained-result
    /// seam: a sealed result outlives its execute operation, and its scratch
    /// reads must not keep consulting that operation's expired deadline.
    ///
    /// Release stays linear by construction: byte reservations already taken
    /// ([`ByteReservation`]) each own an `Arc` to their ORIGINATING ledger
    /// and refund exactly their charge on drop — rebinding moves none of
    /// them, so nothing releases twice and nothing leaks. The scratch
    /// directory itself is unlinked exactly once, by drop order, regardless
    /// of how many times the relation was rebound.
    pub(crate) fn rebind_work(&mut self, work: &WorkContext) {
        self.work = work.clone();
    }

    pub(crate) const fn spilled(&self) -> bool {
        matches!(self.tier, Tier::Lmdb(_))
    }

    /// Exact insert-if-absent: `Ok(true)` iff the key was new. `value` is
    /// stored alongside (empty for pure set membership).
    /// # Errors
    /// Stopped work, allocation/scratch refusal, or scratch I/O failure.
    pub(crate) fn insert_if_absent(&mut self, key: &[u8], value: &[u8]) -> Result<bool> {
        self.work.step(1).map_err(work_error)?;
        self.maybe_spill(key.len() + value.len())?;
        let fresh = match &mut self.tier {
            Tier::Ram {
                map,
                bytes,
                charged,
                charges,
            } => {
                if map.contains_key(key) {
                    false
                } else {
                    let grown = key.len() + value.len() + 64;
                    charge(&self.work, bytes, charged, charges, grown)?;
                    map.insert(Box::from(key), Box::from(value));
                    true
                }
            }
            Tier::Lmdb(env) => env.insert_if_absent(&self.work, key, value)?,
        };
        if fresh {
            self.entries += 1;
        }
        Ok(fresh)
    }

    /// Upsert: stores `value` under `key`, replacing any previous value.
    /// # Errors
    /// As [`Self::insert_if_absent`].
    pub(crate) fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        self.work.step(1).map_err(work_error)?;
        self.maybe_spill(key.len() + value.len())?;
        match &mut self.tier {
            Tier::Ram {
                map,
                bytes,
                charged,
                charges,
            } => {
                let fresh = !map.contains_key(key);
                let grown = key.len() + value.len() + 64;
                charge(&self.work, bytes, charged, charges, grown)?;
                map.insert(Box::from(key), Box::from(value));
                if fresh {
                    self.entries += 1;
                }
            }
            Tier::Lmdb(env) => {
                if env.put(&self.work, key, value)? {
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
        self.work.step(1).map_err(work_error)?;
        out.clear();
        match &mut self.tier {
            Tier::Ram { map, .. } => match map.get(key) {
                Some(value) => {
                    out.extend_from_slice(value);
                    Ok(true)
                }
                None => Ok(false),
            },
            Tier::Lmdb(env) => env.get(&self.work, key, out),
        }
    }

    /// Walk every (key, value) in key order, charging one work step per
    /// entry. The callback returns `false` to stop early.
    /// # Errors
    /// As [`Self::insert_if_absent`], or the callback's failure.
    pub(crate) fn for_each(&mut self, visit: KeyValueVisit<'_>) -> Result<()> {
        self.for_each_from(&[], visit)
    }

    /// As [`Self::for_each`], starting at the first key ≥ `start`.
    /// # Errors
    /// As [`Self::for_each`].
    pub(crate) fn for_each_from(&mut self, start: &[u8], visit: KeyValueVisit<'_>) -> Result<()> {
        match &mut self.tier {
            Tier::Ram { map, .. } => {
                for (key, value) in map.range::<[u8], _>((
                    std::ops::Bound::Included(start),
                    std::ops::Bound::Unbounded,
                )) {
                    self.work.step(1).map_err(work_error)?;
                    if !visit(key, value)? {
                        return Ok(());
                    }
                }
                Ok(())
            }
            Tier::Lmdb(env) => env.for_each_from(&self.work, start, visit),
        }
    }

    /// Force the RAM→LMDB transition now (the Q-FALLBACK/F-RESOURCE test
    /// affordance, and the explicit route when a caller knows the working
    /// set will not fit).
    /// # Errors
    /// Scratch directory/environment failure, stopped work, or refusal of
    /// the transfer's reserved overlap — the relation stays whole (RAM
    /// tier intact) on error, so the caller can surface a typed refusal
    /// without a half-moved table.
    pub(crate) fn force_spill(&mut self) -> Result<()> {
        if self.spilled() {
            return Ok(());
        }
        let Tier::Ram {
            map,
            bytes,
            charged: _,
            charges: _,
        } = &self.tier
        else {
            unreachable!("spilled() checked above");
        };
        // Reserve the transfer overlap BEFORE creating anything: the old
        // table stays charged, and the new environment's copy is charged
        // as scratch for the whole payload.
        let payload = *bytes as u64;
        let _transfer = self
            .work
            .reserve(ByteKind::Scratch, payload)
            .map_err(work_error)?;
        let mut env = ScratchEnv::create(&self.work)?;
        let mut batch: Vec<(&[u8], &[u8])> = Vec::with_capacity(SPILL_BATCH);
        for (key, value) in map {
            batch.push((key, value));
            if batch.len() == SPILL_BATCH {
                env.write_batch(&self.work, &batch)?;
                batch.clear();
            }
        }
        if !batch.is_empty() {
            env.write_batch(&self.work, &batch)?;
        }
        drop(batch);
        // Ownership switches only after the copy completed; dropping the
        // RAM tier releases its working-byte reservations.
        self.tier = Tier::Lmdb(Box::new(env));
        Ok(())
    }

    fn maybe_spill(&mut self, incoming: usize) -> Result<()> {
        if let Tier::Ram { bytes, .. } = &self.tier
            && bytes + incoming + 64 > self.ram_limit
        {
            self.force_spill()?;
        }
        Ok(())
    }
}

fn charge(
    work: &WorkContext,
    bytes: &mut usize,
    charged: &mut usize,
    reservations: &mut Vec<ByteReservation>,
    grown: usize,
) -> Result<()> {
    *bytes += grown;
    while *bytes > *charged {
        let chunk = work
            .reserve(ByteKind::Working, CHARGE_CHUNK as u64)
            .map_err(work_error)?;
        reservations.push(chunk);
        *charged += CHARGE_CHUNK;
    }
    Ok(())
}

/// The execution-owned temporary LMDB environment: one directory, one
/// environment, a fixed database roster (exactly one map — namespacing is
/// the key prefix), disposed with the relation.
struct ScratchEnv {
    // Drop order is declaration order: the environment's native handle
    // closes FIRST, and `cleanup` (declared last) unlinks the directory
    // strictly afterwards — close before unlink, by construction.
    env: heed::Env<heed::WithoutTls>,
    database: heed::Database<heed::types::Bytes, heed::types::Bytes>,
    map_bytes: usize,
    /// Charged scratch growth (retained until disposal).
    charges: Vec<ByteReservation>,
    charged: usize,
    written: usize,
    /// Sequence for bucket suffixes of oversized keys.
    bucket_seq: u64,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "held for Drop: unlinks the scratch directory after the \
                      environment closes; tests read its path"
        )
    )]
    cleanup: DirCleanup,
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
    #[expect(
        unsafe_code,
        reason = "heed marks flag-setting and environment opening unsafe: \
                  double-opening one path in a process is LMDB UB, and \
                  NO_SYNC weakens durability. The directory is created here \
                  with a process/seed-unique name so no second environment \
                  opens it, and scratch is disposable by contract."
    )]
    fn create(work: &WorkContext) -> Result<Self> {
        work.checkpoint().map_err(work_error)?;
        let path = std::env::temp_dir().join(format!(
            "bumbledb-scratch-{}-{:x}",
            std::process::id(),
            fastrand_seed()
        ));
        std::fs::create_dir_all(&path)?;
        let mut options = heed::EnvOpenOptions::new().read_txn_without_tls();
        options.map_size(INITIAL_MAP).max_dbs(1);
        // Scratch is disposable by contract: NO_SYNC is confined here and
        // never reachable through any persistent-store constructor.
        unsafe {
            options.flags(heed::EnvFlags::NO_SYNC);
        }
        // SAFETY (heed open contract): the directory was just created with
        // a process/seed-unique name — no second environment opens it.
        let env = unsafe { options.open(&path) }.map_err(Error::from)?;
        let mut wtxn = env.write_txn().map_err(Error::from)?;
        let database = env
            .create_database(&mut wtxn, Some("scratch"))
            .map_err(Error::from)?;
        wtxn.commit().map_err(Error::from)?;
        Ok(Self {
            env,
            database,
            map_bytes: INITIAL_MAP,
            charges: Vec::new(),
            charged: 0,
            written: 0,
            bucket_seq: 0,
            cleanup: DirCleanup(path),
        })
    }

    fn charge(&mut self, work: &WorkContext, grown: usize) -> Result<()> {
        self.written += grown;
        while self.written > self.charged {
            let chunk = work
                .reserve(ByteKind::Scratch, CHARGE_CHUNK as u64)
                .map_err(work_error)?;
            self.charges.push(chunk);
            self.charged += CHARGE_CHUNK;
        }
        Ok(())
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
        txn: &heed::RoTxn<'_, heed::WithoutTls>,
        work: &WorkContext,
        prefix: &[u8],
        key: &[u8],
    ) -> Result<Option<Vec<u8>>> {
        let range = self
            .database
            .prefix_iter(txn, prefix)
            .map_err(Error::from)?;
        for entry in range {
            work.step(1).map_err(work_error)?;
            let (physical, value) = entry.map_err(Error::from)?;
            let (stored_key, _) = split_bucket_value(value)?;
            if stored_key == key {
                return Ok(Some(physical.to_vec()));
            }
        }
        Ok(None)
    }

    fn insert_if_absent(&mut self, work: &WorkContext, key: &[u8], value: &[u8]) -> Result<bool> {
        let work = work.clone();
        self.with_retry(move |env| {
            let mut physical = Vec::new();
            if Self::inline_key(key, &mut physical) {
                let mut wtxn = env.env.write_txn().map_err(Error::from)?;
                if env
                    .database
                    .get(&wtxn, physical.as_slice())
                    .map_err(Error::from)?
                    .is_some()
                {
                    return Ok(false);
                }
                env.database
                    .put(&mut wtxn, physical.as_slice(), value)
                    .map_err(Error::from)?;
                wtxn.commit().map_err(Error::from)?;
                env.charge(&work, physical.len() + value.len() + 32)?;
                return Ok(true);
            }
            // Oversized key: exact bucket membership before insertion.
            let mut prefix = Vec::new();
            Self::bucket_prefix(key, &mut prefix);
            {
                let rtxn = env.env.read_txn().map_err(Error::from)?;
                if env.find_bucket_slot(&rtxn, &work, &prefix, key)?.is_some() {
                    return Ok(false);
                }
            }
            let mut wtxn = env.env.write_txn().map_err(Error::from)?;
            let mut physical = prefix;
            physical.extend_from_slice(&env.bucket_seq.to_be_bytes());
            env.bucket_seq += 1;
            let stored = join_bucket_value(key, value);
            env.database
                .put(&mut wtxn, physical.as_slice(), stored.as_slice())
                .map_err(Error::from)?;
            wtxn.commit().map_err(Error::from)?;
            env.charge(&work, physical.len() + stored.len() + 32)?;
            Ok(true)
        })
    }

    fn put(&mut self, work: &WorkContext, key: &[u8], value: &[u8]) -> Result<bool> {
        let work = work.clone();
        self.with_retry(move |env| {
            let mut physical = Vec::new();
            if Self::inline_key(key, &mut physical) {
                let mut wtxn = env.env.write_txn().map_err(Error::from)?;
                let fresh = env
                    .database
                    .get(&wtxn, physical.as_slice())
                    .map_err(Error::from)?
                    .is_none();
                env.database
                    .put(&mut wtxn, physical.as_slice(), value)
                    .map_err(Error::from)?;
                wtxn.commit().map_err(Error::from)?;
                env.charge(&work, physical.len() + value.len() + 32)?;
                return Ok(fresh);
            }
            let mut prefix = Vec::new();
            Self::bucket_prefix(key, &mut prefix);
            let existing = {
                let rtxn = env.env.read_txn().map_err(Error::from)?;
                env.find_bucket_slot(&rtxn, &work, &prefix, key)?
            };
            let mut wtxn = env.env.write_txn().map_err(Error::from)?;
            let (physical, fresh) = if let Some(physical) = existing {
                (physical, false)
            } else {
                let mut physical = prefix.clone();
                physical.extend_from_slice(&env.bucket_seq.to_be_bytes());
                env.bucket_seq += 1;
                (physical, true)
            };
            let stored = join_bucket_value(key, value);
            env.database
                .put(&mut wtxn, physical.as_slice(), stored.as_slice())
                .map_err(Error::from)?;
            wtxn.commit().map_err(Error::from)?;
            env.charge(&work, physical.len() + stored.len() + 32)?;
            Ok(fresh)
        })
    }

    fn get(&mut self, work: &WorkContext, key: &[u8], out: &mut Vec<u8>) -> Result<bool> {
        let rtxn = self.env.read_txn().map_err(Error::from)?;
        let mut physical = Vec::new();
        if Self::inline_key(key, &mut physical) {
            return match self
                .database
                .get(&rtxn, physical.as_slice())
                .map_err(Error::from)?
            {
                Some(value) => {
                    out.extend_from_slice(value);
                    Ok(true)
                }
                None => Ok(false),
            };
        }
        let mut prefix = Vec::new();
        Self::bucket_prefix(key, &mut prefix);
        match self.find_bucket_slot(&rtxn, work, &prefix, key)? {
            Some(physical) => {
                let value = self
                    .database
                    .get(&rtxn, physical.as_slice())
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

    /// Key-ordered walk from the first logical key ≥ `start`. Exact for
    /// inline-sized keys (the log/seq use); oversized bucketed keys sort
    /// after every inline key, in fingerprint order — set semantics only.
    fn for_each_from(
        &mut self,
        work: &WorkContext,
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
        let range = self.database.range(&rtxn, &bounds).map_err(Error::from)?;
        for entry in range {
            work.step(1).map_err(work_error)?;
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

    fn write_batch(&mut self, work: &WorkContext, batch: &[(&[u8], &[u8])]) -> Result<()> {
        // The spill copy: entries come from the RAM tier (already
        // deduplicated), so plain puts are exact.
        for (key, value) in batch {
            work.step(1).map_err(work_error)?;
            self.put(work, key, value)?;
        }
        Ok(())
    }
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
