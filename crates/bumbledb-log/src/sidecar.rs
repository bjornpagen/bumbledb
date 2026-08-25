//! The chain sidecar: `dir/chain.json`, the one local file carrying
//! protocol state. It is a floor cache, never a truth the store must
//! reconcile against — the wholeness identity in `crate::replica` is the
//! only check it ever faces, and a torn sidecar costs a re-pull, not a
//! recovery procedure. Canonical one-line JSON, field order fixed,
//! written atomically (exclusive temp, fsync, rename, fsync parent).
//!
//! The chain is a sum — `Settled` or `Pending` — and `generation()` is
//! a total function of the value: the vector sum, plus one exactly when
//! the arm is `Pending`. `Pending` is writer-only; on a pure replica the
//! chain is permanently `Settled`. The read is a total sum too: `Absent`
//! is NotFound only, an infra fault is never absence, and a parse
//! refusal is `Corrupt`.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::path::Path;

use crate::braids::{BraidId, Braids};
use crate::manifest::{DOC_VERSION, Text, hex32};

/// The sidecar's file name inside a replica directory.
pub const CHAIN_FILE: &str = "chain.json";

/// One braid's chain position: the applied count, the blake3 of the
/// head log object (what the next batch's header must cite), and the
/// head timestamp (what the next batch's header must dominate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainEntry {
    pub g: u64,
    pub prev: [u8; 32],
    pub ts: u64,
}

impl ChainEntry {
    /// Braid genesis: count zero, the zero hash, timestamp zero.
    pub const GENESIS: Self = Self {
        g: 0,
        prev: [0u8; 32],
        ts: 0,
    };
}

/// The writer's one recovery slot: the batch it fsynced before applying
/// locally, so an interrupted commit resolves at open (60 owns the
/// resolution; this module only carries the bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    pub braid: BraidId,
    pub slot: u64,
    pub bytes: Vec<u8>,
}

/// Why sidecar bytes refused to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidecarError {
    Malformed {
        at: usize,
    },
    Version {
        got: u64,
    },
    /// A braid id the schema's own decomposition does not mint.
    UnknownBraid {
        got: u32,
    },
    /// The vector sum of the `g` column overflows `u64`.
    Overflow,
}

impl SidecarError {
    #[must_use]
    pub const fn identity(&self) -> &'static str {
        match self {
            Self::Malformed { .. } => "Malformed",
            Self::Version { .. } => "Version",
            Self::UnknownBraid { .. } => "UnknownBraid",
            Self::Overflow => "Overflow",
        }
    }
}

/// The sidecar read as a total sum. `Absent` is NotFound and nothing
/// else — an infra fault is never "no sidecar"; a parse refusal is
/// `Corrupt` and takes the disposable-law discard.
#[derive(Debug)]
pub enum SidecarRead {
    Absent,
    Fault(io::Error),
    Corrupt(SidecarError),
    Read(Chain),
}

impl SidecarRead {
    #[must_use]
    pub const fn identity(&self) -> &'static str {
        match self {
            Self::Absent => "Absent",
            Self::Fault(_) => "Fault",
            Self::Corrupt(_) => "Corrupt",
            Self::Read(_) => "Read",
        }
    }
}

/// The parsed sidecar: a settled vector, or that vector plus the one
/// pending batch. Braids absent from the map sit at genesis; the
/// canonical rendering materializes every braid explicitly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Chain {
    Settled {
        entries: BTreeMap<BraidId, ChainEntry>,
    },
    Pending {
        entries: BTreeMap<BraidId, ChainEntry>,
        batch: Pending,
    },
}

impl Chain {
    /// Every braid of the decomposition at genesis — the bootstrap
    /// chain: zero vector, zero-hash heads, zero timestamps.
    #[must_use]
    pub fn genesis(braids: &Braids) -> Self {
        Self::Settled {
            entries: braids
                .components()
                .keys()
                .map(|braid| (*braid, ChainEntry::GENESIS))
                .collect(),
        }
    }

    /// The per-braid positions. Absent keys are genesis.
    #[must_use]
    pub fn entries(&self) -> &BTreeMap<BraidId, ChainEntry> {
        match self {
            Self::Settled { entries } | Self::Pending { entries, .. } => entries,
        }
    }

    pub fn entries_mut(&mut self) -> &mut BTreeMap<BraidId, ChainEntry> {
        match self {
            Self::Settled { entries } | Self::Pending { entries, .. } => entries,
        }
    }

    /// The braid's position; absent entries are genesis.
    #[must_use]
    pub fn position(&self, braid: BraidId) -> ChainEntry {
        self.entries()
            .get(&braid)
            .copied()
            .unwrap_or(ChainEntry::GENESIS)
    }

    /// The vector sum — the settled side of `generation`.
    #[must_use]
    pub fn sum(&self) -> u64 {
        self.entries()
            .values()
            .fold(0u64, |acc, entry| acc.saturating_add(entry.g))
    }

    /// The generation a store must show: the vector sum, plus one
    /// exactly when the chain is `Pending`. There is no addend a
    /// reader can forget.
    #[must_use]
    pub fn generation(&self) -> u64 {
        match self {
            Self::Settled { .. } => self.sum(),
            Self::Pending { .. } => self.sum().saturating_add(1),
        }
    }

    /// The vector: applied counts keyed by braid.
    #[must_use]
    pub fn vector(&self) -> BTreeMap<BraidId, u64> {
        self.entries()
            .iter()
            .map(|(braid, entry)| (*braid, entry.g))
            .collect()
    }

    /// The canonical single-line rendering. Wire `pending` is `null` on
    /// `Settled` and the hex object on `Pending`; that is a rendering
    /// of the constructor, not a field beside it.
    #[must_use]
    pub fn render(&self) -> Vec<u8> {
        use std::fmt::Write as _;
        let mut chain = String::new();
        for (index, (braid, entry)) in self.entries().iter().enumerate() {
            if index > 0 {
                chain.push(',');
            }
            write!(
                chain,
                "\"{braid}\":{{\"g\":\"{}\",\"prev\":\"{}\",\"ts\":\"{}\"}}",
                entry.g,
                hex32(&entry.prev),
                entry.ts
            )
            .expect("write to string");
        }
        let pending = match self {
            Self::Settled { .. } => "null".to_string(),
            Self::Pending { batch, .. } => {
                let mut body = format!(
                    "{{\"braid\":\"{}\",\"gen\":\"{}\",\"bytes\":\"",
                    batch.braid, batch.slot
                );
                for byte in &batch.bytes {
                    write!(body, "{byte:02x}").expect("write to string");
                }
                body.push_str("\"}");
                body
            }
        };
        format!("{{\"v\":{DOC_VERSION},\"chain\":{{{chain}}},\"pending\":{pending}}}").into_bytes()
    }

    /// Strict parse to a canonical fixpoint, order-strict like
    /// `Checkpoint::parse`: braid ids must be ones the decomposition
    /// mints, entries must ascend in the render's canonical braid order
    /// (which leaves a duplicate no place to stand), and an accepted
    /// document re-renders byte-identically. Braids the file omits sit
    /// at genesis. `v:2` is a version refusal; every `g`/`ts`/`gen` is
    /// a quoted decimal u64; pending bytes are lowercase hex.
    pub fn parse(bytes: &[u8], braids: &Braids) -> Result<Self, SidecarError> {
        let mal = |at| SidecarError::Malformed { at };
        let mut text = Text::new(bytes);
        text.lit("{\"v\":").map_err(mal)?;
        let version = text.u64().map_err(mal)?;
        if version != DOC_VERSION {
            return Err(SidecarError::Version { got: version });
        }
        text.lit(",\"chain\":{").map_err(mal)?;
        let mut entries: BTreeMap<BraidId, ChainEntry> = BTreeMap::new();
        let mut first = true;
        while !text.peek("}") {
            if !first {
                text.lit(",").map_err(mal)?;
            }
            first = false;
            text.lit("\"c").map_err(mal)?;
            let raw = text.hex_u32().map_err(mal)?;
            let Some(braid) = braids.parse(raw) else {
                return Err(SidecarError::UnknownBraid { got: raw });
            };
            text.lit("\":{\"g\":").map_err(mal)?;
            let g = text.quoted_u64().map_err(mal)?;
            text.lit(",\"prev\":\"").map_err(mal)?;
            let prev = text.hex32().map_err(mal)?;
            text.lit("\",\"ts\":").map_err(mal)?;
            let ts = text.quoted_u64().map_err(mal)?;
            text.lit("}").map_err(mal)?;
            if entries
                .last_key_value()
                .is_some_and(|(last, _)| *last >= braid)
            {
                return Err(mal(text.at()));
            }
            entries.insert(braid, ChainEntry { g, prev, ts });
        }
        if entries
            .values()
            .try_fold(0u64, |acc, entry| acc.checked_add(entry.g))
            .is_none()
        {
            return Err(SidecarError::Overflow);
        }
        text.lit("},\"pending\":").map_err(mal)?;
        let chain = if text.peek("null") {
            text.lit("null").map_err(mal)?;
            Self::Settled { entries }
        } else {
            text.lit("{\"braid\":\"c").map_err(mal)?;
            let raw = text.hex_u32().map_err(mal)?;
            let Some(braid) = braids.parse(raw) else {
                return Err(SidecarError::UnknownBraid { got: raw });
            };
            text.lit("\",\"gen\":").map_err(mal)?;
            let slot = text.quoted_u64().map_err(mal)?;
            text.lit(",\"bytes\":\"").map_err(mal)?;
            let body = text.hex_bytes().map_err(mal)?;
            text.lit("\"}").map_err(mal)?;
            Self::Pending {
                entries,
                batch: Pending {
                    braid,
                    slot,
                    bytes: body,
                },
            }
        };
        text.lit("}").map_err(mal)?;
        text.end().map_err(mal)?;
        Ok(chain)
    }

    /// Atomic publication: exclusive temp beside the target, fsync,
    /// rename over `chain.json`, fsync the directory — so a crash leaves
    /// either the old sidecar or the new one, never a torn line.
    pub fn write_atomic(&self, dir: &Path) -> io::Result<()> {
        let target = dir.join(CHAIN_FILE);
        let temp = dir.join(format!(".{CHAIN_FILE}.tmp.{}", std::process::id()));
        let _ = fs::remove_file(&temp);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        let written = file
            .write_all(&self.render())
            .and_then(|()| file.sync_all());
        drop(file);
        if let Err(err) = written {
            let _ = fs::remove_file(&temp);
            return Err(err);
        }
        if let Err(err) = fs::rename(&temp, &target) {
            let _ = fs::remove_file(&temp);
            return Err(err);
        }
        File::open(dir)?.sync_all()
    }

    /// Reads `dir/chain.json` as a total sum. `Absent` is NotFound
    /// only; every other io error is `Fault`; a parse refusal is
    /// `Corrupt`.
    #[must_use]
    pub fn read(dir: &Path, braids: &Braids) -> SidecarRead {
        match fs::read(dir.join(CHAIN_FILE)) {
            Ok(bytes) => match Self::parse(&bytes, braids) {
                Ok(chain) => SidecarRead::Read(chain),
                Err(error) => SidecarRead::Corrupt(error),
            },
            Err(err) if err.kind() == io::ErrorKind::NotFound => SidecarRead::Absent,
            Err(err) => SidecarRead::Fault(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Chain, SidecarError, SidecarRead};
    use crate::braids::braids;
    use bumbledb::schema::{
        FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
        StatementDescriptor, ValueType,
    };

    fn kitchen_braids() -> crate::braids::Braids {
        let field = |name: &str, value_type: ValueType| FieldDescriptor {
            name: name.into(),
            value_type,
            generation: Generation::None,
        };
        let descriptor = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "sample".into(),
                fields: vec![field("id", ValueType::U64)],
                extension: None,
            }],
            statements: vec![StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::from([FieldId(0)]),
            }],
        };
        braids(&descriptor)
    }

    #[test]
    fn genesis_renders_quoted_u64s_and_v3() {
        let chain = Chain::genesis(&kitchen_braids());
        let text = String::from_utf8(chain.render()).expect("utf8");
        assert!(text.starts_with("{\"v\":3,"));
        assert!(text.contains("\"g\":\"0\""));
        assert!(text.contains("\"ts\":\"0\""));
        assert!(text.contains("\"pending\":null"));
        assert!(matches!(chain, Chain::Settled { .. }));
        assert_eq!(chain.generation(), 0);
    }

    #[test]
    fn parse_refuses_v2_and_a_json_number_g() {
        let braids = kitchen_braids();
        let v2 = br#"{"v":2,"chain":{"c00000000":{"g":"0","prev":"0000000000000000000000000000000000000000000000000000000000000000","ts":"0"}},"pending":null}"#;
        assert_eq!(
            Chain::parse(v2, &braids),
            Err(SidecarError::Version { got: 2 })
        );
        let numbered = br#"{"v":3,"chain":{"c00000000":{"g":40,"prev":"0000000000000000000000000000000000000000000000000000000000000000","ts":"0"}},"pending":null}"#;
        assert_eq!(
            Chain::parse(numbered, &braids)
                .expect_err("number g")
                .identity(),
            "Malformed"
        );
    }

    #[test]
    fn pending_bytes_are_lowercase_hex() {
        let braids = kitchen_braids();
        let hex = br#"{"v":3,"chain":{"c00000000":{"g":"0","prev":"0000000000000000000000000000000000000000000000000000000000000000","ts":"0"}},"pending":{"braid":"c00000000","gen":"1","bytes":"4244424c"}}"#;
        let parsed = Chain::parse(hex, &braids).expect("hex pending");
        let Chain::Pending { batch, .. } = parsed else {
            panic!("pending constructor");
        };
        assert_eq!(batch.bytes, [0x42, 0x44, 0x42, 0x4c]);
        assert_eq!(batch.slot, 1);
        let numbered_gen = br#"{"v":3,"chain":{"c00000000":{"g":"0","prev":"0000000000000000000000000000000000000000000000000000000000000000","ts":"0"}},"pending":{"braid":"c00000000","gen":1,"bytes":"4244424c"}}"#;
        assert_eq!(
            Chain::parse(numbered_gen, &braids)
                .expect_err("number gen")
                .identity(),
            "Malformed"
        );
        let b64 = br#"{"v":3,"chain":{"c00000000":{"g":"0","prev":"0000000000000000000000000000000000000000000000000000000000000000","ts":"0"}},"pending":{"braid":"c00000000","gen":"1","bytes":"QkRCTAM="}}"#;
        assert_eq!(
            Chain::parse(b64, &braids).expect_err("base64").identity(),
            "Malformed"
        );
    }

    #[test]
    fn generation_is_the_sum_plus_one_exactly_when_pending() {
        let braids = kitchen_braids();
        let settled = br#"{"v":3,"chain":{"c00000000":{"g":"40","prev":"0000000000000000000000000000000000000000000000000000000000000000","ts":"0"}},"pending":null}"#;
        let settled = Chain::parse(settled, &braids).expect("settled");
        assert!(matches!(settled, Chain::Settled { .. }));
        assert_eq!(settled.sum(), 40);
        assert_eq!(settled.generation(), 40);
        let pending = br#"{"v":3,"chain":{"c00000000":{"g":"40","prev":"0000000000000000000000000000000000000000000000000000000000000000","ts":"0"}},"pending":{"braid":"c00000000","gen":"41","bytes":"4244424c"}}"#;
        let pending = Chain::parse(pending, &braids).expect("pending");
        assert!(matches!(pending, Chain::Pending { .. }));
        assert_eq!(pending.sum(), 40);
        assert_eq!(pending.generation(), 41);
    }

    #[test]
    fn read_is_absent_corrupt_or_read() {
        let braids = kitchen_braids();
        let dir = std::env::temp_dir().join(format!(
            "bumbledb-sidecar-read-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("scratch");
        assert_eq!(Chain::read(&dir, &braids).identity(), "Absent");
        let v2 = br#"{"v":2,"chain":{"c00000000":{"g":"0","prev":"0000000000000000000000000000000000000000000000000000000000000000","ts":"0"}},"pending":null}"#;
        std::fs::write(dir.join(super::CHAIN_FILE), v2).expect("write v2");
        let SidecarRead::Corrupt(SidecarError::Version { got: 2 }) = Chain::read(&dir, &braids)
        else {
            panic!("v2 is Corrupt, not Absent");
        };
        let chain = Chain::genesis(&braids);
        chain.write_atomic(&dir).expect("write settled");
        match Chain::read(&dir, &braids) {
            SidecarRead::Read(got) => {
                assert_eq!(got, chain);
                assert_eq!(got.generation(), 0);
            }
            other => panic!("expected Read, got {}", other.identity()),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
