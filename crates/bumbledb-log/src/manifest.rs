//! The manifest — the protocol's one mutable object — and the
//! checkpoint document it points to. Both are canonical single-line
//! UTF-8 JSON with a fixed field order, so parse and render are exact
//! template walks (hand-rolled on purpose: a general JSON reader would
//! accept re-orderings and whitespace the canon forbids). The manifest
//! is a pure pointer: every checkpoint fact lives in
//! `ckpt/{digest}.json` beside the store bytes whose digest names both,
//! the document's `prev` is proven by the manifest CAS that installs
//! it, and the mutable CAS surface of the whole protocol is one 64-hex
//! field. This module also owns the object key layout, so every
//! consumer spells a key exactly one way.

use std::collections::BTreeMap;

use crate::braids::{BraidId, Braids};
use crate::store::{Create, ObjectStore, Result as StoreResult, Swap};

/// The one accepted manifest and sidecar document version.
pub const DOC_VERSION: u64 = 2;

fn key(prefix: &str, rest: &str) -> String {
    if prefix.is_empty() {
        rest.to_string()
    } else {
        format!("{prefix}/{rest}")
    }
}

#[must_use]
pub fn manifest_key(prefix: &str) -> String {
    key(prefix, "manifest.json")
}

#[must_use]
pub fn log_key(prefix: &str, braid: BraidId, slot: u64) -> String {
    key(prefix, &format!("log/{braid}/{slot:016x}"))
}

#[must_use]
pub fn ckpt_mdb_key(prefix: &str, digest: &[u8; 32]) -> String {
    key(prefix, &format!("ckpt/{}.mdb", hex32(digest)))
}

#[must_use]
pub fn ckpt_json_key(prefix: &str, digest: &[u8; 32]) -> String {
    key(prefix, &format!("ckpt/{}.json", hex32(digest)))
}

/// 64 lowercase hex characters for a 32-byte digest — the one rendering
/// every document in the protocol uses.
#[must_use]
pub fn hex32(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        write!(out, "{byte:02x}").expect("write to string");
        out
    })
}

/// A strict template walker over canonical document bytes: every method
/// refuses at the first byte that deviates, carrying the offset. No
/// whitespace skipping exists — the canon has no whitespace.
pub(crate) struct Text<'b> {
    bytes: &'b [u8],
    at: usize,
}

impl<'b> Text<'b> {
    pub(crate) const fn new(bytes: &'b [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    pub(crate) const fn at(&self) -> usize {
        self.at
    }

    pub(crate) fn lit(&mut self, expected: &str) -> Result<(), usize> {
        let want = expected.as_bytes();
        let end = self.at.checked_add(want.len()).ok_or(self.at)?;
        if self.bytes.get(self.at..end) == Some(want) {
            self.at = end;
            Ok(())
        } else {
            Err(self.at)
        }
    }

    pub(crate) fn peek(&self, expected: &str) -> bool {
        let want = expected.as_bytes();
        self.at
            .checked_add(want.len())
            .is_some_and(|end| self.bytes.get(self.at..end) == Some(want))
    }

    fn hex_nibble(&mut self) -> Result<u8, usize> {
        let byte = *self.bytes.get(self.at).ok_or(self.at)?;
        let value = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => return Err(self.at),
        };
        self.at += 1;
        Ok(value)
    }

    pub(crate) fn hex32(&mut self) -> Result<[u8; 32], usize> {
        let mut out = [0u8; 32];
        for slot in &mut out {
            *slot = self.hex_nibble()? << 4 | self.hex_nibble()?;
        }
        Ok(out)
    }

    pub(crate) fn hex_u32(&mut self) -> Result<u32, usize> {
        let mut out: u32 = 0;
        for _ in 0..8 {
            out = out << 4 | u32::from(self.hex_nibble()?);
        }
        Ok(out)
    }

    /// A canonical JSON u64: decimal digits, no leading zero unless the
    /// value is exactly zero.
    pub(crate) fn u64(&mut self) -> Result<u64, usize> {
        let start = self.at;
        let mut value: u64 = 0;
        while let Some(byte @ b'0'..=b'9') = self.bytes.get(self.at) {
            value = value
                .checked_mul(10)
                .and_then(|v| v.checked_add(u64::from(byte - b'0')))
                .ok_or(self.at)?;
            self.at += 1;
        }
        let len = self.at - start;
        if len == 0 || (len > 1 && self.bytes[start] == b'0') {
            return Err(start);
        }
        Ok(value)
    }

    /// Lowercase hex payload of even length up to the closing quote; the
    /// pending slot's batch bytes ride here.
    pub(crate) fn hex_bytes(&mut self) -> Result<Vec<u8>, usize> {
        let mut out = Vec::new();
        while !self.peek("\"") {
            out.push(self.hex_nibble()? << 4 | self.hex_nibble()?);
        }
        Ok(out)
    }

    pub(crate) fn end(&self) -> Result<(), usize> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(self.at)
        }
    }
}

/// Why manifest bytes refused to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestError {
    /// The bytes deviate from the canonical template at this offset.
    Malformed { at: usize },
    /// A well-formed document of a version this consumer refuses.
    Version { got: u64 },
}

/// The parsed manifest: `{"v":2,"fingerprint":…,"checkpoint":…}`.
/// `checkpoint` is a real null arm from store birth until the first
/// checkpoint lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Manifest {
    pub fingerprint: [u8; 32],
    pub checkpoint: Option<[u8; 32]>,
}

impl Manifest {
    /// The canonical single-line rendering, byte-exact for CAS bodies.
    #[must_use]
    pub fn render(&self) -> Vec<u8> {
        let checkpoint = match &self.checkpoint {
            Some(digest) => format!("\"{}\"", hex32(digest)),
            None => "null".to_string(),
        };
        format!(
            "{{\"v\":{DOC_VERSION},\"fingerprint\":\"{}\",\"checkpoint\":{checkpoint}}}",
            hex32(&self.fingerprint)
        )
        .into_bytes()
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, ManifestError> {
        let mal = |at| ManifestError::Malformed { at };
        let mut text = Text::new(bytes);
        text.lit("{\"v\":").map_err(mal)?;
        let version = text.u64().map_err(mal)?;
        if version != DOC_VERSION {
            return Err(ManifestError::Version { got: version });
        }
        text.lit(",\"fingerprint\":\"").map_err(mal)?;
        let fingerprint = text.hex32().map_err(mal)?;
        text.lit("\",\"checkpoint\":").map_err(mal)?;
        let checkpoint = if text.peek("null") {
            text.lit("null").map_err(mal)?;
            None
        } else {
            text.lit("\"").map_err(mal)?;
            let digest = text.hex32().map_err(mal)?;
            text.lit("\"").map_err(mal)?;
            Some(digest)
        };
        text.lit("}").map_err(mal)?;
        text.end().map_err(mal)?;
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
    /// A braid id the schema's own decomposition does not mint.
    UnknownBraid {
        got: u32,
    },
    /// The document's braid set is not exactly the derived set — the
    /// schema/checkpoint drift refusal (both are pure functions of the
    /// same schema, so any disagreement is drift, not variety).
    BraidSet,
}

/// The parsed `ckpt/{digest}.json`: one map, one fact per braid, the
/// catalog content claim, the publisher, and the backlink. The vector is
/// the `g` column; its sum is derived, never stored.
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
        use std::fmt::Write as _;
        let mut braids = String::new();
        for (index, (braid, head)) in self.braids.iter().enumerate() {
            if index > 0 {
                braids.push(',');
            }
            write!(
                braids,
                "\"{braid}\":{{\"g\":{},\"hash\":\"{}\",\"ts\":{}}}",
                head.g,
                hex32(&head.hash),
                head.ts
            )
            .expect("write to string");
        }
        let prev = match &self.prev {
            Some(digest) => format!("\"{}\"", hex32(digest)),
            None => "null".to_string(),
        };
        format!(
            "{{\"braids\":{{{braids}}},\"catalog\":\"{}\",\"writer\":{},\"prev\":{prev}}}",
            hex32(&self.catalog),
            self.writer
        )
        .into_bytes()
    }

    /// Strict parse against the derived braid set: unknown ids refuse,
    /// entries must ascend in the render's canonical braid order (which
    /// leaves a duplicate no place to stand), and the set must match
    /// the decomposition exactly.
    pub fn parse(bytes: &[u8], braids: &Braids) -> Result<Self, CheckpointError> {
        let mal = |at| CheckpointError::Malformed { at };
        let mut text = Text::new(bytes);
        let mut map: BTreeMap<BraidId, Head> = BTreeMap::new();
        text.lit("{\"braids\":{").map_err(mal)?;
        let mut first = true;
        while !text.peek("}") {
            if !first {
                text.lit(",").map_err(mal)?;
            }
            first = false;
            text.lit("\"c").map_err(mal)?;
            let raw = text.hex_u32().map_err(mal)?;
            let Some(braid) = braids.parse(raw) else {
                return Err(CheckpointError::UnknownBraid { got: raw });
            };
            text.lit("\":{\"g\":").map_err(mal)?;
            let g = text.u64().map_err(mal)?;
            text.lit(",\"hash\":\"").map_err(mal)?;
            let hash = text.hex32().map_err(mal)?;
            text.lit("\",\"ts\":").map_err(mal)?;
            let ts = text.u64().map_err(mal)?;
            text.lit("}").map_err(mal)?;
            if map.last_key_value().is_some_and(|(last, _)| *last >= braid) {
                return Err(mal(text.at()));
            }
            map.insert(braid, Head { g, hash, ts });
        }
        text.lit("},\"catalog\":\"").map_err(mal)?;
        let catalog = text.hex32().map_err(mal)?;
        text.lit("\",\"writer\":").map_err(mal)?;
        let writer = text.u64().map_err(mal)?;
        text.lit(",\"prev\":").map_err(mal)?;
        let prev = if text.peek("null") {
            text.lit("null").map_err(mal)?;
            None
        } else {
            text.lit("\"").map_err(mal)?;
            let digest = text.hex32().map_err(mal)?;
            text.lit("\"").map_err(mal)?;
            Some(digest)
        };
        text.lit("}").map_err(mal)?;
        text.end().map_err(mal)?;
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

    /// The vector sum — the checkpoint order's total order.
    #[must_use]
    pub fn sum(&self) -> u64 {
        self.braids.values().map(|head| head.g).sum()
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
    /// The incumbent's vector sum is at least the candidate's; the
    /// candidate's objects are gc fodder.
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
/// `Moved` CAS re-reads and re-applies the order. The candidate's
/// document is rendered and uploaded inside the loop, so its `prev`
/// always names the incumbent the winning CAS actually replaced —
/// proven by the CAS itself, never hoped — and every retained
/// checkpoint stays reachable from the manifest by the backlink walk.
/// Termination is structural — every successful swap strictly raises
/// the incumbent sum.
pub fn publish_checkpoint<S: ObjectStore>(
    store: &S,
    prefix: &str,
    braids: &Braids,
    candidate: [u8; 32],
    heads: &BTreeMap<BraidId, Head>,
    catalog: [u8; 32],
    writer: u64,
) -> StoreResult<Published> {
    let candidate_sum: u64 = heads.values().map(|head| head.g).sum();
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
            if incumbent == candidate {
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
            if candidate_sum <= incumbent_doc.sum() {
                return Ok(Published::Kept { incumbent });
            }
        }
        let doc = Checkpoint {
            braids: heads.clone(),
            catalog,
            writer,
            prev: manifest.checkpoint,
        };
        upsert(store, &ckpt_json_key(prefix, &candidate), &doc.render())?;
        let next = Manifest {
            fingerprint: manifest.fingerprint,
            checkpoint: Some(candidate),
        };
        match store.put_swap(&manifest_key(prefix), &next.render(), &fetched.etag)? {
            Swap::Swapped(_) => return Ok(Published::Replaced),
            Swap::Moved => {}
        }
    }
}

/// Writes `bytes` at `key` whatever stands there: create when absent,
/// byte-equal stands, anything else swaps under the read etag. The one
/// consumer is the checkpoint document whose `prev` a CAS race
/// re-renders.
fn upsert<S: ObjectStore>(store: &S, key: &str, bytes: &[u8]) -> StoreResult<()> {
    loop {
        match store.put_create(key, bytes)? {
            Create::Created(_) => return Ok(()),
            Create::Exists => {
                let Some(existing) = store.get(key)? else {
                    continue;
                };
                if existing.bytes == bytes {
                    return Ok(());
                }
                match store.put_swap(key, bytes, &existing.etag)? {
                    Swap::Swapped(_) => return Ok(()),
                    Swap::Moved => {}
                }
            }
        }
    }
}
