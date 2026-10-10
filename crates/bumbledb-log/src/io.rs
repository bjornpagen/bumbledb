//! Object-store requests the machine emits as data, the responses it
//! consumes, and the keys it names.

use std::path::PathBuf;

use bumbledb::SchemaFingerprint;

use crate::ids::{ImageDigest, Millis, Seq};

/// Correlates a response with its request; unique per machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IoId(pub u64);

/// `Log` is the commit bucket (S3 Express: no ordered LIST); `Checkpoints`
/// holds images (S3 Standard: lexicographic LIST).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bucket {
    Log,
    Checkpoints,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoRequest {
    pub id: IoId,
    pub bucket: Bucket,
    pub key: String,
    pub op: Op,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Get(Target),
    /// Create the object only if the key is absent (`If-None-Match: *`).
    PutIfAbsent(Body),
    /// Keys under `key` as a prefix, in lexicographic order.
    List {
        start_after: Option<String>,
        max_keys: u32,
    },
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Memory,
    File(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    Bytes(Vec<u8>),
    File(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoResponse {
    pub id: IoId,
    /// The store's `Date` header, the only clock the machine trusts.
    pub date: Option<Millis>,
    pub result: IoResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IoResult {
    /// `Get(Target::Memory)` found the object.
    Body {
        bytes: Vec<u8>,
        last_modified: Millis,
    },
    /// `Get(Target::File)` wrote the object to the file.
    Saved {
        last_modified: Millis,
    },
    /// `Get` found no object.
    Missing,
    /// `PutIfAbsent` created the object.
    Created,
    /// `PutIfAbsent` found the key taken (412) or, for a log object, being
    /// written by another conditional write (409); after a 409 the slot stays
    /// empty if that write fails.
    Occupied,
    Keys(Vec<String>),
    Deleted,
    /// Anything else, after the executor's own retries of idempotent reads:
    /// 5xx, timeouts, transport errors, a 409 outside the log. A failed write
    /// may have landed.
    Failed,
}

#[must_use]
pub fn log_key(seq: Seq) -> String {
    format!("log/{:020}", seq.get())
}

pub const CHECKPOINT_PREFIX: &str = "ckpt/";

/// `ckpt/{u64::MAX - seq}-{schema}-{digest}.bdb`: a lexicographic LIST
/// returns the newest checkpoint first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckpointKey {
    pub seq: Seq,
    pub schema: SchemaFingerprint,
    pub digest: ImageDigest,
}

impl CheckpointKey {
    #[must_use]
    pub fn format(&self) -> String {
        format!(
            "{CHECKPOINT_PREFIX}{:020}-{}-{}.bdb",
            u64::MAX - self.seq.get(),
            hex(&self.schema.0),
            hex(&self.digest.0)
        )
    }

    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        let rest = key.strip_prefix(CHECKPOINT_PREFIX)?.strip_suffix(".bdb")?;
        let mut parts = rest.split('-');
        let inverted = parts.next()?;
        let schema = unhex(parts.next()?)?;
        let digest = unhex(parts.next()?)?;
        if parts.next().is_some() || inverted.len() != 20 {
            return None;
        }
        let seq = Seq::new(u64::MAX - inverted.parse::<u64>().ok()?)?;
        let parsed = Self {
            seq,
            schema: SchemaFingerprint(schema),
            digest: ImageDigest(digest),
        };
        (parsed.format() == key).then_some(parsed)
    }
}

/// Where a migration's image lives, named by its content.
#[must_use]
pub fn image_key(digest: ImageDigest) -> String {
    format!("mig/{}.bdb", hex(&digest.0))
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

fn unhex(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 {
        return None;
    }
    let mut out = [0; 32];
    let (pairs, _) = text.as_bytes().as_chunks::<2>();
    for (slot, pair) in out.iter_mut().zip(pairs) {
        *slot = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(out)
}
