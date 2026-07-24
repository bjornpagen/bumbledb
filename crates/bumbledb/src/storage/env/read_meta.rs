use heed::types::Bytes;
use heed::{AnyTls, Database, RoTxn, WithoutTls};

use crate::error::{CorruptionError, Error, Result};
use crate::schema::Schema;
use crate::schema::fingerprint::{SchemaFingerprint, fingerprint};

/// The open-time meta-block classification
/// (`docs/architecture/50-storage.md` § open-time taxonomy, ruled
/// 2026-07-23, R18): before any meta check can run, every constructor
/// classifies the block itself through this ONE function — never the
/// same branch hand-written three ways.
pub(super) enum MetaBlock {
    /// `_meta` exists: an initialized store — the version/kind/roster/
    /// fingerprint checks proceed against this handle.
    Present(Database<Bytes, Bytes>),
    /// No `_meta` over an empty root: the half-created store (the crash
    /// window between environment creation and the meta commit) — a
    /// store never born, holding zero data. `Db::create` proceeds
    /// (creation heals it); the ephemeral open treats it as fresh;
    /// `Db::open` refuses it with the typed [`Error::NotInitialized`] —
    /// never `Corruption`.
    HalfCreated,
}

/// Classifies the `_meta` block. No `_meta` over a NON-empty root is the
/// foreign-environment refusal, [`Error::AlreadyInitialized`] — named
/// databases live as root entries, so this covers foreign named DBs too.
///
/// # Errors
///
/// `AlreadyInitialized` on a foreign LMDB environment; `Lmdb` otherwise.
pub(super) fn classify_meta_block(
    env: &heed::Env<WithoutTls>,
    rtxn: &RoTxn<'_, AnyTls>,
) -> Result<MetaBlock> {
    match env.open_database::<Bytes, Bytes>(rtxn, Some("_meta"))? {
        Some(meta) => Ok(MetaBlock::Present(meta)),
        None => {
            if let Some(root) = env.open_database::<Bytes, Bytes>(rtxn, None)?
                && !root.is_empty(rtxn)?
            {
                return Err(Error::AlreadyInitialized);
            }
            Ok(MetaBlock::HalfCreated)
        }
    }
}

// One decode discipline for every `_meta` value
// (`docs/architecture/50-storage.md` § the `_meta` block, ruled
// 2026-07-23, R18) — the split the store-kind reader pins: an absent key
// is `MetaMissing`; a present value that fails to decode is the
// malformed-value corruption naming the key (`what`), never
// `MetaMissing`. The two states point at opposite remedies.

pub(super) fn read_u64(
    meta: &Database<Bytes, Bytes>,
    rtxn: &RoTxn<'_, AnyTls>,
    key: &[u8],
    what: &'static str,
) -> Result<u64> {
    let bytes: [u8; 8] = meta
        .get(rtxn, key)?
        .ok_or(Error::Corruption(CorruptionError::MetaMissing))?
        .try_into()
        .map_err(|_| Error::Corruption(CorruptionError::MalformedValue(what)))?;
    Ok(u64::from_le_bytes(bytes))
}

pub(super) fn read_u32(
    meta: &Database<Bytes, Bytes>,
    rtxn: &RoTxn<'_, AnyTls>,
    key: &[u8],
    what: &'static str,
) -> Result<u32> {
    let bytes: [u8; 4] = meta
        .get(rtxn, key)?
        .ok_or(Error::Corruption(CorruptionError::MetaMissing))?
        .try_into()
        .map_err(|_| Error::Corruption(CorruptionError::MalformedValue(what)))?;
    Ok(u32::from_le_bytes(bytes))
}

/// The stored format version checked against [`super::FORMAT_VERSION`]
/// — ONE definition of the read and the refusal (formerly three
/// byte-identical blocks), first in the open-time check precedence
/// everywhere it runs (readers: `verify_and_open`, the ephemeral
/// constructor's non-mutating probe, and the exhume entry; the
/// precedence is pinned by the marker-matrix tests). Any other version
/// is the typed [`Error::FormatMismatch`] naming both; a missing key is
/// [`CorruptionError::MetaMissing`], a mis-sized value
/// [`CorruptionError::MalformedValue`], via [`read_u32`].
pub(super) fn check_format_version(
    meta: &Database<Bytes, Bytes>,
    rtxn: &RoTxn<'_, AnyTls>,
) -> Result<()> {
    let found = read_u32(meta, rtxn, super::META_FORMAT_VERSION, "format version")?;
    if found != super::FORMAT_VERSION {
        return Err(Error::FormatMismatch {
            found,
            expected: super::FORMAT_VERSION,
        });
    }
    Ok(())
}

/// The `_meta` store-kind marker, decoded with the absent/undecodable
/// distinction the taxonomy draws: a missing key is
/// [`CorruptionError::MetaMissing`]; a present key whose value is the
/// wrong width or an unknown byte is
/// [`CorruptionError::StoreKindInvalid`] — corrupt data, not a missing
/// key. Shared by the durable open path ([`super::Environment::open`]
/// via `verify_and_open`) and the ephemeral constructor's non-mutating
/// probe ([`super::Environment::ephemeral`]).
pub(super) fn read_store_kind(
    meta: &Database<Bytes, Bytes>,
    rtxn: &RoTxn<'_, AnyTls>,
) -> Result<super::StoreKind> {
    let bytes = meta
        .get(rtxn, super::META_STORE_KIND)?
        .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
    <[u8; 1]>::try_from(bytes)
        .ok()
        .and_then(|[byte]| super::StoreKind::from_meta_byte(byte))
        .ok_or(Error::Corruption(CorruptionError::StoreKindInvalid))
}

/// The stored schema fingerprint checked against the opening schema —
/// one definition of the decode and the mismatch (readers:
/// `verify_and_open`, and the ephemeral constructor's non-mutating
/// probe, which must raise the refusal BEFORE the ephemeral-flagged
/// reopen holds the file). A missing key is
/// [`CorruptionError::MetaMissing`], a mis-sized value
/// [`CorruptionError::MalformedValue`]; a present-but-different image is
/// the typed [`Error::SchemaMismatch`] naming both fingerprints.
pub(super) fn check_fingerprint(
    meta: &Database<Bytes, Bytes>,
    rtxn: &RoTxn<'_, AnyTls>,
    schema: &Schema,
) -> Result<()> {
    let stored = read_fingerprint(meta, rtxn)?;
    let expected = fingerprint(schema);
    if stored != expected.0 {
        return Err(Error::SchemaMismatch {
            found: SchemaFingerprint(stored),
            expected,
        });
    }
    Ok(())
}

/// The stored schema fingerprint, raw — the theory-less read
/// [`check_fingerprint`] compares through (readers: the exhume entry,
/// which holds no schema to compare against, and `Db::verify_store`'s
/// descriptor pass). A missing key is [`CorruptionError::MetaMissing`],
/// a mis-sized value [`CorruptionError::MalformedValue`].
pub(super) fn read_fingerprint(
    meta: &Database<Bytes, Bytes>,
    rtxn: &RoTxn<'_, AnyTls>,
) -> Result<[u8; 32]> {
    meta.get(rtxn, super::META_FINGERPRINT)?
        .ok_or(Error::Corruption(CorruptionError::MetaMissing))?
        .try_into()
        .map_err(|_| Error::Corruption(CorruptionError::MalformedValue("schema fingerprint")))
}

/// The dictionary next-id counter, sentinel-checked once for every
/// reader: a stored `u64::MAX` — the miss sentinel, never mintable — is
/// corrupt data, typed.
pub(super) fn read_dict_next_id(
    meta: &Database<Bytes, Bytes>,
    rtxn: &RoTxn<'_, AnyTls>,
) -> Result<u64> {
    let next = read_u64(meta, rtxn, super::META_DICT_NEXT_ID, "dict next id")?;
    if next == u64::MAX {
        return Err(Error::Corruption(CorruptionError::MalformedValue(
            "dict next id",
        )));
    }
    Ok(next)
}
