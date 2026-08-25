//! The manifest — the protocol's one mutable object — and the
//! checkpoint document it points to. Both are binary records of the
//! batch codec's grammar: a leading version byte, fixed field rosters,
//! and length-delimited vectors over `u64le` / `u32le` / `[u8; 32]`.
//! The manifest is a pointer to the head of an immutable Merkle list:
//! every checkpoint fact lives in `ckpt/{digest}` beside the store
//! bytes that share that digest, the digest is the blake3 of the
//! document's bytes including `prev`, and the mutable CAS surface of
//! the whole protocol is the manifest object's checkpoint digest. This
//! module also owns the object key layout, so every consumer spells a
//! key exactly one way.

use std::collections::BTreeMap;

use crate::braids::{BraidId, Braids};
use crate::store::{
    Create, ObjectStore, Result as StoreResult, StoreKey, Swap, prove_create, prove_swap,
};

/// The one accepted document version; it is the leading byte of every
/// manifest and checkpoint record. The binary format is v:3.
pub const DOC_VERSION: u8 = 3;

const ABSENT: u8 = 0;
const PRESENT: u8 = 1;

/// One checkpoint head on the wire: `braid u32le`, `g u64le`,
/// `hash [u8; 32]`, `ts u64le`.
const HEAD_BYTES: usize = 4 + 8 + 32 + 8;

fn key(prefix: &str, rest: &str) -> StoreKey {
    let raw = if prefix.is_empty() {
        rest.to_string()
    } else {
        format!("{prefix}/{rest}")
    };
    StoreKey::of(&raw)
}

#[must_use]
pub fn manifest_key(prefix: &str) -> StoreKey {
    key(prefix, "manifest")
}

#[must_use]
pub fn log_key(prefix: &str, braid: BraidId, slot: u64) -> StoreKey {
    key(prefix, &format!("log/{braid}/{slot:016x}"))
}

#[must_use]
pub fn ckpt_mdb_key(prefix: &str, digest: &[u8; 32]) -> StoreKey {
    key(prefix, &format!("ckpt/{}.mdb", hex32(digest)))
}

#[must_use]
pub fn ckpt_json_key(prefix: &str, digest: &[u8; 32]) -> StoreKey {
    key(prefix, &format!("ckpt/{}", hex32(digest)))
}

/// 64 lowercase hex characters for a 32-byte digest — the object-key
/// spelling of a content address.
#[must_use]
pub fn hex32(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        write!(out, "{byte:02x}").expect("write to string");
        out
    })
}

/// A sequential walker over document bytes: every method refuses at the
/// first short read, carrying the offset. Counts are bounded by the
/// bytes behind them.
pub(crate) struct Cursor<'b> {
    bytes: &'b [u8],
    at: usize,
}

impl<'b> Cursor<'b> {
    pub(crate) const fn new(bytes: &'b [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    pub(crate) const fn at(&self) -> usize {
        self.at
    }

    pub(crate) fn take(&mut self, len: usize) -> Result<&'b [u8], usize> {
        let end = self
            .at
            .checked_add(len)
            .filter(|end| *end <= self.bytes.len())
            .ok_or(self.at)?;
        let slice = &self.bytes[self.at..end];
        self.at = end;
        Ok(slice)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, usize> {
        Ok(self.take(1)?[0])
    }

    pub(crate) fn u32(&mut self) -> Result<u32, usize> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub(crate) fn u64(&mut self) -> Result<u64, usize> {
        let bytes = self.take(8)?;
        let mut raw = [0u8; 8];
        raw.copy_from_slice(bytes);
        Ok(u64::from_le_bytes(raw))
    }

    pub(crate) fn array32(&mut self) -> Result<[u8; 32], usize> {
        let bytes = self.take(32)?;
        let mut raw = [0u8; 32];
        raw.copy_from_slice(bytes);
        Ok(raw)
    }

    pub(crate) fn optional_digest(&mut self) -> Result<Option<[u8; 32]>, usize> {
        let tag_at = self.at;
        match self.u8()? {
            ABSENT => Ok(None),
            PRESENT => Ok(Some(self.array32()?)),
            _ => Err(tag_at),
        }
    }

    pub(crate) const fn remaining(&self) -> usize {
        self.bytes.len() - self.at
    }

    pub(crate) fn end(&self) -> Result<(), usize> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(self.at)
        }
    }
}

fn bytes_back(count: u32, remaining: usize, min_item: usize) -> bool {
    let declared = usize::try_from(count).expect("u32 fits usize");
    if declared == 0 {
        return true;
    }
    if min_item == 0 {
        return false;
    }
    remaining / min_item >= declared
}

fn put_optional_digest(out: &mut Vec<u8>, digest: Option<&[u8; 32]>) {
    match digest {
        None => out.push(ABSENT),
        Some(digest) => {
            out.push(PRESENT);
            out.extend_from_slice(digest);
        }
    }
}

/// Why manifest bytes refused to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestError {
    /// The bytes deviate from the roster at this offset.
    Malformed { at: usize },
    /// A well-formed document of a version this consumer refuses.
    Version { got: u64 },
}

impl ManifestError {
    #[must_use]
    pub const fn identity(&self) -> &'static str {
        match self {
            Self::Malformed { .. } => "Malformed",
            Self::Version { .. } => "Version",
        }
    }
}

/// The parsed manifest: version byte, fingerprint, optional checkpoint
/// digest. `checkpoint` is a real null arm from store birth until the
/// first checkpoint lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Manifest {
    pub fingerprint: [u8; 32],
    pub checkpoint: Option<[u8; 32]>,
}

impl Manifest {
    /// The one encoding, byte-exact for CAS bodies.
    #[must_use]
    pub fn render(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(1 + 32 + 1 + 32);
        out.push(DOC_VERSION);
        out.extend_from_slice(&self.fingerprint);
        put_optional_digest(&mut out, self.checkpoint.as_ref());
        out
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, ManifestError> {
        let mal = |at| ManifestError::Malformed { at };
        let mut cur = Cursor::new(bytes);
        let version = cur.u8().map_err(mal)?;
        if version != DOC_VERSION {
            return Err(ManifestError::Version {
                got: u64::from(version),
            });
        }
        let fingerprint = cur.array32().map_err(mal)?;
        let checkpoint = cur.optional_digest().map_err(mal)?;
        cur.end().map_err(mal)?;
        Ok(Self {
            fingerprint,
            checkpoint,
        })
    }
}

/// One braid's head as a checkpoint records it: the applied count, the
/// blake3 of the head log object, and its timestamp — the seeds for the
/// `prev`-chain and monotone-ts checks across a checkpoint jump.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Head {
    pub g: u64,
    pub hash: [u8; 32],
    pub ts: u64,
}

/// Why checkpoint-document bytes refused to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointError {
    Malformed {
        at: usize,
    },
    /// A well-formed document of a version this consumer refuses.
    Version {
        got: u64,
    },
    /// The vector sum of the `g` column overflows `u64`.
    Overflow,
    /// A braid id the schema's own decomposition does not mint.
    UnknownBraid {
        got: u32,
    },
    /// The document's braid set is not exactly the derived set — the
    /// schema/checkpoint drift refusal (both are pure functions of the
    /// same schema, so any disagreement is drift, not variety).
    BraidSet,
}

impl CheckpointError {
    #[must_use]
    pub const fn identity(&self) -> &'static str {
        match self {
            Self::Malformed { .. } => "Malformed",
            Self::Version { .. } => "Version",
            Self::Overflow => "Overflow",
            Self::UnknownBraid { .. } => "UnknownBraid",
            Self::BraidSet => "BraidSet",
        }
    }
}

/// The parsed `ckpt/{digest}`: one map, one fact per braid, the catalog
/// content claim, the publisher, and the backlink. The vector is the
/// `g` column; its sum is derived, never stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkpoint {
    pub braids: BTreeMap<BraidId, Head>,
    pub catalog: [u8; 32],
    pub writer: u64,
    pub prev: Option<[u8; 32]>,
}

impl Checkpoint {
    #[must_use]
    pub fn render(&self) -> Vec<u8> {
        let count = u32::try_from(self.braids.len()).expect("braid count fits u32");
        let mut out = Vec::with_capacity(1 + 4 + self.braids.len() * HEAD_BYTES + 32 + 8 + 1 + 32);
        out.push(DOC_VERSION);
        out.extend_from_slice(&count.to_le_bytes());
        for (braid, head) in &self.braids {
            out.extend_from_slice(&braid.raw().to_le_bytes());
            out.extend_from_slice(&head.g.to_le_bytes());
            out.extend_from_slice(&head.hash);
            out.extend_from_slice(&head.ts.to_le_bytes());
        }
        out.extend_from_slice(&self.catalog);
        out.extend_from_slice(&self.writer.to_le_bytes());
        put_optional_digest(&mut out, self.prev.as_ref());
        out
    }

    /// Strict parse against the derived braid set: unknown ids refuse,
    /// entries ascend in braid-id order (which leaves a duplicate no
    /// place to stand), and the set must match the decomposition
    /// exactly. A declared count the remaining bytes cannot back is
    /// Malformed.
    pub fn parse(bytes: &[u8], braids: &Braids) -> Result<Self, CheckpointError> {
        let mal = |at| CheckpointError::Malformed { at };
        let mut cur = Cursor::new(bytes);
        let version = cur.u8().map_err(mal)?;
        if version != DOC_VERSION {
            return Err(CheckpointError::Version {
                got: u64::from(version),
            });
        }
        let count = cur.u32().map_err(mal)?;
        if !bytes_back(count, cur.remaining(), HEAD_BYTES) {
            return Err(mal(cur.at()));
        }
        let mut map: BTreeMap<BraidId, Head> = BTreeMap::new();
        for _ in 0..count {
            let raw = cur.u32().map_err(mal)?;
            let Some(braid) = braids.parse(raw) else {
                return Err(CheckpointError::UnknownBraid { got: raw });
            };
            let g = cur.u64().map_err(mal)?;
            let hash = cur.array32().map_err(mal)?;
            let ts = cur.u64().map_err(mal)?;
            if map.last_key_value().is_some_and(|(last, _)| *last >= braid) {
                return Err(mal(cur.at()));
            }
            map.insert(braid, Head { g, hash, ts });
        }
        if map
            .values()
            .try_fold(0u64, |acc, head| acc.checked_add(head.g))
            .is_none()
        {
            return Err(CheckpointError::Overflow);
        }
        let catalog = cur.array32().map_err(mal)?;
        let writer = cur.u64().map_err(mal)?;
        let prev = cur.optional_digest().map_err(mal)?;
        cur.end().map_err(mal)?;
        let derived: Vec<BraidId> = braids.components().keys().copied().collect();
        let carried: Vec<BraidId> = map.keys().copied().collect();
        if derived != carried {
            return Err(CheckpointError::BraidSet);
        }
        Ok(Self {
            braids: map,
            catalog,
            writer,
            prev,
        })
    }

    /// blake3 of the rendered bytes. `prev` is inside the hash, so two
    /// documents that differ only in the backlink are different objects
    /// at different keys.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        *blake3::hash(&self.render()).as_bytes()
    }

    /// The vector sum — the checkpoint order's total order.
    #[must_use]
    pub fn sum(&self) -> u64 {
        self.braids
            .values()
            .fold(0u64, |acc, head| acc.saturating_add(head.g))
    }

    /// The vector: the `g` column keyed by braid.
    #[must_use]
    pub fn vector(&self) -> BTreeMap<BraidId, u64> {
        self.braids
            .iter()
            .map(|(braid, head)| (*braid, head.g))
            .collect()
    }
}

/// Creates the manifest with `put_create` — the store-birth arm; a
/// second creator sees `Exists` and proceeds on the incumbent.
pub fn create_manifest<S: ObjectStore>(
    store: &S,
    prefix: &str,
    manifest: &Manifest,
) -> StoreResult<Create> {
    store.put_create(&manifest_key(prefix), &manifest.render())
}

/// Outcome of a checkpoint publication attempt under the checkpoint
/// order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Published {
    /// The candidate replaced the incumbent pointer.
    Replaced,
    /// The incumbent's vector sum is at least the candidate's. The
    /// candidate is a known-orphan object: addressable by its digest
    /// and collected as the complement of the reachable spine.
    Kept {
        incumbent: [u8; 32],
    },
    Refused(PublishRefusal),
}

/// Typed refusals on the publication path — the pointer or its target
/// failed to parse, which no retry mends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishRefusal {
    ManifestMissing,
    Manifest(ManifestError),
    CheckpointDocMissing { digest: [u8; 32] },
    Checkpoint(CheckpointError),
}

/// Publishes `candidate` under the checkpoint order: the candidate
/// replaces the incumbent iff its vector sum is strictly greater; a
/// `Moved` CAS re-reads and re-applies the order. The document is
/// named by the blake3 of its bytes, `prev` included, and is written
/// once with `put_create`; `Exists` is byte-identity. The manifest
/// points at the head of that immutable list. Termination is
/// structural — every successful swap strictly raises the incumbent
/// sum.
pub fn publish_checkpoint<S: ObjectStore>(
    store: &S,
    prefix: &str,
    braids: &Braids,
    candidate: &Checkpoint,
) -> StoreResult<Published> {
    let bytes = candidate.render();
    let digest = *blake3::hash(&bytes).as_bytes();
    let key = ckpt_json_key(prefix, &digest);
    loop {
        match prove_create(store, &key, &bytes, store.put_create(&key, &bytes)?)? {
            Create::Created(_) | Create::Exists => break,
            Create::Ambiguous => {}
        }
    }
    loop {
        let Some(fetched) = store.get(&manifest_key(prefix))? else {
            return Ok(Published::Refused(PublishRefusal::ManifestMissing));
        };
        let manifest = match Manifest::parse(&fetched.bytes) {
            Ok(manifest) => manifest,
            Err(error) => {
                return Ok(Published::Refused(PublishRefusal::Manifest(error)));
            }
        };
        if let Some(incumbent) = manifest.checkpoint {
            if incumbent == digest {
                return Ok(Published::Replaced);
            }
            let Some(doc) = store.get(&ckpt_json_key(prefix, &incumbent))? else {
                return Ok(Published::Refused(PublishRefusal::CheckpointDocMissing {
                    digest: incumbent,
                }));
            };
            let incumbent_doc = match Checkpoint::parse(&doc.bytes, braids) {
                Ok(doc) => doc,
                Err(error) => {
                    return Ok(Published::Refused(PublishRefusal::Checkpoint(error)));
                }
            };
            if candidate.sum() <= incumbent_doc.sum() {
                return Ok(Published::Kept { incumbent });
            }
        }
        let next = Manifest {
            fingerprint: manifest.fingerprint,
            checkpoint: Some(digest),
        };
        let body = next.render();
        match prove_swap(
            store,
            &manifest_key(prefix),
            &body,
            store.put_swap(&manifest_key(prefix), &body, &fetched.etag)?,
        )? {
            Swap::Swapped(_) => return Ok(Published::Replaced),
            Swap::Moved | Swap::Ambiguous => {}
        }
    }
}
