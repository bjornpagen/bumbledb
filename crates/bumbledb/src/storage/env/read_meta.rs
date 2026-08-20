use heed::types::Bytes;
use heed::{AnyTls, Database, RoTxn, RwTxn, WithoutTls};

use crate::encoding::InternId;
use crate::error::{CorruptionError, Error, Mismatch, Result};
use crate::schema::Schema;
use crate::schema::fingerprint::{SchemaFingerprint, fingerprint};

use super::{FORMAT_VERSION, GenerationId};

/// One `_meta` key: the persisted byte, the diagnostic name, and the
/// codec that `parse_meta` / `write_fresh_meta` share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MetaKey {
    pub key: &'static [u8],
    pub name: &'static str,
}

impl MetaKey {
    pub const FORMAT_VERSION: Self = Self {
        key: &[0],
        name: "format version",
    };
    pub const FINGERPRINT: Self = Self {
        key: &[1],
        name: "schema fingerprint",
    };
    pub const GENERATION: Self = Self {
        key: &[2],
        name: "tx id",
    };
    pub const DICT_NEXT: Self = Self {
        key: &[3],
        name: "dict next id",
    };

    /// Parse order: version, fingerprint, generation, dict-next.
    /// The database roster is not a `_meta` key.
    pub const PARSE_ORDER: [Self; 4] = [
        Self::FORMAT_VERSION,
        Self::FINGERPRINT,
        Self::GENERATION,
        Self::DICT_NEXT,
    ];
}

const _: () = assert!(MetaKey::PARSE_ORDER.len() == 4);

/// Format 8 is the only constructible version. Any other stored word is
/// [`Error::FormatMismatch`] at the parse boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FormatVersion(u32);

impl FormatVersion {
    pub const V8: Self = Self(FORMAT_VERSION);

    fn parse(word: u32) -> Result<Self> {
        if word == FORMAT_VERSION {
            Ok(Self::V8)
        } else {
            Err(Error::FormatMismatch {
                mismatch: Mismatch {
                    witnessed: word,
                    required: FORMAT_VERSION,
                },
            })
        }
    }

    pub const fn word(self) -> u32 {
        self.0
    }
}

/// The four-key `_meta` block as one value. Constructed only by
/// [`parse_meta`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoreMeta {
    pub version: FormatVersion,
    pub fingerprint: SchemaFingerprint,
    pub generation: GenerationId,
    pub dict_next: InternId,
}

impl StoreMeta {
    pub(crate) fn matches_schema(&self, schema: &Schema) -> Result<()> {
        let required = fingerprint(schema);
        if self.fingerprint != required {
            return Err(Error::SchemaMismatch {
                mismatch: Mismatch {
                    witnessed: self.fingerprint,
                    required,
                },
            });
        }
        Ok(())
    }
}

/// The open-time meta-block classification
/// (`docs/architecture/50-storage.md` § open-time taxonomy, ruled
/// 2026-07-23, R18): before any meta check can run, every constructor
/// classifies the block itself through this ONE function — never the
/// same branch hand-written three ways.
pub(super) enum MetaBlock {
    /// `_meta` exists: an initialized store — the version/roster/
    /// fingerprint checks proceed against this handle.
    Present(Database<Bytes, Bytes>),
    /// No `_meta` over an empty root: the half-created store (the crash
    /// window between environment creation and the meta commit) — a
    /// store never born, holding zero data. `Db::open` refuses it with
    /// [`Error::AlreadyInitialized`] — never `Corruption`. Constructors
    /// never claim a pre-existing destination path: that is
    /// [`Error::DestinationExists`].
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
    if let Some(meta) = env.open_database::<Bytes, Bytes>(rtxn, Some("_meta"))? {
        return Ok(MetaBlock::Present(meta));
    }
    if let Some(root) = env.open_database::<Bytes, Bytes>(rtxn, None)?
        && !root.is_empty(rtxn)?
    {
        return Err(Error::AlreadyInitialized);
    }
    Ok(MetaBlock::HalfCreated)
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

/// The stored schema fingerprint, raw (readers: [`parse_meta`] and
/// `Db::verify_store`). A missing key is
/// [`CorruptionError::MetaMissing`], a mis-sized value
/// [`CorruptionError::MalformedValue`].
pub(super) fn read_fingerprint(
    meta: &Database<Bytes, Bytes>,
    rtxn: &RoTxn<'_, AnyTls>,
) -> Result<[u8; 32]> {
    meta.get(rtxn, MetaKey::FINGERPRINT.key)?
        .ok_or(Error::Corruption(CorruptionError::MetaMissing))?
        .try_into()
        .map_err(|_| Error::Corruption(CorruptionError::MalformedValue(MetaKey::FINGERPRINT.name)))
}

/// The dictionary next-id counter, sentinel-checked once for every
/// reader: a stored `u64::MAX` — the miss sentinel, never mintable — is
/// corrupt data, typed.
pub(super) fn read_dict_next_id(
    meta: &Database<Bytes, Bytes>,
    rtxn: &RoTxn<'_, AnyTls>,
) -> Result<u64> {
    let next = read_u64(meta, rtxn, MetaKey::DICT_NEXT.key, MetaKey::DICT_NEXT.name)?;
    if next == crate::encoding::InternId::SENTINEL.raw() {
        return Err(Error::Corruption(CorruptionError::MalformedValue(
            MetaKey::DICT_NEXT.name,
        )));
    }
    Ok(next)
}

/// Version — the open-precedence prefix that must run before
/// the database roster.
pub(super) fn parse_meta_head(
    meta: &Database<Bytes, Bytes>,
    rtxn: &RoTxn<'_, AnyTls>,
) -> Result<FormatVersion> {
    FormatVersion::parse(read_u32(
        meta,
        rtxn,
        MetaKey::FORMAT_VERSION.key,
        MetaKey::FORMAT_VERSION.name,
    )?)
}

/// Parses the four-key `_meta` block: version, fingerprint, generation,
/// dict-next.
pub(crate) fn parse_meta(
    meta: &Database<Bytes, Bytes>,
    rtxn: &RoTxn<'_, AnyTls>,
) -> Result<StoreMeta> {
    let version = FormatVersion::parse(read_u32(
        meta,
        rtxn,
        MetaKey::FORMAT_VERSION.key,
        MetaKey::FORMAT_VERSION.name,
    )?)?;
    let fingerprint = SchemaFingerprint(read_fingerprint(meta, rtxn)?);
    let generation = GenerationId::from_storage(read_u64(
        meta,
        rtxn,
        MetaKey::GENERATION.key,
        MetaKey::GENERATION.name,
    )?);
    let dict_next = InternId::from_raw(read_dict_next_id(meta, rtxn)?);
    Ok(StoreMeta {
        version,
        fingerprint,
        generation,
        dict_next,
    })
}

/// Synthesizes a fresh four-key `_meta` block. Source metadata is never
/// copied; callers pass the values that belong at the destination.
pub(super) fn write_fresh_meta(
    meta: &Database<Bytes, Bytes>,
    wtxn: &mut RwTxn<'_>,
    schema: &Schema,
    generation: GenerationId,
    dict_next: InternId,
) -> Result<()> {
    meta.put(
        wtxn,
        MetaKey::FORMAT_VERSION.key,
        FormatVersion::V8.word().to_le_bytes().as_slice(),
    )?;
    meta.put(
        wtxn,
        MetaKey::FINGERPRINT.key,
        fingerprint(schema).0.as_slice(),
    )?;
    meta.put(
        wtxn,
        MetaKey::GENERATION.key,
        generation.storage_word().to_le_bytes().as_slice(),
    )?;
    meta.put(
        wtxn,
        MetaKey::DICT_NEXT.key,
        dict_next.raw().to_le_bytes().as_slice(),
    )?;
    Ok(())
}
