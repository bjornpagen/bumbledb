//! Successor store identity and `_core_meta` framing.
//!
//! The family magic plus layout counter make old bytes unambiguously
//! incompatible: recognizing the integer `1` alone is forbidden (C12), so
//! every check reads the eight family bytes first. The transitional format-8
//! store uses different database names (`_meta`/`_data`/`_dict`) and no
//! family key; either direction of cross-open refuses before any write.
//!
//! Physical bytes remain provisional until the F3 format probes; changing
//! any layout here requires bumping [`LAYOUT`] so provisional files refuse.

use heed::RoTxn;
use heed::types::Bytes;

use super::error::{StoreCorruption, StoreError, StoreResult};
use crate::schema::fingerprint::SchemaFingerprint;

/// Successor core family magic. Not shared with the log/command/snapshot
/// families, which own their separate magics.
pub const FAMILY: &[u8; 8] = b"BDBCOR1\0";

/// Layout counter within the family. Restarted at 1 by explicit decision;
/// the family magic is what makes old files unambiguous. Layout 2 (F3
/// finding B): the `0x03` determinant namespace is populated and
/// load-bearing — every sealed key statement's scalar determinant entries
/// are maintained with each row and consumed by keyed reads, key probes and
/// judgment enumeration. A layout-1 directory (empty determinant namespace)
/// refuses instead of silently missing on every keyed read.
pub const LAYOUT: u32 = 2;

/// Named databases inside the environment. Deliberately distinct from the
/// transitional store's `_meta`/`_data`/`_dict` so neither format can adopt
/// the other's bytes.
pub const META_DB: &str = "_core_meta";
pub const DATA_DB: &str = "_core_data";

/// `_core_meta` key tags. Single-byte keys except where noted.
pub const K_FAMILY: &[u8] = &[0x00];
pub const K_LAYOUT: &[u8] = &[0x01];
pub const K_STORE_ID: &[u8] = &[0x02];
pub const K_SCHEMA: &[u8] = &[0x03];
pub const K_GENERATION: &[u8] = &[0x04];
pub const K_NEXT_ROW_ID: &[u8] = &[0x05];
/// Per-relation live row count: `[0x06, relation u32 BE]`.
pub const K_ROW_COUNT_TAG: u8 = 0x06;
/// Per-relation change version: `[0x07, relation u32 BE]` → u64 BE. Absent
/// means [`RelationVersion::initial`] — a relation no committed transaction
/// has ever changed. Advanced exactly when a committed transaction changed
/// that relation's rows; host-record/attachment-only seals advance the
/// generation but never a relation version.
pub const K_RELATION_VERSION_TAG: u8 = 0x07;
/// Opaque host records: `[0x10, caller key bytes]`.
pub const K_HOST_RECORD_TAG: u8 = 0x10;
/// Opaque host attachment: `[0x11]`.
pub const K_ATTACHMENT: &[u8] = &[0x11];

/// Persistent store identity, minted once at `create` and never rewritten.
/// Copies of the same store share it; environment identity distinguishes
/// live opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoreStoreId(pub [u8; 16]);

impl CoreStoreId {
    pub(crate) fn mint(path: &std::path::Path) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NONCE: AtomicU64 = AtomicU64::new(1);
        let mut digest = crate::digest::Digest::new();
        digest.update(b"bumbledb/1/core-store-id");
        digest.update(path.as_os_str().as_encoded_bytes());
        digest.update(&std::process::id().to_be_bytes());
        digest.update(&NONCE.fetch_add(1, Ordering::Relaxed).to_be_bytes());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |age| age.as_nanos());
        digest.update(&now.to_be_bytes());
        let bytes = digest.finalize();
        let mut id = [0u8; 16];
        id.copy_from_slice(&bytes[..16]);
        Self(id)
    }
}

impl std::fmt::Display for CoreStoreId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Per-open environment identity. Protects borrowed plans/caches from a
/// different native environment even when it opened a copy of the same
/// store. Process-local; never persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnvironmentId(std::num::NonZeroU64);

impl EnvironmentId {
    pub(crate) fn mint() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let raw = NEXT.fetch_add(1, Ordering::Relaxed);
        Self(std::num::NonZeroU64::new(raw).expect("environment ids start at 1"))
    }

    #[must_use]
    pub fn value(self) -> u64 {
        self.0.get()
    }
}

/// The pair every snapshot and commit names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreIdentity {
    pub store: CoreStoreId,
    pub environment: EnvironmentId,
}

/// Local physical row identity: a storage surrogate, never an application
/// scalar and never part of logical export identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RowId(pub u64);

pub(crate) fn row_count_key(relation: bumbledb_theory::schema::RelationId) -> [u8; 5] {
    let mut key = [K_ROW_COUNT_TAG, 0, 0, 0, 0];
    key[1..5].copy_from_slice(&relation.0.to_be_bytes());
    key
}

/// One relation's change version: advanced exactly when a committed
/// transaction changed that relation's rows, never by metadata-only
/// (host-record/attachment) seals. The image-reuse key: two snapshots of one
/// environment that witness equal versions for a relation witnessed the same
/// rows for it, so a memo keyed by (relation, version) reuses only proved
/// unchanged content. Monotonic within a store; bounded above by the
/// generation (every version advance rides a generation advance).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct RelationVersion(u64);

impl RelationVersion {
    /// The version of a relation no committed transaction ever changed.
    #[must_use]
    pub const fn initial() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }

    pub(crate) const fn from_storage(word: u64) -> Self {
        Self(word)
    }

    pub(crate) const fn storage_word(self) -> u64 {
        self.0
    }

    pub(crate) fn next(self) -> StoreResult<Self> {
        // Unreachable in practice: a version advance always rides a
        // generation advance, which refuses overflow first.
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(StoreError::GenerationExhausted)
    }
}

impl std::fmt::Display for RelationVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub(crate) fn relation_version_key(relation: bumbledb_theory::schema::RelationId) -> [u8; 5] {
    let mut key = [K_RELATION_VERSION_TAG, 0, 0, 0, 0];
    key[1..5].copy_from_slice(&relation.0.to_be_bytes());
    key
}

/// Read one relation's committed change version from one transaction's
/// view; an absent entry is [`RelationVersion::initial`].
pub(crate) fn read_relation_version(
    meta: &heed::Database<Bytes, Bytes>,
    txn: &RoTxn<'_, heed::AnyTls>,
    relation: bumbledb_theory::schema::RelationId,
) -> StoreResult<RelationVersion> {
    let Some(bytes) = meta
        .get(txn, relation_version_key(relation).as_slice())
        .map_err(StoreError::from_heed)?
    else {
        return Ok(RelationVersion::initial());
    };
    let word = u64::from_be_bytes(bytes.try_into().map_err(|_| {
        StoreError::Corruption(StoreCorruption::MalformedKey("relation version value width"))
    })?);
    Ok(RelationVersion::from_storage(word))
}

pub(crate) fn read_u64(
    meta: &heed::Database<Bytes, Bytes>,
    txn: &RoTxn<'_, heed::AnyTls>,
    key: &[u8],
    what: &'static str,
) -> StoreResult<u64> {
    let bytes = meta
        .get(txn, key)
        .map_err(StoreError::from_heed)?
        .ok_or(StoreError::Corruption(StoreCorruption::MetaMissing(what)))?;
    Ok(u64::from_be_bytes(bytes.try_into().map_err(|_| {
        StoreError::Corruption(StoreCorruption::MetaMissing(what))
    })?))
}

pub(crate) fn read_store_id(
    meta: &heed::Database<Bytes, Bytes>,
    txn: &RoTxn<'_, heed::AnyTls>,
) -> StoreResult<CoreStoreId> {
    let bytes = meta
        .get(txn, K_STORE_ID)
        .map_err(StoreError::from_heed)?
        .ok_or(StoreError::Corruption(StoreCorruption::MetaMissing(
            "store id",
        )))?;
    Ok(CoreStoreId(bytes.try_into().map_err(|_| {
        StoreError::Corruption(StoreCorruption::MetaMissing("store id"))
    })?))
}

/// Family/layout/schema verification against one read view, before any
/// write, cleanup or adoption. An unrecognized directory refuses with
/// [`StoreError::UnrecognizedStore`]; a recognized family with a different
/// layout refuses with the exact counters; a recognized store with a foreign
/// schema refuses with [`StoreError::SchemaMismatch`].
pub(crate) fn verify_meta(
    meta: &heed::Database<Bytes, Bytes>,
    txn: &RoTxn<'_, heed::AnyTls>,
    path: &std::path::Path,
    schema_fp: &SchemaFingerprint,
) -> StoreResult<CoreStoreId> {
    let family = meta.get(txn, K_FAMILY).map_err(StoreError::from_heed)?;
    match family {
        Some(bytes) if bytes == FAMILY => {}
        _ => {
            return Err(StoreError::UnrecognizedStore {
                path: path.to_path_buf(),
            });
        }
    }
    let layout = meta
        .get(txn, K_LAYOUT)
        .map_err(StoreError::from_heed)?
        .ok_or(StoreError::Corruption(StoreCorruption::MetaMissing(
            "layout",
        )))?;
    let layout = u32::from_be_bytes(
        layout
            .try_into()
            .map_err(|_| StoreError::Corruption(StoreCorruption::MetaMissing("layout")))?,
    );
    if layout != LAYOUT {
        return Err(StoreError::LayoutMismatch {
            found: layout,
            expected: LAYOUT,
        });
    }
    let stored_schema = meta
        .get(txn, K_SCHEMA)
        .map_err(StoreError::from_heed)?
        .ok_or(StoreError::Corruption(StoreCorruption::MetaMissing(
            "schema fingerprint",
        )))?;
    if stored_schema != schema_fp.0 {
        return Err(StoreError::SchemaMismatch);
    }
    read_store_id(meta, txn)
}
